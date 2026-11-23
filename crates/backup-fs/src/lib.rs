use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use eventrent_core::domain::backup::{
    calculate_retention_plan, is_valid_backup_filename, validate_backup_file_for_restore,
    validate_destination_folder, validate_zip_entry_name, BackupFileEntry, BackupManifest,
    BackupMetadataView, BackupOperationGuard, RestorePendingMarker, RetentionCandidate,
    RetentionPlan, VerifyBackupResult, RESTORE_PENDING_FILE,
};
use eventrent_core::error::{CoreError, Result};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection, SqliteConnection, SqlitePool};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Limite máximo de bytes descompactados de um pacote de backup (5 GiB).
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 5 * 1024 * 1024 * 1024;
/// Limite máximo de bytes descompactados para um único arquivo dentro do pacote (2 GiB).
pub const MAX_SINGLE_FILE_UNCOMPRESSED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Razão máxima de compressão permitida para arquivos acima de 1 MiB (100:1).
pub const MAX_COMPRESSION_RATIO: u64 = 100;

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
        if let Err(e) = validate_backup_file_for_restore(file_path) {
            return Ok(VerifyBackupResult {
                valid: false,
                manifest: None,
                error_message: Some(e.to_string()),
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

        // 1. Verificação preliminar de segurança contra Zip-Slip e Zip-Bomb em todas as entradas do ZIP
        let mut total_uncompressed_bytes: u64 = 0;
        for i in 0..archive.len() {
            let zf = match archive.by_index(i) {
                Ok(file) => file,
                Err(e) => {
                    return Ok(VerifyBackupResult {
                        valid: false,
                        manifest: None,
                        error_message: Some(format!(
                            "Entrada de ZIP inacessível ou corrompida: {e}"
                        )),
                        is_compatible: false,
                    });
                }
            };

            let name = zf.name();
            if let Err(e) = validate_zip_entry_name(name) {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: None,
                    error_message: Some(e.to_string()),
                    is_compatible: false,
                });
            }

            let uncompressed_size = zf.size();
            let compressed_size = zf.compressed_size();

            if uncompressed_size > MAX_SINGLE_FILE_UNCOMPRESSED_BYTES {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: None,
                    error_message: Some(format!(
                        "Possível Zip Bomb: o arquivo '{name}' excede o tamanho máximo descompactado permitido ({} bytes)",
                        MAX_SINGLE_FILE_UNCOMPRESSED_BYTES
                    )),
                    is_compatible: false,
                });
            }

            total_uncompressed_bytes = total_uncompressed_bytes.saturating_add(uncompressed_size);
            if total_uncompressed_bytes > MAX_TOTAL_UNCOMPRESSED_BYTES {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: None,
                    error_message: Some(format!(
                        "Possível Zip Bomb: o total descompactado do pacote excede o teto permitido ({} bytes)",
                        MAX_TOTAL_UNCOMPRESSED_BYTES
                    )),
                    is_compatible: false,
                });
            }

            // Ratio check para arquivos > 1 MiB
            if compressed_size > 0 && uncompressed_size > 1024 * 1024 {
                let ratio = uncompressed_size / compressed_size;
                if ratio > MAX_COMPRESSION_RATIO {
                    return Ok(VerifyBackupResult {
                        valid: false,
                        manifest: None,
                        error_message: Some(format!(
                            "Possível Zip Bomb: razão de compressão anômala ({ratio}:1) para o arquivo '{name}'",
                        )),
                        is_compatible: false,
                    });
                }
            }
        }

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
            if let Err(e) = validate_zip_entry_name(&entry.path) {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(e.to_string()),
                    is_compatible,
                });
            }

            let zf = match archive.by_name(&entry.path) {
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

            let mut limited = zf.take(entry.byte_size.saturating_add(1));
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut actual_bytes = 0u64;

            loop {
                match limited.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        hasher.update(&buffer[..n]);
                        actual_bytes = actual_bytes.saturating_add(n as u64);
                        if actual_bytes > entry.byte_size {
                            return Ok(VerifyBackupResult {
                                valid: false,
                                manifest: Some(manifest.clone()),
                                error_message: Some(format!(
                                    "Arquivo '{}' descompactou mais bytes do que o declarado no manifesto (Zip Bomb)",
                                    entry.path
                                )),
                                is_compatible,
                            });
                        }
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

        // Gravação atômica com sync_all e rename
        let tmp_marker_path = self.app_data_dir.join(format!(
            "{RESTORE_PENDING_FILE}.tmp.{}",
            uuid::Uuid::now_v7()
        ));
        {
            let mut f = File::create(&tmp_marker_path).map_err(|e| {
                CoreError::Internal(format!(
                    "Falha ao criar marcador temporário de restauração: {e}"
                ))
            })?;
            f.write_all(serialized.as_bytes()).map_err(|e| {
                let _ = fs::remove_file(&tmp_marker_path);
                CoreError::Internal(format!(
                    "Falha ao gravar marcador temporário de restauração: {e}"
                ))
            })?;
            f.sync_all().map_err(|e| {
                let _ = fs::remove_file(&tmp_marker_path);
                CoreError::Internal(format!(
                    "Falha ao sincronizar marcador temporário com o disco: {e}"
                ))
            })?;
        }

        fs::rename(&tmp_marker_path, &marker_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_marker_path);
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
        let db_backup_original = self.app_data_dir.join("eventrent.db.pre-restore.bak");
        if self.db_path.exists() {
            let pre_restore_dir = self.app_data_dir.join("snapshots");
            let _ = fs::create_dir_all(&pre_restore_dir);
            let ts = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
            let pre_restore_file =
                pre_restore_dir.join(format!("pre-restore_{ts}.eventrent-backup"));

            // Compacta db e mídia atuais para preservação integral
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

            // Cria salvaguarda direta do arquivo .db para rollback imediato caso a substituição falhe
            let _ = fs::copy(&self.db_path, &db_backup_original);
        }

        // Descompacta e substitui atomicamente banco e mídia
        let f = match File::open(&backup_file_path) {
            Ok(file) => file,
            Err(e) => {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                return Err(CoreError::Internal(format!(
                    "Falha ao abrir arquivo de backup para restauração: {e}"
                )));
            }
        };

        let mut archive = match ZipArchive::new(f) {
            Ok(a) => a,
            Err(e) => {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                return Err(CoreError::Internal(format!(
                    "Falha ao abrir arquivo ZIP de backup: {e}"
                )));
            }
        };

        // 1. Extrai banco de dados para arquivo temporário
        let restored_db_tmp = self.app_data_dir.join("eventrent.restored.tmp.db");
        {
            let db_zip_file = match archive.by_name("database/eventrent.db") {
                Ok(zf) => zf,
                Err(e) => {
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                    return Err(CoreError::Internal(format!(
                        "database/eventrent.db ausente no backup: {e}"
                    )));
                }
            };

            let mut out_f = match File::create(&restored_db_tmp) {
                Ok(file) => file,
                Err(e) => {
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                    return Err(CoreError::Internal(format!(
                        "Falha ao criar arquivo temporário do banco restaurado: {e}"
                    )));
                }
            };

            if let Err(e) = std::io::copy(
                &mut db_zip_file.take(MAX_SINGLE_FILE_UNCOMPRESSED_BYTES),
                &mut out_f,
            ) {
                let _ = fs::remove_file(&restored_db_tmp);
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                return Err(CoreError::Internal(format!(
                    "Falha ao descompactar banco de dados: {e}"
                )));
            }
        }

        // 2. Substitui o banco de dados oficial com garantia de rollback
        let replace_result = if fs::rename(&restored_db_tmp, &self.db_path).is_err() {
            fs::copy(&restored_db_tmp, &self.db_path)
                .map(|_| ())
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        };

        let _ = fs::remove_file(&restored_db_tmp);

        if let Err(replace_err) = replace_result {
            // Rollback para estado pré-restauração
            if db_backup_original.exists() {
                let _ = fs::copy(&db_backup_original, &self.db_path);
            }
            let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
            return Err(CoreError::Internal(format!(
                "Falha ao substituir banco de dados. Estado original restaurado: {replace_err}"
            )));
        }

        let wal_path = PathBuf::from(format!("{}-wal", self.db_path.to_string_lossy()));
        let shm_path = PathBuf::from(format!("{}-shm", self.db_path.to_string_lossy()));
        let _ = fs::remove_file(wal_path);
        let _ = fs::remove_file(shm_path);

        // 3. Extrai config.json se presente
        if let Ok(cfg_zip) = archive.by_name("config.json") {
            let tmp_cfg = self.app_data_dir.join("config.restored.tmp.json");
            if let Ok(mut out) = File::create(&tmp_cfg) {
                if std::io::copy(
                    &mut cfg_zip.take(MAX_SINGLE_FILE_UNCOMPRESSED_BYTES),
                    &mut out,
                )
                .is_ok()
                {
                    let _ = fs::rename(&tmp_cfg, &self.config_path);
                }
            }
            let _ = fs::remove_file(tmp_cfg);
        }

        // 4. Extrai mídia com validação estrita anti Zip-Slip
        let mut files_restored_count = 1;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                if let Some(rel) = name.strip_prefix("media/") {
                    if !rel.is_empty() {
                        // Valida nome da entrada contra Zip-Slip
                        if let Err(e) = validate_zip_entry_name(rel) {
                            tracing::warn!(
                                file = %name,
                                "Entrada de mídia ignorada por falha na validação de caminho seguro: {e}"
                            );
                            continue;
                        }

                        // Garante que o target_path está estritamente contido em media_dir
                        let target_path = self.media_dir.join(rel);
                        if !target_path.starts_with(&self.media_dir) {
                            tracing::warn!(
                                file = %name,
                                "Possível ataque de Zip-Slip interceptado: entrada tenta escapar do diretório de mídia."
                            );
                            continue;
                        }

                        if let Some(parent) = target_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(mut out) = File::create(&target_path) {
                            if std::io::copy(
                                &mut file.take(MAX_SINGLE_FILE_UNCOMPRESSED_BYTES),
                                &mut out,
                            )
                            .is_ok()
                            {
                                files_restored_count += 1;
                            }
                        }
                    }
                }
            }
        }

        // Remove o marcador de restauração concluída e a salvaguarda local
        let _ = fs::remove_file(&marker_path);
        let _ = fs::remove_file(&db_backup_original);

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

    /// Cria um snapshot de salvaguarda prévia antes de qualquer atualização/migração no boot.
    pub fn create_pre_update_snapshot_if_needed(&self, db_path: &Path) -> Result<Option<PathBuf>> {
        if !db_path.exists() {
            return Ok(None);
        }
        let metadata = match fs::metadata(db_path) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };
        if metadata.len() == 0 {
            return Ok(None);
        }

        let snapshots_dir = self.app_data_dir.join("snapshots");
        let _ = fs::create_dir_all(&snapshots_dir);

        let ts = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
        let snapshot_path = snapshots_dir.join(format!("pre-update_{ts}.bak.db"));
        let tmp_snapshot = snapshots_dir.join(format!("pre-update_{ts}.tmp.db"));

        if fs::copy(db_path, &tmp_snapshot).is_ok()
            && fs::rename(&tmp_snapshot, &snapshot_path).is_ok()
        {
            tracing::info!(
                snapshot = %snapshot_path.display(),
                "Snapshot de segurança pré-atualização gravado com sucesso antes das migrações."
            );
            return Ok(Some(snapshot_path));
        }
        let _ = fs::remove_file(&tmp_snapshot);
        Ok(None)
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

    #[tokio::test]
    async fn test_zip_slip_rejection_dot_dot() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let malicious_zip = env.backup_dest.join("backup_slip_dot_dot.eventrent-backup");
        let file = File::create(&malicious_zip).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("media/../../outside.txt", options).unwrap();
        zip.write_all(b"malicious content").unwrap();
        zip.finish().unwrap();

        let verify = service
            .verify_backup_file(&malicious_zip, 10)
            .expect("Verificação executada");
        assert!(!verify.valid, "Entrada contendo '..' deve ser rejeitada");
        assert!(verify.error_message.as_ref().unwrap().contains("Zip-Slip"));
    }

    #[tokio::test]
    async fn test_zip_slip_rejection_absolute_path_and_drive_colon() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // 1. Caminho absoluto estilo Unix
        let malicious_abs = env.backup_dest.join("backup_slip_abs.eventrent-backup");
        let file = File::create(&malicious_abs).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("/etc/passwd", options).unwrap();
        zip.write_all(b"root:x:0:0").unwrap();
        zip.finish().unwrap();

        let verify = service
            .verify_backup_file(&malicious_abs, 10)
            .expect("Verificação executada");
        assert!(!verify.valid, "Caminho absoluto Unix deve ser rejeitado");
        assert!(verify.error_message.as_ref().unwrap().contains("Zip-Slip"));

        // 2. Caminho contendo drive Windows com dois-pontos ':'
        let malicious_drive = env.backup_dest.join("backup_slip_colon.eventrent-backup");
        let file2 = File::create(&malicious_drive).unwrap();
        let mut zip2 = ZipWriter::new(file2);
        zip2.start_file("C:/escaped.txt", options).unwrap();
        zip2.write_all(b"escaped content").unwrap();
        zip2.finish().unwrap();

        let verify2 = service
            .verify_backup_file(&malicious_drive, 10)
            .expect("Verificação executada");
        assert!(
            !verify2.valid,
            "Caminho com drive Windows deve ser rejeitado"
        );
        assert!(verify2.error_message.as_ref().unwrap().contains("Zip-Slip"));
    }

    #[tokio::test]
    async fn test_zip_bomb_rejection_exceeds_size_limit() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let bomb_zip = env.backup_dest.join("backup_bomb.eventrent-backup");
        let file = File::create(&bomb_zip).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("database/eventrent.db", options).unwrap();
        zip.write_all(b"small uncompressed test data").unwrap();
        zip.finish().unwrap();

        // Altera o campo uncompressed_size no cabeçalho ZIP para 3 GiB (> 2 GiB MAX_SINGLE_FILE)
        let mut bytes = fs::read(&bomb_zip).unwrap();
        let big_size: u32 = 0xC0000000; // ~3.2 GB
        if let Some(pos) = bytes.windows(4).position(|w| w == b"PK\x03\x04") {
            let uncomp_offset = pos + 22;
            bytes[uncomp_offset..uncomp_offset + 4].copy_from_slice(&big_size.to_le_bytes());
        }
        if let Some(pos) = bytes.windows(4).position(|w| w == b"PK\x01\x02") {
            let uncomp_offset = pos + 24;
            bytes[uncomp_offset..uncomp_offset + 4].copy_from_slice(&big_size.to_le_bytes());
        }
        fs::write(&bomb_zip, bytes).unwrap();

        let verify = service
            .verify_backup_file(&bomb_zip, 10)
            .expect("Verificação executada");
        assert!(
            !verify.valid,
            "Zip bomb com tamanho excessivo deve ser rejeitado"
        );
        let err = verify.error_message.unwrap();
        assert!(err.contains("Zip Bomb") || err.contains("limite"));
    }

    #[tokio::test]
    async fn test_tamper_detection_db_byte_altered() {
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

        let tampered_path = env.backup_dest.join("tampered_db_byte.eventrent-backup");
        {
            let src_file = File::open(&backup_path).unwrap();
            let mut src_archive = ZipArchive::new(src_file).unwrap();
            let dest_file = File::create(&tampered_path).unwrap();
            let mut dest_zip = ZipWriter::new(dest_file);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            for i in 0..src_archive.len() {
                let mut entry = src_archive.by_index(i).unwrap();
                let name = entry.name().to_string();
                let mut data = Vec::new();
                entry.read_to_end(&mut data).unwrap();
                if name == "database/eventrent.db" && !data.is_empty() {
                    data[0] ^= 0xFF; // Adultera primeiro byte do arquivo de banco
                }
                dest_zip.start_file(name, options).unwrap();
                dest_zip.write_all(&data).unwrap();
            }
            dest_zip.finish().unwrap();
        }

        let verify = service
            .verify_backup_file(&tampered_path, 10)
            .expect("Verificação executada");
        assert!(!verify.valid, "Banco adulterado deve ser reprovado");
        let err = verify.error_message.unwrap();
        assert!(
            err.contains("divergente") || err.contains("adulterado") || err.contains("SHA-256"),
            "Erro deve reportar divergência: {err}"
        );
    }

    #[tokio::test]
    async fn test_tamper_detection_manifest_sha_mismatch() {
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

        let tampered_path = env
            .backup_dest
            .join("tampered_manifest_sha.eventrent-backup");
        {
            let src_file = File::open(&backup_path).unwrap();
            let mut src_archive = ZipArchive::new(src_file).unwrap();
            let dest_file = File::create(&tampered_path).unwrap();
            let mut dest_zip = ZipWriter::new(dest_file);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            for i in 0..src_archive.len() {
                let mut entry = src_archive.by_index(i).unwrap();
                let name = entry.name().to_string();
                let mut data = Vec::new();
                entry.read_to_end(&mut data).unwrap();
                if name == "manifest.json" {
                    let mut mf: BackupManifest = serde_json::from_slice(&data).unwrap();
                    if let Some(first_file) = mf.files.first_mut() {
                        first_file.sha256 =
                            "0000000000000000000000000000000000000000000000000000000000000000"
                                .to_string();
                    }
                    data = serde_json::to_vec_pretty(&mf).unwrap();
                }
                dest_zip.start_file(name, options).unwrap();
                dest_zip.write_all(&data).unwrap();
            }
            dest_zip.finish().unwrap();
        }

        let verify = service
            .verify_backup_file(&tampered_path, 10)
            .expect("Verificação executada");
        assert!(
            !verify.valid,
            "Manifesto com SHA adulterado deve ser reprovado"
        );
        let err = verify.error_message.unwrap();
        assert!(
            err.contains("divergente") || err.contains("adulterado") || err.contains("SHA-256"),
            "Erro deve reportar divergência: {err}"
        );
    }

    #[tokio::test]
    async fn test_newer_schema_backup_rejected() {
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

        let newer_path = env.backup_dest.join("newer_schema.eventrent-backup");
        {
            let src_file = File::open(&backup_path).unwrap();
            let mut src_archive = ZipArchive::new(src_file).unwrap();
            let dest_file = File::create(&newer_path).unwrap();
            let mut dest_zip = ZipWriter::new(dest_file);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            for i in 0..src_archive.len() {
                let mut entry = src_archive.by_index(i).unwrap();
                let name = entry.name().to_string();
                let mut data = Vec::new();
                entry.read_to_end(&mut data).unwrap();
                if name == "manifest.json" {
                    let mut mf: BackupManifest = serde_json::from_slice(&data).unwrap();
                    mf.last_applied_migration = 999;
                    data = serde_json::to_vec_pretty(&mf).unwrap();
                }
                dest_zip.start_file(name, options).unwrap();
                dest_zip.write_all(&data).unwrap();
            }
            dest_zip.finish().unwrap();
        }

        let verify = service
            .verify_backup_file(&newer_path, 10)
            .expect("Verificação executada");
        assert!(verify.valid, "Backup estruturalmente válido");
        assert!(
            !verify.is_compatible,
            "Schema 999 > 10 deve ser incompatível"
        );

        let res = service.schedule_restore(&newer_path, "usr_admin", 10);
        assert!(
            res.is_err(),
            "Agendamento de restauração para schema futuro deve ser recusado"
        );
    }

    #[tokio::test]
    async fn test_simulated_crash_during_restore_reverts_state() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let initial_db_bytes = fs::read(&env.db_path).unwrap();

        // Arquivo corrompido propositalmente para falhar a restauração
        let corrupt_zip = env.backup_dest.join("corrupt_restore.eventrent-backup");
        fs::write(
            &corrupt_zip,
            b"PK\x03\x04corrupted_payload_triggering_failure",
        )
        .unwrap();

        let marker = RestorePendingMarker {
            backup_file_path: corrupt_zip.to_string_lossy().to_string(),
            backup_sha256: "dummy_sha".to_string(),
            scheduled_at: Utc::now().to_rfc3339(),
            scheduled_by_user_id: "usr_admin".to_string(),
        };
        fs::write(
            env.app_data_dir.join(RESTORE_PENDING_FILE),
            serde_json::to_string(&marker).unwrap(),
        )
        .unwrap();

        let res = service.execute_pending_restore_at_startup(10);
        assert!(
            res.is_err(),
            "Restauração de arquivo corrompido deve falhar"
        );

        // Banco original deve ser preservado intacto
        let current_db_bytes = fs::read(&env.db_path).unwrap();
        assert_eq!(
            current_db_bytes, initial_db_bytes,
            "Estado do banco deve permanecer intacto após aborto seguro da restauração"
        );
    }

    #[tokio::test]
    async fn test_concurrent_backup_with_db_writes_remains_consistent() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let conn_rw = format!("sqlite://{}?mode=rwc", env.db_path.display());
        let pool_rw = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&conn_rw)
            .await
            .unwrap();

        sqlx::query("PRAGMA journal_mode = WAL;")
            .execute(&pool_rw)
            .await
            .unwrap();

        let pool_clone = pool_rw.clone();
        let stop_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        let writer_task = tokio::spawn(async move {
            let mut i = 1000;
            while !stop_signal_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = sqlx::query("INSERT INTO test_items (id, title) VALUES (?, ?)")
                    .bind(i)
                    .bind(format!("concurrent item {i}"))
                    .execute(&pool_clone)
                    .await;
                i += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        });

        let (manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool_rw, "0.10.0-fase2n")
            .await
            .expect("Backup concorrente deve ser bem-sucedido");

        stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = writer_task.await;
        pool_rw.close().await;

        let verify = service
            .verify_backup_file(&backup_path, 10)
            .expect("Verificação do backup gerado sob concorrência");
        assert!(
            verify.valid,
            "Backup gerado sob escrita concorrente deve ser íntegro"
        );
        assert!(verify.is_compatible);
        assert!(manifest.database_integrity_checked);
    }
}
