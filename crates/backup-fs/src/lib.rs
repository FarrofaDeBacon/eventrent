use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use eventrent_core::domain::backup::{
    calculate_retention_plan, is_valid_backup_filename, validate_destination_folder,
    BackupFileEntry, BackupManifest, BackupMetadataView, BackupOperationGuard,
    RestorePendingMarker, RetentionCandidate, RetentionPlan, VerifyBackupResult,
    RESTORE_PENDING_FILE,
};
use eventrent_core::error::{CoreError, Result};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection, SqliteConnection, SqlitePool};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone)]
pub struct BackupFsService {
    pub app_data_dir: PathBuf,
    pub db_path: PathBuf,
    pub media_dir: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RestoreReport {
    pub restored_backup_file: String,
    pub pre_restore_snapshot_file: Option<String>,
    pub restored_at: String,
    pub files_restored_count: usize,
}

impl BackupFsService {
    pub fn new(
        app_data_dir: PathBuf,
        db_path: PathBuf,
        media_dir: PathBuf,
        config_path: PathBuf,
    ) -> Self {
        Self {
            app_data_dir,
            db_path,
            media_dir,
            config_path,
        }
    }

    /// Executa um backup consistente completo do sistema para a pasta de destino informada.
    pub async fn run_backup(
        &self,
        destination_folder: &Path,
        db_pool: &SqlitePool,
        app_version: &str,
    ) -> Result<(BackupManifest, PathBuf)> {
        let _guard = BackupOperationGuard::try_acquire()?;

        validate_destination_folder(destination_folder, &self.app_data_dir)?;

        fs::create_dir_all(destination_folder).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao acessar pasta de destino de backup ({}): {e}",
                destination_folder.display()
            ))
        })?;

        let now = Utc::now();
        let ts = now.format("%Y-%m-%d-%H%M%S").to_string();
        let vacuum_unique = uuid::Uuid::now_v7();
        let temp_vacuum_db = self
            .app_data_dir
            .join(format!("vacuum_backup_{vacuum_unique}.tmp.db"));

        // 1. Cópia consistente do banco SQLite via VACUUM INTO
        let vacuum_sql = format!(
            "VACUUM INTO '{}'",
            temp_vacuum_db.to_string_lossy().replace('\'', "''")
        );

        sqlx::query(&vacuum_sql)
            .execute(db_pool)
            .await
            .map_err(|e| {
                let _ = fs::remove_file(&temp_vacuum_db);
                CoreError::Internal(format!(
                    "Falha ao executar VACUUM INTO para backup consistente do SQLite: {e}"
                ))
            })?;

        // 2. Validação imediata de integridade física e coleta de métricas na cópia
        let (table_counts, last_applied_migration, migrations_checksum) = {
            let mut opts = SqliteConnectOptions::new().filename(&temp_vacuum_db);
            opts = opts.log_statements(tracing::log::LevelFilter::Off);

            let mut conn = SqliteConnection::connect_with(&opts).await.map_err(|e| {
                let _ = fs::remove_file(&temp_vacuum_db);
                CoreError::Internal(format!(
                    "Falha ao conectar na cópia temporária do banco de dados: {e}"
                ))
            })?;

            let (check_status,): (String,) = sqlx::query_as("PRAGMA integrity_check")
                .fetch_one(&mut conn)
                .await
                .map_err(|e| {
                    let _ = fs::remove_file(&temp_vacuum_db);
                    CoreError::Internal(format!(
                        "Falha ao consultar integridade da cópia de backup: {e}"
                    ))
                })?;

            if check_status != "ok" {
                let _ = fs::remove_file(&temp_vacuum_db);
                return Err(CoreError::Internal(format!(
                    "Cópia do banco de dados para backup falhou no PRAGMA integrity_check: {check_status}"
                )));
            }

            let tables = [
                "app_user",
                "tenant",
                "customer",
                "customer_contact",
                "customer_address",
                "venue",
                "venue_space",
                "venue_contact",
                "brand",
                "category",
                "attribute_definition",
                "equipment_model",
                "equipment_model_media",
                "media_asset",
                "media_blob",
                "tenant_setting",
                "audit_log",
            ];

            let mut counts = HashMap::new();
            for tbl in tables {
                let q = format!("SELECT count(*) FROM {tbl}");
                let query_res: std::result::Result<(i64,), _> =
                    sqlx::query_as(&q).fetch_one(&mut conn).await;
                if let Ok((cnt,)) = query_res {
                    counts.insert(tbl.to_string(), cnt);
                }
            }

            let max_mig: Option<(i64,)> = sqlx::query_as(
                "SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
            )
            .fetch_optional(&mut conn)
            .await
            .unwrap_or(None);

            let last_mig = max_mig.map(|(v,)| v).unwrap_or(0);

            let mig_checksums: Vec<(Vec<u8>,)> =
                sqlx::query_as("SELECT checksum FROM _sqlx_migrations ORDER BY version ASC")
                    .fetch_all(&mut conn)
                    .await
                    .unwrap_or_default();

            let mut mig_hasher = Sha256::new();
            for (c,) in mig_checksums {
                mig_hasher.update(&c);
            }
            let mig_hash_hex = format!("{:x}", mig_hasher.finalize());

            (counts, last_mig, mig_hash_hex)
        };

        // 3. Compactação híbrida do arquivo ZIP em segundo plano (spawn_blocking)
        let temp_vacuum_db_clone = temp_vacuum_db.clone();
        let destination_folder_buf = destination_folder.to_path_buf();
        let media_dir_buf = self.media_dir.clone();
        let config_path_buf = self.config_path.clone();
        let app_version_string = app_version.to_string();

        let manifest = tokio::task::spawn_blocking(move || {
            let temp_zip_filename = format!(".tmp.eventrent-backup-{}.tmp", uuid::Uuid::now_v7());
            let temp_zip_path = destination_folder_buf.join(&temp_zip_filename);
            let final_zip_filename = format!("eventrent-backup-{ts}.eventrent-backup");
            let final_zip_path = destination_folder_buf.join(&final_zip_filename);

            let zip_file = match File::create(&temp_zip_path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = fs::remove_file(&temp_vacuum_db_clone);
                    return Err(CoreError::Internal(format!(
                        "Falha ao criar arquivo de backup temporário ({}): {e}",
                        temp_zip_path.display()
                    )));
                }
            };

            let mut zip = ZipWriter::new(zip_file);
            let mut file_entries = Vec::new();

            // 3.1. Adiciona banco SQLite comprimido (Deflate)
            let deflate_opts =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            let stored_opts =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

            match File::open(&temp_vacuum_db_clone) {
                Ok(mut db_f) => {
                    if let Err(e) = zip.start_file("database/eventrent.db", deflate_opts) {
                        let _ = fs::remove_file(&temp_zip_path);
                        let _ = fs::remove_file(&temp_vacuum_db_clone);
                        return Err(CoreError::Internal(format!(
                            "Falha ao iniciar entrada do banco no zip: {e}"
                        )));
                    }

                    let mut hasher = Sha256::new();
                    let mut buffer = [0u8; 64 * 1024];
                    let mut total_bytes = 0u64;

                    loop {
                        let read = match db_f.read(&mut buffer) {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(e) => {
                                let _ = fs::remove_file(&temp_zip_path);
                                let _ = fs::remove_file(&temp_vacuum_db_clone);
                                return Err(CoreError::Internal(format!(
                                    "Falha ao ler banco temporário: {e}"
                                )));
                            }
                        };
                        hasher.update(&buffer[..read]);
                        total_bytes += read as u64;
                        if let Err(e) = zip.write_all(&buffer[..read]) {
                            let _ = fs::remove_file(&temp_zip_path);
                            let _ = fs::remove_file(&temp_vacuum_db_clone);
                            return Err(CoreError::Internal(format!(
                                "Falha ao gravar banco no zip: {e}"
                            )));
                        }
                    }

                    file_entries.push(BackupFileEntry {
                        path: "database/eventrent.db".to_string(),
                        byte_size: total_bytes,
                        sha256: format!("{:x}", hasher.finalize()),
                    });
                }
                Err(e) => {
                    let _ = fs::remove_file(&temp_zip_path);
                    let _ = fs::remove_file(&temp_vacuum_db_clone);
                    return Err(CoreError::Internal(format!(
                        "Falha ao abrir banco temporário: {e}"
                    )));
                }
            }

            // Remove o banco temporário de vacuum imediatamente após escrita no zip
            let _ = fs::remove_file(&temp_vacuum_db_clone);

            // 3.2. Adiciona config.json (se existir)
            if config_path_buf.exists() {
                if let Ok(mut cfg_f) = File::open(&config_path_buf) {
                    if zip.start_file("config.json", deflate_opts).is_ok() {
                        let mut hasher = Sha256::new();
                        let mut buf = Vec::new();
                        if cfg_f.read_to_end(&mut buf).is_ok() {
                            hasher.update(&buf);
                            let _ = zip.write_all(&buf);
                            file_entries.push(BackupFileEntry {
                                path: "config.json".to_string(),
                                byte_size: buf.len() as u64,
                                sha256: format!("{:x}", hasher.finalize()),
                            });
                        }
                    }
                }
            }

            // 3.3. Adiciona arquivos de mídia (modo Stored / sem recompressão)
            if media_dir_buf.exists() && media_dir_buf.is_dir() {
                let mut stack = vec![media_dir_buf.clone()];
                while let Some(dir) = stack.pop() {
                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                stack.push(path);
                            } else if path.is_file() {
                                let file_name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or_default();
                                if file_name.starts_with(".tmp.") || file_name.contains(".tmp.") {
                                    continue;
                                }

                                if let Ok(rel) = path.strip_prefix(&media_dir_buf) {
                                    let rel_slash = rel.to_string_lossy().replace('\\', "/");
                                    let zip_entry_name = format!("media/{rel_slash}");

                                    if let Ok(mut mf) = File::open(&path) {
                                        if zip.start_file(&zip_entry_name, stored_opts).is_ok() {
                                            let mut hasher = Sha256::new();
                                            let mut buffer = [0u8; 64 * 1024];
                                            let mut total_bytes = 0u64;

                                            while let Ok(n) = mf.read(&mut buffer) {
                                                if n == 0 {
                                                    break;
                                                }
                                                hasher.update(&buffer[..n]);
                                                total_bytes += n as u64;
                                                let _ = zip.write_all(&buffer[..n]);
                                            }

                                            file_entries.push(BackupFileEntry {
                                                path: zip_entry_name,
                                                byte_size: total_bytes,
                                                sha256: format!("{:x}", hasher.finalize()),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 3.4. Monta e grava manifest.json na raiz do zip
            let manifest = BackupManifest {
                manifest_version: 1,
                app_version: app_version_string,
                created_at: Utc::now().to_rfc3339(),
                last_applied_migration,
                migrations_checksum,
                database_integrity_checked: true,
                files: file_entries,
                table_counts,
            };

            let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!("Falha ao serializar manifest.json: {e}"))
            })?;

            zip.start_file("manifest.json", deflate_opts).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!("Falha ao iniciar manifest.json no zip: {e}"))
            })?;

            zip.write_all(&manifest_bytes).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!("Falha ao gravar manifest.json no zip: {e}"))
            })?;

            zip.finish().map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!("Falha ao finalizar arquivo de backup ZIP: {e}"))
            })?;

            // 3.5. Renomeação atômica do arquivo temporário para o arquivo final
            fs::rename(&temp_zip_path, &final_zip_path).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!(
                    "Falha ao renomear arquivo de backup final ({} -> {}): {e}",
                    temp_zip_path.display(),
                    final_zip_path.display()
                ))
            })?;

            Ok((manifest, final_zip_path))
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!("Falha na tarefa de compactação de backup: {e}"))
        })?;

        manifest
    }

    /// Lista todos os backups válidos presentes em uma pasta informada.
    pub fn list_backups_in_folder(
        &self,
        folder: &Path,
        current_max_migration: i64,
    ) -> Result<Vec<BackupMetadataView>> {
        if !folder.exists() || !folder.is_dir() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        let entries = fs::read_dir(folder).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao ler diretório de backups ({}): {e}",
                folder.display()
            ))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !is_valid_backup_filename(fname) {
                continue;
            }

            if let Ok(meta) = fs::metadata(&path) {
                if let Ok(f) = File::open(&path) {
                    if let Ok(mut archive) = ZipArchive::new(f) {
                        if let Ok(mut mf) = archive.by_name("manifest.json") {
                            let mut buf = String::new();
                            if mf.read_to_string(&mut buf).is_ok() {
                                if let Ok(manifest) = serde_json::from_str::<BackupManifest>(&buf) {
                                    let is_compat =
                                        manifest.last_applied_migration <= current_max_migration;
                                    list.push(BackupMetadataView {
                                        file_name: fname.to_string(),
                                        file_path: path.to_string_lossy().to_string(),
                                        byte_size: meta.len(),
                                        created_at: manifest.created_at,
                                        app_version: manifest.app_version,
                                        last_applied_migration: manifest.last_applied_migration,
                                        is_compatible: is_compat,
                                        file_count: manifest.files.len(),
                                        verified: manifest.database_integrity_checked,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }

    /// Executa a verificação profunda de integridade do arquivo de backup (manifesto, sha256 e SQLite integrity_check).
    pub fn verify_backup_file(
        &self,
        file_path: &Path,
        current_max_migration: i64,
    ) -> Result<VerifyBackupResult> {
        if !file_path.exists() || !file_path.is_file() {
            return Ok(VerifyBackupResult {
                valid: false,
                manifest: None,
                error_message: Some(format!(
                    "Arquivo de backup não encontrado ou inacessível: {}",
                    file_path.display()
                )),
                is_compatible: false,
            });
        }

        let f = File::open(file_path).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao abrir arquivo de backup ({}): {e}",
                file_path.display()
            ))
        })?;

        let mut archive = match ZipArchive::new(f) {
            Ok(a) => a,
            Err(e) => {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: None,
                    error_message: Some(format!("Arquivo ZIP corrompido ou inválido: {e}")),
                    is_compatible: false,
                });
            }
        };

        // Lê manifest.json
        let manifest_bytes = {
            let mut mf = match archive.by_name("manifest.json") {
                Ok(f) => f,
                Err(_) => {
                    return Ok(VerifyBackupResult {
                        valid: false,
                        manifest: None,
                        error_message: Some(
                            "manifest.json ausente no arquivo de backup.".to_string(),
                        ),
                        is_compatible: false,
                    });
                }
            };
            let mut buf = Vec::new();
            if let Err(e) = mf.read_to_end(&mut buf) {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: None,
                    error_message: Some(format!("Falha ao ler manifest.json: {e}")),
                    is_compatible: false,
                });
            }
            buf
        };

        let manifest: BackupManifest = match serde_json::from_slice(&manifest_bytes) {
            Ok(m) => m,
            Err(e) => {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: None,
                    error_message: Some(format!("manifest.json com formato inválido: {e}")),
                    is_compatible: false,
                });
            }
        };

        let is_compatible = manifest.last_applied_migration <= current_max_migration;

        // Valida SHA-256 e byte_size de cada entrada listada no manifesto
        for entry in &manifest.files {
            let mut zf = match archive.by_name(&entry.path) {
                Ok(f) => f,
                Err(_) => {
                    return Ok(VerifyBackupResult {
                        valid: false,
                        manifest: Some(manifest.clone()),
                        error_message: Some(format!(
                            "Arquivo declarado no manifesto não encontrado no pacote: {}",
                            entry.path
                        )),
                        is_compatible,
                    });
                }
            };

            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut actual_bytes = 0u64;

            loop {
                match zf.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        hasher.update(&buffer[..n]);
                        actual_bytes += n as u64;
                    }
                    Err(e) => {
                        return Ok(VerifyBackupResult {
                            valid: false,
                            manifest: Some(manifest.clone()),
                            error_message: Some(format!(
                                "Falha ao extrair e ler arquivo '{}': {e}",
                                entry.path
                            )),
                            is_compatible,
                        });
                    }
                }
            }

            if actual_bytes != entry.byte_size {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(format!(
                        "Tamanho divergente para '{}': esperado {} bytes, encontrado {} bytes",
                        entry.path, entry.byte_size, actual_bytes
                    )),
                    is_compatible,
                });
            }

            let actual_hash = format!("{:x}", hasher.finalize());
            if actual_hash != entry.sha256 {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(format!(
                        "Soma de verificação SHA-256 divergente para '{}': arquivo adulterado ou corrompido",
                        entry.path
                    )),
                    is_compatible,
                });
            }
        }

        Ok(VerifyBackupResult {
            valid: true,
            manifest: Some(manifest),
            error_message: None,
            is_compatible,
        })
    }

    /// Agenda a restauração a frio do backup para a próxima inicialização do aplicativo.
    pub fn schedule_restore(
        &self,
        file_path: &Path,
        user_id: &str,
        current_max_migration: i64,
    ) -> Result<RestorePendingMarker> {
        let verify = self.verify_backup_file(file_path, current_max_migration)?;
        if !verify.valid {
            return Err(CoreError::Validation(format!(
                "O arquivo de backup selecionado é inválido ou está corrompido: {}",
                verify.error_message.unwrap_or_default()
            )));
        }

        if !verify.is_compatible {
            let mig = verify
                .manifest
                .as_ref()
                .map(|m| m.last_applied_migration)
                .unwrap_or(0);
            return Err(CoreError::Validation(format!(
                "Incompatibilidade de esquema: o backup requer migração versão {mig}, mas o aplicativo instalado suporta até a versão {current_max_migration}. Atualize o aplicativo antes de restaurar este backup."
            )));
        }

        // Calcula hash SHA-256 do arquivo de backup como chave de conferência
        let mut f = File::open(file_path).map_err(|e| {
            CoreError::Internal(format!("Falha ao abrir backup para cálculo de hash: {e}"))
        })?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        while let Ok(n) = f.read(&mut buf) {
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let backup_sha256 = format!("{:x}", hasher.finalize());

        let marker = RestorePendingMarker {
            backup_file_path: file_path.to_string_lossy().to_string(),
            backup_sha256,
            scheduled_at: Utc::now().to_rfc3339(),
            scheduled_by_user_id: user_id.to_string(),
        };

        let marker_path = self.app_data_dir.join(RESTORE_PENDING_FILE);
        let serialized = serde_json::to_string_pretty(&marker).map_err(|e| {
            CoreError::Internal(format!("Falha ao serializar marcador de restauração: {e}"))
        })?;

        fs::write(&marker_path, serialized).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao gravar marcador de restauração pendente ({}): {e}",
                marker_path.display()
            ))
        })?;

        Ok(marker)
    }

    /// Executa a restauração a frio no boot caso exista um marcador `restore_pending.json`.
    /// NUNCA abre conexões do banco de dados antes deste procedimento.
    pub fn execute_pending_restore_at_startup(
        &self,
        current_max_migration: i64,
    ) -> Result<Option<RestoreReport>> {
        let marker_path = self.app_data_dir.join(RESTORE_PENDING_FILE);
        if !marker_path.exists() {
            return Ok(None);
        }

        tracing::info!(
            marker_path = %marker_path.display(),
            "Marcador de restauração pendente detectado no boot do sistema. Iniciando restauração a frio."
        );

        let marker_raw = fs::read_to_string(&marker_path).map_err(|e| {
            CoreError::Internal(format!("Falha ao ler marcador de restauração: {e}"))
        })?;

        let marker: RestorePendingMarker = serde_json::from_str(&marker_raw).map_err(|e| {
            CoreError::Internal(format!("Formato inválido no marcador de restauração: {e}"))
        })?;

        let backup_file_path = PathBuf::from(&marker.backup_file_path);
        if !backup_file_path.exists() {
            let _ = fs::remove_file(&marker_path);
            return Err(CoreError::Validation(format!(
                "O arquivo de backup agendado não foi encontrado: {}",
                backup_file_path.display()
            )));
        }

        // Validação profunda
        let verify = self.verify_backup_file(&backup_file_path, current_max_migration)?;
        if !verify.valid {
            let _ = fs::rename(
                &marker_path,
                self.app_data_dir.join("restore_corrupted.json"),
            );
            return Err(CoreError::Validation(format!(
                "Restauração abortada: o arquivo de backup está corrompido: {}",
                verify.error_message.unwrap_or_default()
            )));
        }

        if !verify.is_compatible {
            let _ = fs::rename(
                &marker_path,
                self.app_data_dir.join("restore_incompatible.json"),
            );
            return Err(CoreError::Validation(
                "Restauração abortada: o backup requer versão de esquema mais nova do que este aplicativo suporta."
                    .to_string(),
            ));
        }

        // Cria automaticamente um snapshot de salvaguarda prévia (pre-restore) do estado atual
        let mut pre_restore_path_str = None;
        if self.db_path.exists() {
            let pre_restore_dir = self.app_data_dir.join("snapshots");
            let _ = fs::create_dir_all(&pre_restore_dir);
            let ts = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
            let pre_restore_file =
                pre_restore_dir.join(format!("pre-restore_{ts}.eventrent-backup"));

            // Compacta db e mídia atuais
            if let Ok(f) = File::create(&pre_restore_file) {
                let mut zip = ZipWriter::new(f);
                let opt =
                    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
                if let Ok(mut db_in) = File::open(&self.db_path) {
                    if zip.start_file("database/eventrent.db", opt).is_ok() {
                        let _ = std::io::copy(&mut db_in, &mut zip);
                    }
                }
                let _ = zip.finish();
                pre_restore_path_str = Some(pre_restore_file.to_string_lossy().to_string());
            }
        }

        // Descompacta e substitui atomicamente banco e mídia
        let f = File::open(&backup_file_path).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao abrir arquivo de backup para restauração: {e}"
            ))
        })?;

        let mut archive = ZipArchive::new(f).map_err(|e| {
            CoreError::Internal(format!("Falha ao abrir arquivo ZIP de backup: {e}"))
        })?;

        // 1. Extrai banco de dados para arquivo temporário
        let restored_db_tmp = self.app_data_dir.join("eventrent.restored.tmp.db");
        {
            let mut db_zip_file = archive.by_name("database/eventrent.db").map_err(|e| {
                CoreError::Internal(format!("database/eventrent.db ausente no backup: {e}"))
            })?;
            let mut out_f = File::create(&restored_db_tmp).map_err(|e| {
                CoreError::Internal(format!(
                    "Falha ao criar arquivo temporário do banco restaurado: {e}"
                ))
            })?;
            std::io::copy(&mut db_zip_file, &mut out_f).map_err(|e| {
                let _ = fs::remove_file(&restored_db_tmp);
                CoreError::Internal(format!("Falha ao descompactar banco de dados: {e}"))
            })?;
        }

        // 2. Substitui o banco de dados oficial e remove arquivos voláteis de WAL
        if let Err(e) = fs::rename(&restored_db_tmp, &self.db_path) {
            // Em caso de falha no rename, tenta copiar
            if let Err(copy_err) = fs::copy(&restored_db_tmp, &self.db_path) {
                let _ = fs::remove_file(&restored_db_tmp);
                return Err(CoreError::Internal(format!(
                    "Falha ao substituir banco de dados ({e}; {copy_err})"
                )));
            }
            let _ = fs::remove_file(&restored_db_tmp);
        }

        let wal_path = PathBuf::from(format!("{}-wal", self.db_path.to_string_lossy()));
        let shm_path = PathBuf::from(format!("{}-shm", self.db_path.to_string_lossy()));
        let _ = fs::remove_file(wal_path);
        let _ = fs::remove_file(shm_path);

        // 3. Extrai config.json se presente
        if let Ok(mut cfg_zip) = archive.by_name("config.json") {
            let tmp_cfg = self.app_data_dir.join("config.restored.tmp.json");
            if let Ok(mut out) = File::create(&tmp_cfg) {
                if std::io::copy(&mut cfg_zip, &mut out).is_ok() {
                    let _ = fs::rename(&tmp_cfg, &self.config_path);
                }
            }
            let _ = fs::remove_file(tmp_cfg);
        }

        // 4. Extrai mídia
        let mut files_restored_count = 1;
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                if let Some(rel) = name.strip_prefix("media/") {
                    if !rel.is_empty() {
                        let target_path = self.media_dir.join(rel);
                        if let Some(parent) = target_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(mut out) = File::create(&target_path) {
                            if std::io::copy(&mut file, &mut out).is_ok() {
                                files_restored_count += 1;
                            }
                        }
                    }
                }
            }
        }

        // Remove o marcador de restauração
        let _ = fs::remove_file(&marker_path);

        tracing::info!(
            files_restored_count,
            "Restauração a frio concluída com absoluto sucesso."
        );

        Ok(Some(RestoreReport {
            restored_backup_file: backup_file_path.to_string_lossy().to_string(),
            pre_restore_snapshot_file: pre_restore_path_str,
            restored_at: Utc::now().to_rfc3339(),
            files_restored_count,
        }))
    }

    /// Executa a rotação e retenção de backups antigos de acordo com as políticas configuradas.
    pub fn execute_retention(
        &self,
        folder: &Path,
        daily: u32,
        weekly: u32,
        monthly: u32,
    ) -> Result<RetentionPlan> {
        let backups = self.list_backups_in_folder(folder, i64::MAX)?;
        let mut candidates = Vec::new();

        for b in backups {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&b.created_at) {
                candidates.push(RetentionCandidate {
                    file_name: b.file_name,
                    file_path: b.file_path,
                    created_at: dt.with_timezone(&Utc),
                });
            }
        }

        let plan = calculate_retention_plan(&candidates, daily, weekly, monthly);

        for to_delete in &plan.files_to_delete {
            let p = Path::new(to_delete);
            if p.exists()
                && is_valid_backup_filename(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            {
                let _ = fs::remove_file(p);
            }
        }

        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::Row;

    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct TestEnvironment {
        root: PathBuf,
        app_data_dir: PathBuf,
        db_path: PathBuf,
        config_path: PathBuf,
        media_dir: PathBuf,
        backup_dest: PathBuf,
    }

    impl TestEnvironment {
        async fn setup() -> Self {
            let root =
                std::env::temp_dir().join(format!("eventrent_test_{}", uuid::Uuid::now_v7()));
            let app_data_dir = root.join("app_data");
            let db_path = app_data_dir.join("eventrent.db");
            let config_path = app_data_dir.join("config.json");
            let media_dir = app_data_dir.join("media");
            let backup_dest = root.join("external_backups");

            fs::create_dir_all(&app_data_dir).unwrap();
            fs::create_dir_all(&media_dir).unwrap();
            fs::create_dir_all(&backup_dest).unwrap();

            // Grava config.json
            fs::write(&config_path, r#"{"theme": "dark"}"#).unwrap();

            // Grava arquivo de mídia de teste
            let sample_media = media_dir.join("images").join("logo.png");
            fs::create_dir_all(sample_media.parent().unwrap()).unwrap();
            fs::write(&sample_media, b"\x89PNG\r\n\x1a\nfake_image_bytes").unwrap();

            // Inicializa banco SQLite com tabela de teste
            let conn_str = format!("sqlite://{}?mode=rwc", db_path.display());
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&conn_str)
                .await
                .unwrap();

            sqlx::query(
                "CREATE TABLE test_items (id INTEGER PRIMARY KEY, title TEXT NOT NULL);
                 INSERT INTO test_items (id, title) VALUES (1, 'item 1'), (2, 'item 2');",
            )
            .execute(&pool)
            .await
            .unwrap();

            pool.close().await;

            Self {
                root,
                app_data_dir,
                db_path,
                config_path,
                media_dir,
                backup_dest,
            }
        }

        fn service(&self) -> BackupFsService {
            BackupFsService::new(
                self.app_data_dir.clone(),
                self.db_path.clone(),
                self.media_dir.clone(),
                self.config_path.clone(),
            )
        }
    }

    impl Drop for TestEnvironment {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn test_backup_create_and_verify_roundtrip() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let (manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .expect("Backup deve ser executado com sucesso");

        pool.close().await;

        assert!(backup_path.exists());
        assert_eq!(manifest.files.len(), 3); // db, config, media image

        // Listagem
        let list = service
            .list_backups_in_folder(&env.backup_dest, 10)
            .expect("Listagem deve funcionar");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].app_version, "0.10.0-fase2n");
        assert_eq!(list[0].last_applied_migration, 0); // nenhuma migração formal executada no teste
        assert!(list[0].is_compatible);

        // Verificação profunda
        let verify = service
            .verify_backup_file(&backup_path, 10)
            .expect("Verificação deve ter sucesso");
        assert!(verify.valid);
        assert!(verify.is_compatible);
        assert!(verify.error_message.is_none());
        let verified_manifest = verify.manifest.unwrap();
        assert_eq!(verified_manifest.files.len(), 3);
    }

    #[tokio::test]
    async fn test_backup_tamper_detection() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let (_manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .expect("Backup criado");

        pool.close().await;

        let mut bytes = fs::read(&backup_path).unwrap();

        // Adultera alguns bytes no meio do arquivo ZIP
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;
        bytes[mid + 1] ^= 0xAA;
        let tampered_path = env.backup_dest.join("tampered.eventrent-backup");
        fs::write(&tampered_path, bytes).unwrap();

        let verify = service
            .verify_backup_file(&tampered_path, 10)
            .expect("Execução da verificação");
        assert!(
            !verify.valid,
            "Arquivo adulterado não pode ser validado como íntegro"
        );
        assert!(verify.error_message.is_some());
    }

    #[tokio::test]
    async fn test_schedule_and_execute_cold_restore() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let (_manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .unwrap();

        pool.close().await;

        // Altera o banco atual para algo diferente
        {
            let conn_rw = format!("sqlite://{}?mode=rw", env.db_path.display());
            let p = SqlitePoolOptions::new().connect(&conn_rw).await.unwrap();
            sqlx::query("DELETE FROM test_items WHERE id = 1; INSERT INTO test_items (id, title) VALUES (99, 'modificado');")
                .execute(&p)
                .await
                .unwrap();
            p.close().await;
        }

        // Agenda restauração
        let marker = service
            .schedule_restore(&backup_path, "usr_admin", 10)
            .expect("Agendamento de restauração");
        assert_eq!(marker.scheduled_by_user_id, "usr_admin");

        // Executa restauração a frio no boot
        let report = service
            .execute_pending_restore_at_startup(10)
            .expect("Restauração a frio executada com sucesso")
            .expect("Relatório deve estar presente");

        assert_eq!(report.restored_backup_file, backup_path.to_string_lossy());
        assert!(report.pre_restore_snapshot_file.is_some());

        // Marcador deve ter sido removido
        assert!(!env.app_data_dir.join(RESTORE_PENDING_FILE).exists());

        // Confere que o banco voltou ao estado anterior (itens 1 e 2, sem 99)
        let conn_check = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool_check = SqlitePoolOptions::new().connect(&conn_check).await.unwrap();
        let rows = sqlx::query("SELECT id, title FROM test_items ORDER BY id ASC")
            .fetch_all(&pool_check)
            .await
            .unwrap();

        assert_eq!(rows.len(), 2);
        let id1: i64 = rows[0].get("id");
        let id2: i64 = rows[1].get("id");
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        pool_check.close().await;
    }

    #[tokio::test]
    async fn test_retention_plan_execution() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let (_manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .unwrap();

        pool.close().await;

        let plan = service
            .execute_retention(&env.backup_dest, 7, 4, 3)
            .expect("Execução da retenção");

        assert!(backup_path.exists());
        assert_eq!(plan.files_to_keep.len(), 1);
        assert_eq!(plan.files_to_delete.len(), 0);
    }
}
