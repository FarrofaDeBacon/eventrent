use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
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

/// Valida que o caminho relativo de mídia segue rigorosamente a estrutura canônica
/// `media/<tenant_id>/<2_primeiros_hex>/<sha256_64_hex>`.
pub fn validate_canonical_media_path(rel: &str) -> Result<()> {
    validate_zip_entry_name(rel)?;
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() != 3 {
        return Err(CoreError::Validation(format!(
            "Caminho de mídia deve seguir rigorosamente a estrutura canônica media/<tenant>/<prefix2>/<hash64>. Encontrado: media/{rel}"
        )));
    }
    let tenant = parts[0];
    let prefix2 = parts[1];
    let hash64 = parts[2];
    if tenant.is_empty() || prefix2.len() != 2 || hash64.len() != 64 {
        return Err(CoreError::Validation(format!(
            "Segmentos de mídia inválidos no caminho: media/{rel}"
        )));
    }
    if !hash64.starts_with(prefix2) || !hash64.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::Validation(format!(
            "Nome do arquivo de mídia deve ser o hash SHA-256 canônico em hexadecimal de 64 caracteres: {hash64}"
        )));
    }
    Ok(())
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
        let app_data_dir_buf = self.app_data_dir.clone();
        let db_path_buf = self.db_path.clone();
        let media_dir_buf = self.media_dir.clone();
        let media_dir_buf_clone = self.media_dir.clone();
        let config_path_buf = self.config_path.clone();
        let config_path_buf_clone = self.config_path.clone();
        let app_version_string = app_version.to_string();

        let manifest = tokio::task::spawn_blocking(move || {
            let temp_zip_filename = format!(
                ".tmp.eventrent-backup-{}.eventrent-backup",
                uuid::Uuid::now_v7()
            );
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

            // 3.2. Adiciona config.json (obrigatório: se ausente ou com erro, aborta imediatamente sem gravar backup)
            if !config_path_buf.exists() {
                let _ = fs::remove_file(&temp_zip_path);
                return Err(CoreError::Validation(format!(
                    "Arquivo de configuração obrigatório ausente: {}",
                    config_path_buf.display()
                )));
            }

            let mut cfg_f = File::open(&config_path_buf).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!(
                    "Falha ao abrir arquivo de configuração ({}): {e}",
                    config_path_buf.display()
                ))
            })?;

            zip.start_file("config.json", deflate_opts).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!("Falha ao iniciar config.json no zip: {e}"))
            })?;

            let mut cfg_hasher = Sha256::new();
            let mut cfg_buffer = [0u8; 64 * 1024];
            let mut cfg_total_bytes = 0u64;
            loop {
                let read = cfg_f.read(&mut cfg_buffer).map_err(|e| {
                    let _ = fs::remove_file(&temp_zip_path);
                    CoreError::Internal(format!(
                        "Falha ao ler arquivo de configuração ({}): {e}",
                        config_path_buf.display()
                    ))
                })?;
                if read == 0 {
                    break;
                }
                cfg_hasher.update(&cfg_buffer[..read]);
                cfg_total_bytes += read as u64;
                zip.write_all(&cfg_buffer[..read]).map_err(|e| {
                    let _ = fs::remove_file(&temp_zip_path);
                    CoreError::Internal(format!(
                        "Falha ao gravar config.json no zip ({}): {e}",
                        config_path_buf.display()
                    ))
                })?;
            }

            file_entries.push(BackupFileEntry {
                path: "config.json".to_string(),
                byte_size: cfg_total_bytes,
                sha256: format!("{:x}", cfg_hasher.finalize()),
            });

            // 3.3. Adiciona arquivos de mídia (modo Stored / sem recompressão)
            let mut skipped_media_files = 0u64;
            if media_dir_buf.exists() {
                if !media_dir_buf.is_dir() {
                    let _ = fs::remove_file(&temp_zip_path);
                    return Err(CoreError::Validation(format!(
                        "Diretório de mídia informado não é uma pasta válida: {}",
                        media_dir_buf.display()
                    )));
                }

                let mut stack = vec![media_dir_buf.clone()];
                while let Some(dir) = stack.pop() {
                    let entries = fs::read_dir(&dir).map_err(|e| {
                        let _ = fs::remove_file(&temp_zip_path);
                        CoreError::Internal(format!(
                            "Falha ao listar diretório de mídia ({}): {e}",
                            dir.display()
                        ))
                    })?;

                    for entry_res in entries {
                        let entry = entry_res.map_err(|e| {
                            let _ = fs::remove_file(&temp_zip_path);
                            CoreError::Internal(format!(
                                "Falha ao ler entrada no diretório de mídia ({}): {e}",
                                dir.display()
                            ))
                        })?;

                        let path = entry.path();
                        if path.is_dir() {
                            stack.push(path);
                        } else if path.is_file() {
                            let rel = match path.strip_prefix(&media_dir_buf) {
                                Ok(r) => r,
                                Err(_) => {
                                    skipped_media_files += 1;
                                    continue;
                                }
                            };

                            let rel_slash = rel.to_string_lossy().replace('\\', "/");

                            // Rejeita qualquer arquivo não-canônico (ex: Thumbs.db, .DS_Store, *.restore_tmp, *.tmp.*).
                            // Apenas caminhos estritamente no padrão media/<tenant>/<2 hex>/<64 hex> são aceitos.
                            if validate_canonical_media_path(&rel_slash).is_err() {
                                tracing::warn!(
                                    path = %path.display(),
                                    rel = %rel_slash,
                                    "Arquivo não-canônico na pasta de mídia ignorado durante backup"
                                );
                                skipped_media_files += 1;
                                continue;
                            }

                            // Extrai o hash esperado a partir do nome do arquivo (último segmento de 64 hex)
                            let expected_hash = match rel_slash.rsplit('/').next() {
                                Some(h) if h.len() == 64 => h,
                                _ => {
                                    skipped_media_files += 1;
                                    continue;
                                }
                            };

                            // Lê o arquivo para verificar se o hash SHA-256 do conteúdo realmente bate com o nome
                            let mut mf = File::open(&path).map_err(|e| {
                                let _ = fs::remove_file(&temp_zip_path);
                                CoreError::Internal(format!(
                                    "Falha ao abrir arquivo de mídia para backup ({}): {e}",
                                    path.display()
                                ))
                            })?;

                            let mut hasher = Sha256::new();
                            let mut buffer = [0u8; 64 * 1024];
                            let mut total_bytes = 0u64;

                            loop {
                                let n = mf.read(&mut buffer).map_err(|e| {
                                    let _ = fs::remove_file(&temp_zip_path);
                                    CoreError::Internal(format!(
                                        "Falha ao ler arquivo de mídia ({}): {e}",
                                        path.display()
                                    ))
                                })?;
                                if n == 0 {
                                    break;
                                }
                                hasher.update(&buffer[..n]);
                                total_bytes += n as u64;
                            }

                            let actual_hash = format!("{:x}", hasher.finalize());
                            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                                tracing::warn!(
                                    path = %path.display(),
                                    expected = %expected_hash,
                                    actual = %actual_hash,
                                    "Arquivo de mídia com conteúdo divergente do nome hash ignorado durante backup"
                                );
                                skipped_media_files += 1;
                                continue;
                            }

                            // Grava no arquivo ZIP o arquivo canônico validado
                            let zip_entry_name = format!("media/{rel_slash}");
                            validate_zip_entry_name(&zip_entry_name).inspect_err(|_| {
                                let _ = fs::remove_file(&temp_zip_path);
                            })?;

                            mf.seek(std::io::SeekFrom::Start(0)).map_err(|e| {
                                let _ = fs::remove_file(&temp_zip_path);
                                CoreError::Internal(format!(
                                    "Falha ao reposicionar ponteiro do arquivo de mídia ({}): {e}",
                                    path.display()
                                ))
                            })?;

                            zip.start_file(&zip_entry_name, stored_opts).map_err(|e| {
                                let _ = fs::remove_file(&temp_zip_path);
                                CoreError::Internal(format!(
                                    "Falha ao iniciar entrada de mídia no zip ({}): {e}",
                                    zip_entry_name
                                ))
                            })?;

                            std::io::copy(&mut mf, &mut zip).map_err(|e| {
                                let _ = fs::remove_file(&temp_zip_path);
                                CoreError::Internal(format!(
                                    "Falha ao gravar arquivo de mídia no zip ({}): {e}",
                                    path.display()
                                ))
                            })?;

                            file_entries.push(BackupFileEntry {
                                path: zip_entry_name,
                                byte_size: total_bytes,
                                sha256: actual_hash,
                            });
                        }
                    }
                }
            }

            // 3.4. Monta e grava manifest.json na raiz do zip (após todos os arquivos gravados e conferidos)
            let manifest = BackupManifest {
                manifest_version: 1,
                app_version: app_version_string,
                created_at: Utc::now().to_rfc3339(),
                last_applied_migration,
                migrations_checksum,
                database_integrity_checked: true,
                files: file_entries,
                table_counts,
                skipped_media_files,
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

            // 3.5. Verificação profunda e revalidação do arquivo temporário ANTES do rename
            let test_service = BackupFsService::new(
                app_data_dir_buf,
                db_path_buf,
                media_dir_buf_clone,
                config_path_buf_clone,
            );
            let verify_res = test_service.verify_backup_file(&temp_zip_path, i64::MAX);
            match verify_res {
                Ok(v) if v.valid => {
                    // Validado com sucesso
                }
                Ok(v) => {
                    let _ = fs::remove_file(&temp_zip_path);
                    return Err(CoreError::Validation(format!(
                        "Backup gerado falhou na auto-verificação de integridade prévia: {}",
                        v.error_message.unwrap_or_default()
                    )));
                }
                Err(e) => {
                    let _ = fs::remove_file(&temp_zip_path);
                    return Err(CoreError::Internal(format!(
                        "Falha ao executar auto-verificação prévia no backup gerado: {e}"
                    )));
                }
            }

            // 3.6. Renomeação atômica do arquivo temporário para o arquivo final
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
                                        // A listagem lê apenas o manifesto; a verificação profunda com conferência
                                        // criptográfica de SHA-256 de todas as entradas ocorre sob demanda via verify_backup.
                                        verified: false,
                                        skipped_media_files: manifest.skipped_media_files,
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
    /// NUNCA abre conexões do banco de dados principal antes deste procedimento.
    pub async fn execute_pending_restore_at_startup(
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

        // Validação profunda de integridade do arquivo
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

        // Validação de correspondência de hash do arquivo agendado
        let mut f_chk = File::open(&backup_file_path).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao abrir backup para conferência de hash: {e}"
            ))
        })?;
        let mut hasher = Sha256::new();
        let mut chk_buf = [0u8; 64 * 1024];
        loop {
            let n = f_chk.read(&mut chk_buf).map_err(|e| {
                CoreError::Internal(format!("Erro ao ler backup para verificação de hash: {e}"))
            })?;
            if n == 0 {
                break;
            }
            hasher.update(&chk_buf[..n]);
        }
        let current_sha = format!("{:x}", hasher.finalize());
        if current_sha != marker.backup_sha256 {
            let _ = fs::rename(
                &marker_path,
                self.app_data_dir.join("restore_tampered.json"),
            );
            return Err(CoreError::Validation(format!(
                "Restauração abortada: o hash do arquivo de backup divergiu desde o agendamento (esperado: {}, atual: {})",
                marker.backup_sha256, current_sha
            )));
        }

        // Abre arquivo ZIP e lê manifesto
        let f = File::open(&backup_file_path).map_err(|e| {
            let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
            CoreError::Internal(format!(
                "Falha ao abrir arquivo de backup para restauração: {e}"
            ))
        })?;

        let mut archive = ZipArchive::new(f).map_err(|e| {
            let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
            CoreError::Internal(format!("Falha ao abrir arquivo ZIP de backup: {e}"))
        })?;

        let manifest = match verify.manifest {
            Some(m) => m,
            None => {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                return Err(CoreError::Validation(
                    "Manifesto ausente no backup".to_string(),
                ));
            }
        };

        // Validação de limites declarados no manifesto
        let mut total_declared_bytes: u64 = 0;
        let mut manifest_map: HashMap<String, BackupFileEntry> = HashMap::new();
        for entry in &manifest.files {
            if entry.byte_size > MAX_SINGLE_FILE_UNCOMPRESSED_BYTES {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                return Err(CoreError::Validation(format!(
                    "Entrada '{}' excede o limite máximo de tamanho de arquivo único (2 GiB)",
                    entry.path
                )));
            }
            total_declared_bytes += entry.byte_size;
            manifest_map.insert(entry.path.clone(), entry.clone());
        }

        if total_declared_bytes > MAX_TOTAL_UNCOMPRESSED_BYTES {
            let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
            return Err(CoreError::Validation(format!(
                "Tamanho total descompactado declarado ({total_declared_bytes} bytes) excede o limite máximo permitido (5 GiB)"
            )));
        }

        // Validação estrita de entradas no ZIP: NENHUMA entrada fora do manifesto é permitida
        for i in 0..archive.len() {
            let zf = archive.by_index(i).map_err(|e| {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!("Falha ao ler entrada de índice {i} no zip: {e}"))
            })?;
            let name = zf.name().to_string();
            if name == "manifest.json" {
                continue;
            }
            if !manifest_map.contains_key(&name) {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                return Err(CoreError::Validation(format!(
                    "Entrada não declarada no manifesto detectada no arquivo de backup: {name}"
                )));
            }
            if let Some(rel) = name.strip_prefix("media/") {
                validate_canonical_media_path(rel).inspect_err(|_| {
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                })?;
            }
        }

        // Garante presença obrigatória do banco de dados no manifesto
        if !manifest_map.contains_key("database/eventrent.db") {
            let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
            return Err(CoreError::Validation(
                "database/eventrent.db ausente no manifesto do backup".to_string(),
            ));
        }

        // Cria snapshot de salvaguarda prévia (pre-restore) cobrindo banco e mídia com propagação estrita de erros
        let mut pre_restore_path_str = None;
        let db_backup_original = self.app_data_dir.join("eventrent.db.pre-restore.bak");
        let _ = fs::remove_file(&db_backup_original);

        if self.db_path.exists() {
            let pre_restore_dir = self.app_data_dir.join("snapshots");
            fs::create_dir_all(&pre_restore_dir).map_err(|e| {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!(
                    "Falha ao criar diretório snapshots para pré-restauração: {e}"
                ))
            })?;

            let ts = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
            let pre_restore_file =
                pre_restore_dir.join(format!("pre-restore_{ts}.eventrent-backup"));
            let pre_restore_tmp =
                pre_restore_dir.join(format!("pre-restore_{ts}.tmp.eventrent-backup"));
            let _ = fs::remove_file(&pre_restore_tmp);

            let f_pre = File::create(&pre_restore_tmp).map_err(|e| {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!("Falha ao criar arquivo de pre-restore: {e}"))
            })?;
            let mut zip_pre = ZipWriter::new(f_pre);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            // Grava banco no pre-restore
            zip_pre
                .start_file("database/eventrent.db", opt)
                .map_err(|e| {
                    let _ = fs::remove_file(&pre_restore_tmp);
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                    CoreError::Internal(format!("Falha ao iniciar db no pre-restore: {e}"))
                })?;
            let mut db_in = File::open(&self.db_path).map_err(|e| {
                let _ = fs::remove_file(&pre_restore_tmp);
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!("Falha ao ler db para pre-restore: {e}"))
            })?;
            std::io::copy(&mut db_in, &mut zip_pre).map_err(|e| {
                let _ = fs::remove_file(&pre_restore_tmp);
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!("Falha ao gravar db no pre-restore: {e}"))
            })?;

            // Grava config.json se existir
            if self.config_path.exists() {
                zip_pre.start_file("config.json", opt).map_err(|e| {
                    let _ = fs::remove_file(&pre_restore_tmp);
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                    CoreError::Internal(format!("Falha ao iniciar config no pre-restore: {e}"))
                })?;
                let mut cfg_in = File::open(&self.config_path).map_err(|e| {
                    let _ = fs::remove_file(&pre_restore_tmp);
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                    CoreError::Internal(format!("Falha ao ler config para pre-restore: {e}"))
                })?;
                std::io::copy(&mut cfg_in, &mut zip_pre).map_err(|e| {
                    let _ = fs::remove_file(&pre_restore_tmp);
                    let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                    CoreError::Internal(format!("Falha ao gravar config no pre-restore: {e}"))
                })?;
            }

            // Grava diretório media/ no pre-restore
            if self.media_dir.exists() && self.media_dir.is_dir() {
                let mut stack = vec![self.media_dir.clone()];
                while let Some(current_dir) = stack.pop() {
                    let rd = fs::read_dir(&current_dir).map_err(|e| {
                        let _ = fs::remove_file(&pre_restore_tmp);
                        let _ =
                            fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                        CoreError::Internal(format!("Falha ao listar media para pre-restore: {e}"))
                    })?;
                    for entry in rd {
                        let entry = entry.map_err(|e| {
                            let _ = fs::remove_file(&pre_restore_tmp);
                            let _ = fs::rename(
                                &marker_path,
                                self.app_data_dir.join("restore_failed.json"),
                            );
                            CoreError::Internal(format!("Falha ao ler entrada de media: {e}"))
                        })?;
                        let p = entry.path();
                        if p.is_dir() {
                            stack.push(p);
                        } else if p.is_file() {
                            let rel = p.strip_prefix(&self.media_dir).map_err(|e| {
                                let _ = fs::remove_file(&pre_restore_tmp);
                                let _ = fs::rename(
                                    &marker_path,
                                    self.app_data_dir.join("restore_failed.json"),
                                );
                                CoreError::Internal(format!(
                                    "Falha ao calcular relativo de media: {e}"
                                ))
                            })?;
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            let stored_opts = SimpleFileOptions::default()
                                .compression_method(CompressionMethod::Stored);
                            zip_pre
                                .start_file(format!("media/{rel_str}"), stored_opts)
                                .map_err(|e| {
                                    let _ = fs::remove_file(&pre_restore_tmp);
                                    let _ = fs::rename(
                                        &marker_path,
                                        self.app_data_dir.join("restore_failed.json"),
                                    );
                                    CoreError::Internal(format!(
                                        "Falha ao iniciar media/{rel_str} no pre-restore: {e}"
                                    ))
                                })?;
                            let mut mf = File::open(&p).map_err(|e| {
                                let _ = fs::remove_file(&pre_restore_tmp);
                                let _ = fs::rename(
                                    &marker_path,
                                    self.app_data_dir.join("restore_failed.json"),
                                );
                                CoreError::Internal(format!("Falha ao ler {}: {e}", p.display()))
                            })?;
                            std::io::copy(&mut mf, &mut zip_pre).map_err(|e| {
                                let _ = fs::remove_file(&pre_restore_tmp);
                                let _ = fs::rename(
                                    &marker_path,
                                    self.app_data_dir.join("restore_failed.json"),
                                );
                                CoreError::Internal(format!("Falha ao gravar {}: {e}", p.display()))
                            })?;
                        }
                    }
                }
            }

            zip_pre.finish().map_err(|e| {
                let _ = fs::remove_file(&pre_restore_tmp);
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!("Falha ao finalizar pacote zip de pre-restore: {e}"))
            })?;

            fs::rename(&pre_restore_tmp, &pre_restore_file).map_err(|e| {
                let _ = fs::remove_file(&pre_restore_tmp);
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!("Falha ao renomear arquivo de pre-restore: {e}"))
            })?;

            // Cópia direta do .db para salvaguarda de rollback
            fs::copy(&self.db_path, &db_backup_original).map_err(|e| {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!(
                    "Falha ao criar salvaguarda direta do banco .db: {e}"
                ))
            })?;

            pre_restore_path_str = Some(pre_restore_file.to_string_lossy().to_string());
        }

        // Rastreamento de arquivos criados para garantia de rollback integral
        let mut extracted_media_paths: Vec<PathBuf> = Vec::new();
        let restored_db_tmp = self.app_data_dir.join("eventrent.restored.tmp.db");
        let restored_cfg_tmp = self.app_data_dir.join("config.restored.tmp.json");
        let _ = fs::remove_file(&restored_db_tmp);
        let _ = fs::remove_file(&restored_cfg_tmp);

        let rollback = |extracted: &[PathBuf]| {
            let _ = fs::remove_file(&restored_db_tmp);
            let _ = fs::remove_file(&restored_cfg_tmp);
            for p in extracted {
                let _ = fs::remove_file(p);
            }
            if db_backup_original.exists() {
                let _ = fs::copy(&db_backup_original, &self.db_path);
            }
            let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
        };

        // 1. Extrai e valida o banco de dados
        {
            let db_entry_meta = manifest_map.get("database/eventrent.db").unwrap();
            let mut db_zip_file = match archive.by_name("database/eventrent.db") {
                Ok(zf) => zf,
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "database/eventrent.db ausente no zip: {e}"
                    )));
                }
            };

            let mut out_db = match File::create(&restored_db_tmp) {
                Ok(f) => f,
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao criar temporário do db restaurado: {e}"
                    )));
                }
            };

            let mut db_hasher = Sha256::new();
            let mut db_total_bytes = 0u64;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = match db_zip_file.read(&mut buf) {
                    Ok(read) => read,
                    Err(e) => {
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Internal(format!(
                            "Erro ao descompactar banco de dados: {e}"
                        )));
                    }
                };
                if n == 0 {
                    break;
                }
                db_total_bytes += n as u64;
                if db_total_bytes > MAX_SINGLE_FILE_UNCOMPRESSED_BYTES {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Validation(
                        "database/eventrent.db excede limite de 2 GiB".to_string(),
                    ));
                }
                db_hasher.update(&buf[..n]);
                if let Err(e) = out_db.write_all(&buf[..n]) {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao gravar banco descompactado: {e}"
                    )));
                }
            }

            if db_total_bytes != db_entry_meta.byte_size {
                rollback(&extracted_media_paths);
                return Err(CoreError::Validation(format!(
                    "Tamanho do banco divergente do manifesto: esperado {}, obtido {}",
                    db_entry_meta.byte_size, db_total_bytes
                )));
            }

            let computed_db_sha = format!("{:x}", db_hasher.finalize());
            if computed_db_sha != db_entry_meta.sha256 {
                rollback(&extracted_media_paths);
                return Err(CoreError::Validation(format!(
                    "Hash SHA-256 do banco divergente do manifesto: esperado {}, obtido {}",
                    db_entry_meta.sha256, computed_db_sha
                )));
            }
        }

        // 2. Validação física de integridade no banco restaurado via PRAGMA integrity_check
        {
            let check_opts = SqliteConnectOptions::new()
                .filename(&restored_db_tmp)
                .read_only(true);
            let check_pool = match sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(check_opts)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao conectar no banco restaurado para integrity_check: {e}"
                    )));
                }
            };

            let integrity_res: std::result::Result<String, sqlx::Error> =
                sqlx::query_scalar("PRAGMA integrity_check")
                    .fetch_one(&check_pool)
                    .await;
            check_pool.close().await;

            match integrity_res {
                Ok(ref integrity) if integrity == "ok" => {}
                Ok(integrity) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Validation(format!(
                        "Restauração abortada: PRAGMA integrity_check reprovou o banco: {integrity}"
                    )));
                }
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao executar PRAGMA integrity_check no banco restaurado: {e}"
                    )));
                }
            }
        }

        // 3. Extrai config.json se presente
        let has_config = manifest_map.contains_key("config.json");
        if has_config {
            let cfg_meta = manifest_map.get("config.json").unwrap();
            let mut cfg_zip = match archive.by_name("config.json") {
                Ok(zf) => zf,
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "config.json declarado mas ausente no zip: {e}"
                    )));
                }
            };
            let mut out_cfg = match File::create(&restored_cfg_tmp) {
                Ok(f) => f,
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao criar temporário de config.json: {e}"
                    )));
                }
            };
            let mut cfg_hasher = Sha256::new();
            let mut cfg_total_bytes = 0u64;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = match cfg_zip.read(&mut buf) {
                    Ok(read) => read,
                    Err(e) => {
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Internal(format!(
                            "Erro ao ler config.json no zip: {e}"
                        )));
                    }
                };
                if n == 0 {
                    break;
                }
                cfg_total_bytes += n as u64;
                cfg_hasher.update(&buf[..n]);
                if let Err(e) = out_cfg.write_all(&buf[..n]) {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao gravar config.json temporário: {e}"
                    )));
                }
            }
            if cfg_total_bytes != cfg_meta.byte_size
                || format!("{:x}", cfg_hasher.finalize()) != cfg_meta.sha256
            {
                rollback(&extracted_media_paths);
                return Err(CoreError::Validation(
                    "config.json adulterado ou com hash divergente do manifesto".to_string(),
                ));
            }
        }

        // 4. Extrai mídia com validação SHA individual e verificação canônica
        let mut files_restored_count = 1; // database contado
        for i in 0..archive.len() {
            let mut zf = match archive.by_index(i) {
                Ok(file) => file,
                Err(e) => {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao acessar entrada de mídia {i}: {e}"
                    )));
                }
            };
            let name = zf.name().to_string();
            if let Some(rel) = name.strip_prefix("media/") {
                if rel.is_empty() {
                    continue;
                }
                let meta = match manifest_map.get(&name) {
                    Some(m) => m,
                    None => {
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Validation(format!(
                            "Arquivo de mídia não declarado: {name}"
                        )));
                    }
                };

                let target_path = self.media_dir.join(rel);
                if !target_path.starts_with(&self.media_dir) {
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Validation(format!(
                        "Tentativa de Zip-Slip interceptada: {name}"
                    )));
                }

                // Verifica se o arquivo de mídia já existe previamente com o mesmo tamanho e SHA-256.
                // Como o armazenamento de mídia é endereçado por conteúdo (CAS), arquivos preexistentes
                // idênticos não precisam ser regravados e JAMAIS devem entrar em extracted_media_paths,
                // garantindo que um rollback eventual não destrua mídias preexistentes legítimas do banco atual.
                let already_present_and_valid = if target_path.is_file() {
                    if let Ok(existing_meta) = fs::metadata(&target_path) {
                        if existing_meta.len() == meta.byte_size {
                            if let Ok(mut ef) = File::open(&target_path) {
                                let mut e_hasher = Sha256::new();
                                let mut e_buf = [0u8; 64 * 1024];
                                while let Ok(n) = ef.read(&mut e_buf) {
                                    if n == 0 {
                                        break;
                                    }
                                    e_hasher.update(&e_buf[..n]);
                                }
                                let e_sha = format!("{:x}", e_hasher.finalize());
                                e_sha.eq_ignore_ascii_case(&meta.sha256)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if already_present_and_valid {
                    // Arquivo idêntico já existe previamente: preserva intacto e não adiciona a extracted_media_paths
                    files_restored_count += 1;
                    continue;
                }

                if let Some(parent) = target_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Internal(format!(
                            "Falha ao criar diretório para {name}: {e}"
                        )));
                    }
                }

                let target_tmp = target_path.with_extension("restore_tmp");
                let mut out_m = match File::create(&target_tmp) {
                    Ok(f) => f,
                    Err(e) => {
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Internal(format!(
                            "Falha ao criar arquivo de mídia temporário: {e}"
                        )));
                    }
                };

                let mut m_hasher = Sha256::new();
                let mut m_total_bytes = 0u64;
                let mut buf = [0u8; 64 * 1024];
                loop {
                    let n = match zf.read(&mut buf) {
                        Ok(read) => read,
                        Err(e) => {
                            let _ = fs::remove_file(&target_tmp);
                            rollback(&extracted_media_paths);
                            return Err(CoreError::Internal(format!("Erro ao ler {name}: {e}")));
                        }
                    };
                    if n == 0 {
                        break;
                    }
                    m_total_bytes += n as u64;
                    if m_total_bytes > MAX_SINGLE_FILE_UNCOMPRESSED_BYTES {
                        let _ = fs::remove_file(&target_tmp);
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Validation(format!(
                            "{name} excede o limite de 2 GiB"
                        )));
                    }
                    m_hasher.update(&buf[..n]);
                    if let Err(e) = out_m.write_all(&buf[..n]) {
                        let _ = fs::remove_file(&target_tmp);
                        rollback(&extracted_media_paths);
                        return Err(CoreError::Internal(format!(
                            "Falha ao gravar mídia {name}: {e}"
                        )));
                    }
                }

                let computed_m_sha = format!("{:x}", m_hasher.finalize());
                let parts: Vec<&str> = rel.split('/').collect();
                let expected_filename_hash = parts.last().copied().unwrap_or_default();

                if m_total_bytes != meta.byte_size
                    || computed_m_sha != meta.sha256
                    || computed_m_sha != expected_filename_hash
                {
                    let _ = fs::remove_file(&target_tmp);
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Validation(format!(
                        "Hash ou tamanho divergente no arquivo de mídia {name} (calculado: {computed_m_sha}, manifesto: {})",
                        meta.sha256
                    )));
                }

                if let Err(e) = fs::rename(&target_tmp, &target_path) {
                    let _ = fs::remove_file(&target_tmp);
                    rollback(&extracted_media_paths);
                    return Err(CoreError::Internal(format!(
                        "Falha ao mover mídia restaurada {name}: {e}"
                    )));
                }
                extracted_media_paths.push(target_path);
                files_restored_count += 1;
            }
        }

        // 5. Substituição atômica final do banco e config
        let replace_result = if fs::rename(&restored_db_tmp, &self.db_path).is_err() {
            fs::copy(&restored_db_tmp, &self.db_path)
                .map(|_| ())
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        };

        let _ = fs::remove_file(&restored_db_tmp);

        if let Err(replace_err) = replace_result {
            rollback(&extracted_media_paths);
            return Err(CoreError::Internal(format!(
                "Falha ao substituir banco de dados. Estado original restaurado: {replace_err}"
            )));
        }

        let wal_path = PathBuf::from(format!("{}-wal", self.db_path.to_string_lossy()));
        let shm_path = PathBuf::from(format!("{}-shm", self.db_path.to_string_lossy()));
        let _ = fs::remove_file(wal_path);
        let _ = fs::remove_file(shm_path);

        if has_config && restored_cfg_tmp.exists() {
            let _ = fs::rename(&restored_cfg_tmp, &self.config_path);
        }
        let _ = fs::remove_file(&restored_cfg_tmp);

        // Remove o marcador de restauração pendente e o backup original de rollback
        let _ = fs::remove_file(&marker_path);
        let _ = fs::remove_file(&db_backup_original);

        tracing::info!(
            files_restored_count,
            "Restauração a frio concluída com absoluto sucesso e integridade total comprovada."
        );

        Ok(Some(RestoreReport {
            restored_backup_file: backup_file_path.to_string_lossy().to_string(),
            pre_restore_snapshot_file: pre_restore_path_str,
            restored_at: Utc::now().to_rfc3339(),
            files_restored_count,
        }))
    }

    /// Cria um snapshot de salvaguarda prévia antes de qualquer atualização/migração no boot.
    ///
    /// Regras:
    /// 1. Somente cria se o arquivo `db_path` existir e tiver tamanho > 0.
    /// 2. Abre conexão e consulta `_sqlx_migrations`. Se a tabela existir e `max_version >= current_max_migration`, não há migração pendente: retorna Ok(None).
    /// 3. Se houver migração pendente (`max_version < current_max_migration`):
    ///    - Executa snapshot consistente via `VACUUM INTO`, incorporando atomicamente todos os dados pendentes de WAL/SHM.
    ///    - Valida o snapshot gerado através de `PRAGMA integrity_check`.
    ///    - Move para `snapshots/pre-update_{ts}.bak.db`.
    ///    - Executa retenção própria de snapshots mantendo apenas os 3 mais recentes em `snapshots/`, removendo os mais antigos.
    pub async fn create_pre_update_snapshot_if_needed(
        &self,
        db_path: &Path,
        current_max_migration: i64,
    ) -> Result<Option<PathBuf>> {
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

        // Abre conexão somente-leitura para conferir se há migração pendente
        let ro_opts = SqliteConnectOptions::new()
            .filename(db_path)
            .read_only(true);
        let pool = match sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(ro_opts)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Não foi possível abrir banco para conferir migrações: {e}");
                return Ok(None);
            }
        };

        // Verifica se a tabela _sqlx_migrations existe
        let table_exists: bool = match sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
        )
        .fetch_one(&pool)
        .await
        {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        let max_applied: i64 = if table_exists {
            match sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
            {
                Ok(Some(v)) => v,
                _ => 0,
            }
        } else {
            // Se o arquivo existe e tem dados (>0 bytes), mas não tem _sqlx_migrations,
            // todas as migrações estão pendentes.
            0
        };

        pool.close().await;

        if max_applied >= current_max_migration {
            tracing::info!(
                max_applied,
                current_max_migration,
                "Banco de dados já está na versão máxima de migração; nenhum snapshot pré-atualização é necessário."
            );
            return Ok(None);
        }

        tracing::info!(
            max_applied,
            current_max_migration,
            "Migração pendente detectada! Gerando snapshot de segurança pré-atualização..."
        );

        let snapshots_dir = self.app_data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir).map_err(|e| {
            CoreError::Internal(format!("Falha ao criar diretório de snapshots: {e}"))
        })?;

        let ts = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
        let snapshot_path = snapshots_dir.join(format!("pre-update_{ts}.bak.db"));
        let tmp_snapshot = snapshots_dir.join(format!("pre-update_{ts}.tmp.db"));
        let _ = fs::remove_file(&tmp_snapshot);

        // Conecta ao banco para snapshot consistente via VACUUM INTO
        let rw_opts = SqliteConnectOptions::new()
            .filename(db_path)
            .read_only(false);
        let rw_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(rw_opts)
            .await
            .map_err(|e| {
                CoreError::Internal(format!(
                    "Falha ao conectar ao SQLite para snapshot pré-atualização: {e}"
                ))
            })?;

        let tmp_str = tmp_snapshot.to_string_lossy().replace('\'', "''");
        let vacuum_sql = format!("VACUUM INTO '{tmp_str}'");
        let vacuum_res = sqlx::query(&vacuum_sql).execute(&rw_pool).await;
        rw_pool.close().await;

        vacuum_res.map_err(|e| {
            let _ = fs::remove_file(&tmp_snapshot);
            CoreError::Internal(format!(
                "Falha ao executar VACUUM INTO no snapshot pré-atualização: {e}"
            ))
        })?;

        // Valida integridade física do snapshot gerado via PRAGMA integrity_check
        let snap_opts = SqliteConnectOptions::new()
            .filename(&tmp_snapshot)
            .read_only(true);
        let check_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(snap_opts)
            .await
            .map_err(|e| {
                let _ = fs::remove_file(&tmp_snapshot);
                CoreError::Internal(format!(
                    "Falha ao conectar ao snapshot para integrity_check: {e}"
                ))
            })?;

        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&check_pool)
            .await
            .map_err(|e| {
                let _ = fs::remove_file(&tmp_snapshot);
                CoreError::Internal(format!(
                    "Falha ao rodar PRAGMA integrity_check no snapshot: {e}"
                ))
            })?;
        check_pool.close().await;

        if integrity != "ok" {
            let _ = fs::remove_file(&tmp_snapshot);
            return Err(CoreError::Validation(format!(
                "Integridade física do snapshot corrompida: {integrity}"
            )));
        }

        // Renomeia o temporário para o nome final
        fs::rename(&tmp_snapshot, &snapshot_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_snapshot);
            CoreError::Internal(format!("Falha ao finalizar snapshot pré-atualização: {e}"))
        })?;

        tracing::info!(
            snapshot = %snapshot_path.display(),
            "Snapshot pré-atualização criado e validado com sucesso."
        );

        // Executa retenção própria de snapshots (mantém os 3 mais recentes)
        Self::clean_old_snapshots(&snapshots_dir, 3)?;

        Ok(Some(snapshot_path))
    }

    /// Limpa snapshots antigos mantendo apenas os `keep_count` mais recentes.
    pub fn clean_old_snapshots(snapshots_dir: &Path, keep_count: usize) -> Result<()> {
        if !snapshots_dir.exists() {
            return Ok(());
        }
        let read_dir = match fs::read_dir(snapshots_dir) {
            Ok(rd) => rd,
            Err(_) => return Ok(()),
        };

        let mut snapshots = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".tmp.db") {
                    let _ = fs::remove_file(&path);
                } else if file_name.starts_with("pre-update_") && file_name.ends_with(".bak.db") {
                    snapshots.push(path);
                }
            }
        }

        // Ordena em ordem decrescente (mais recentes primeiro)
        snapshots.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        if snapshots.len() > keep_count {
            for old_snapshot in &snapshots[keep_count..] {
                tracing::info!(
                    snapshot = %old_snapshot.display(),
                    "Removendo snapshot pré-atualização antigo de acordo com a política de retenção."
                );
                let _ = fs::remove_file(old_snapshot);
            }
        }
        Ok(())
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

            // Grava arquivo de mídia de teste no padrão canônico media/<tenant>/<prefix2>/<sha256>
            let img_bytes = b"\x89PNG\r\n\x1a\nfake_image_bytes";
            let mut hasher = Sha256::new();
            hasher.update(img_bytes);
            let sha = format!("{:x}", hasher.finalize());
            let sample_dir = media_dir.join("default").join(&sha[..2]);
            fs::create_dir_all(&sample_dir).unwrap();
            let sample_media = sample_dir.join(&sha);
            fs::write(&sample_media, img_bytes).unwrap();

            // Inicializa banco SQLite com tabela de teste
            let conn_opts = SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(conn_opts)
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

        #[allow(dead_code)]
        async fn pool_ro(&self) -> SqlitePool {
            let opts = SqliteConnectOptions::new()
                .filename(&self.db_path)
                .read_only(true);
            SqlitePoolOptions::new().connect_with(opts).await.unwrap()
        }

        #[allow(dead_code)]
        async fn pool_rw(&self) -> SqlitePool {
            let opts = SqliteConnectOptions::new()
                .filename(&self.db_path)
                .read_only(false);
            SqlitePoolOptions::new().connect_with(opts).await.unwrap()
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
            .await
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
    async fn test_retention_execution_never_deletes_most_recent_when_limits_are_zero() {
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
            .execute_retention(&env.backup_dest, 0, 0, 0)
            .expect("Execução da retenção");

        assert!(
            backup_path.exists(),
            "O backup recém-criado deve continuar existindo mesmo com retenção zerada"
        );
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

        let res = service.execute_pending_restore_at_startup(10).await;
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

    #[tokio::test]
    async fn test_backup_fails_when_config_json_missing() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // Remove config.json
        let _ = fs::remove_file(&env.config_path);

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let res = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await;
        assert!(
            res.is_err(),
            "Backup deve falhar quando config.json está ausente"
        );
        let err_str = res.err().unwrap().to_string();
        assert!(
            err_str.contains("config.json") || err_str.contains("configuração"),
            "Mensagem de erro deve identificar config.json: {err_str}"
        );

        // Garante que nenhum arquivo de backup final foi criado na pasta de destino
        let files = fs::read_dir(&env.backup_dest).unwrap().count();
        assert_eq!(
            files, 0,
            "Nenhum arquivo final deve restar na pasta de destino após falha"
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn test_backup_fails_when_media_file_unreadable() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let img_bytes = b"sample locked media content";
        let mut hasher = Sha256::new();
        hasher.update(img_bytes);
        let sha = format!("{:x}", hasher.finalize());
        let sample_dir = env.media_dir.join("default").join(&sha[..2]);
        fs::create_dir_all(&sample_dir).unwrap();
        let sample_media = sample_dir.join(&sha);
        fs::write(&sample_media, img_bytes).unwrap();

        // Trava o arquivo para leitura
        #[cfg(windows)]
        use std::os::windows::fs::OpenOptionsExt;
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true).write(true);
        #[cfg(windows)]
        opts.share_mode(0); // Bloqueia qualquer leitura concorrente no Windows
        let _exclusive_file = opts.open(&sample_media).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&sample_media).unwrap().permissions();
            perms.set_mode(0o000); // Remove todas as permissões no Unix/Linux
            fs::set_permissions(&sample_media, perms).unwrap();
        }

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let res = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await;
        assert!(
            res.is_err(),
            "Backup deve falhar se arquivo de mídia estiver ilegível"
        );

        let count = fs::read_dir(&env.backup_dest).unwrap().count();
        assert_eq!(
            count, 0,
            "Nenhum arquivo final deve ser criado em caso de erro de leitura de mídia"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&sample_media).unwrap().permissions();
            perms.set_mode(0o644);
            let _ = fs::set_permissions(&sample_media, perms);
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn test_backup_fails_when_media_dir_unreadable() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;

        // Aponta media_dir para um arquivo comum (não diretório), forçando erro de listagem
        let file_as_dir = env.root.join("fake_dir_file");
        fs::write(&file_as_dir, b"not a dir").unwrap();

        let service = BackupFsService::new(
            env.app_data_dir.clone(),
            env.db_path.clone(),
            file_as_dir,
            env.config_path.clone(),
        );

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let res = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await;
        assert!(
            res.is_err(),
            "Backup deve falhar quando diretório de mídia for inválido/ilegível"
        );

        let count = fs::read_dir(&env.backup_dest).unwrap().count();
        assert_eq!(count, 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn test_backup_fails_when_destination_unwritable() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // Aponta a pasta de destino para um arquivo existente
        let file_as_dest = env.root.join("dest_file_blocking");
        fs::write(&file_as_dest, b"blocker").unwrap();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let res = service
            .run_backup(&file_as_dest, &pool, "0.10.0-fase2n")
            .await;
        assert!(
            res.is_err(),
            "Backup deve falhar quando destino for inacessível ou inválido"
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn test_pre_update_snapshot_skipped_when_no_migration_pending() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // Simula banco com migração 10 já aplicada
        {
            let conn_rw = format!("sqlite://{}?mode=rw", env.db_path.display());
            let p = SqlitePoolOptions::new().connect(&conn_rw).await.unwrap();
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS _sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT, installed_on TIMESTAMP, success BOOLEAN, checksum BLOB, execution_time BIGINT);
                 INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (10, 'auth_epoch', 1, X'', 10);",
            )
            .execute(&p)
            .await
            .unwrap();
            p.close().await;
        }

        let snapshot = service
            .create_pre_update_snapshot_if_needed(&env.db_path, 10)
            .await
            .expect("Execução da verificação de snapshot");

        assert!(
            snapshot.is_none(),
            "Nenhum snapshot deve ser criado quando a versão máxima da migração já estiver aplicada"
        );

        let snapshots_dir = env.app_data_dir.join("snapshots");
        if snapshots_dir.exists() {
            let files = fs::read_dir(&snapshots_dir).unwrap().count();
            assert_eq!(files, 0, "Nenhum arquivo de snapshot deve ser criado");
        }
    }

    #[tokio::test]
    async fn test_pre_update_snapshot_created_and_verified_when_migration_pending() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // Simula banco com migração pendente (versão 8 aplicada, max atual 10)
        {
            let conn_rw = format!("sqlite://{}?mode=rw", env.db_path.display());
            let p = SqlitePoolOptions::new().connect(&conn_rw).await.unwrap();
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS _sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT, installed_on TIMESTAMP, success BOOLEAN, checksum BLOB, execution_time BIGINT);
                 INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (8, 'venues', 1, X'', 10);",
            )
            .execute(&p)
            .await
            .unwrap();
            p.close().await;
        }

        let snapshot = service
            .create_pre_update_snapshot_if_needed(&env.db_path, 10)
            .await
            .expect("Execução do snapshot pré-atualização");

        assert!(
            snapshot.is_some(),
            "Snapshot pré-atualização deve ser criado quando houver migração pendente"
        );

        let snapshot_path = snapshot.unwrap();
        assert!(snapshot_path.exists());
        assert!(snapshot_path.to_string_lossy().ends_with(".bak.db"));

        // Valida que o snapshot gerado passa no PRAGMA integrity_check
        let check_conn = format!("sqlite://{}?mode=ro", snapshot_path.display());
        let check_pool = SqlitePoolOptions::new().connect(&check_conn).await.unwrap();
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&check_pool)
            .await
            .unwrap();
        check_pool.close().await;
        assert_eq!(integrity, "ok", "Snapshot gerado deve ser 100% íntegro");
    }

    #[tokio::test]
    async fn test_pre_update_snapshots_retention_keeps_only_three_most_recent() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;

        let snapshots_dir = env.app_data_dir.join("snapshots");
        fs::create_dir_all(&snapshots_dir).unwrap();

        // Cria 5 snapshots fictícios ordenados no tempo e 1 temporário órfão
        let s1 = snapshots_dir.join("pre-update_2026-11-24-100001.bak.db");
        let s2 = snapshots_dir.join("pre-update_2026-11-24-100002.bak.db");
        let s3 = snapshots_dir.join("pre-update_2026-11-24-100003.bak.db");
        let s4 = snapshots_dir.join("pre-update_2026-11-24-100004.bak.db");
        let s5 = snapshots_dir.join("pre-update_2026-11-24-100005.bak.db");
        let tmp = snapshots_dir.join("pre-update_2026-11-24-100006.tmp.db");

        for f in &[&s1, &s2, &s3, &s4, &s5, &tmp] {
            fs::write(f, b"dummy db snapshot data").unwrap();
        }

        BackupFsService::clean_old_snapshots(&snapshots_dir, 3)
            .expect("Retenção de snapshots pré-atualização");

        // Os 3 mais recentes (s5, s4, s3) devem ser mantidos
        assert!(s5.exists(), "Snapshot mais recente (s5) deve existir");
        assert!(s4.exists(), "Segundo snapshot (s4) deve existir");
        assert!(s3.exists(), "Terceiro snapshot (s3) deve existir");

        // Os mais antigos (s1, s2) e o temporário órfão devem ter sido removidos
        assert!(!s1.exists(), "Snapshot antigo (s1) deve ser removido");
        assert!(!s2.exists(), "Snapshot antigo (s2) deve ser removido");
        assert!(!tmp.exists(), "Arquivo temporário órfão deve ser removido");
    }

    #[tokio::test]
    async fn test_restore_fails_when_zip_contains_undeclared_entry() {
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

        let modified_zip = env.backup_dest.join("undeclared_entry.eventrent-backup");
        {
            let src = File::open(&backup_path).unwrap();
            let mut archive = ZipArchive::new(src).unwrap();
            let dst = File::create(&modified_zip).unwrap();
            let mut writer = ZipWriter::new(dst);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            for i in 0..archive.len() {
                let mut e = archive.by_index(i).unwrap();
                let name = e.name().to_string();
                let mut d = Vec::new();
                e.read_to_end(&mut d).unwrap();
                writer.start_file(name, opt).unwrap();
                writer.write_all(&d).unwrap();
            }
            // Injeta arquivo não declarado
            writer.start_file("secret_payload.txt", opt).unwrap();
            writer.write_all(b"malicious injected entry").unwrap();
            writer.finish().unwrap();
        }

        let schedule_res = service.schedule_restore(&modified_zip, "usr_admin", 10);
        // O schedule pode passar ou falhar dependendo da auto-verificação; se passar, execute_pending_restore_at_startup DEVE falhar
        if schedule_res.is_ok() {
            let res = service.execute_pending_restore_at_startup(10).await;
            assert!(
                res.is_err(),
                "Restauração deve falhar categoricamente quando há entrada não declarada"
            );
            let err_str = res.err().unwrap().to_string();
            assert!(
                err_str.contains("não declarada no manifesto") || err_str.contains("inválido"),
                "Mensagem deve apontar entrada não declarada: {err_str}"
            );
        }
    }

    #[tokio::test]
    async fn test_restore_fails_when_media_path_not_canonical() {
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

        let non_canonical_zip = env.backup_dest.join("non_canonical.eventrent-backup");
        {
            let src = File::open(&backup_path).unwrap();
            let mut archive = ZipArchive::new(src).unwrap();
            let dst = File::create(&non_canonical_zip).unwrap();
            let mut writer = ZipWriter::new(dst);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            let bad_media_name = "media/not_canonical_file.png";
            let mut manifest: Option<BackupManifest> = None;

            for i in 0..archive.len() {
                let mut e = archive.by_index(i).unwrap();
                let name = e.name().to_string();
                let mut d = Vec::new();
                e.read_to_end(&mut d).unwrap();
                if name == "manifest.json" {
                    let mut mf: BackupManifest = serde_json::from_slice(&d).unwrap();
                    mf.files.push(BackupFileEntry {
                        path: bad_media_name.to_string(),
                        byte_size: 10,
                        sha256: "0000000000000000000000000000000000000000000000000000000000000000"
                            .to_string(),
                    });
                    manifest = Some(mf);
                } else {
                    writer.start_file(name, opt).unwrap();
                    writer.write_all(&d).unwrap();
                }
            }
            if let Some(mf) = manifest {
                let data = serde_json::to_vec_pretty(&mf).unwrap();
                writer.start_file("manifest.json", opt).unwrap();
                writer.write_all(&data).unwrap();
            }
            writer.start_file(bad_media_name, opt).unwrap();
            writer.write_all(b"1234567890").unwrap();
            writer.finish().unwrap();
        }

        let schedule_res = service.schedule_restore(&non_canonical_zip, "usr_admin", 10);
        if schedule_res.is_ok() {
            let res = service.execute_pending_restore_at_startup(10).await;
            assert!(
                res.is_err(),
                "Restauração deve falhar com caminho de mídia não canônico"
            );
        }
    }

    #[tokio::test]
    async fn test_restore_fails_and_rolls_back_when_file_sha_mismatch() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        let initial_db_bytes = fs::read(&env.db_path).unwrap();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();
        let (_manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .unwrap();
        pool.close().await;

        let sha_mismatch_zip = env.backup_dest.join("sha_mismatch.eventrent-backup");
        {
            let src = File::open(&backup_path).unwrap();
            let mut archive = ZipArchive::new(src).unwrap();
            let dst = File::create(&sha_mismatch_zip).unwrap();
            let mut writer = ZipWriter::new(dst);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            for i in 0..archive.len() {
                let mut e = archive.by_index(i).unwrap();
                let name = e.name().to_string();
                let mut d = Vec::new();
                e.read_to_end(&mut d).unwrap();
                if name.starts_with("media/") {
                    // Adultera os bytes de mídia para forçar mismatch de SHA
                    if let Some(first) = d.first_mut() {
                        *first ^= 0xFF;
                    }
                }
                writer.start_file(name, opt).unwrap();
                writer.write_all(&d).unwrap();
            }
            writer.finish().unwrap();
        }

        // Agenda marcador
        let mut hasher = Sha256::new();
        let bytes = fs::read(&sha_mismatch_zip).unwrap();
        hasher.update(&bytes);
        let marker = RestorePendingMarker {
            backup_file_path: sha_mismatch_zip.to_string_lossy().to_string(),
            backup_sha256: format!("{:x}", hasher.finalize()),
            scheduled_at: Utc::now().to_rfc3339(),
            scheduled_by_user_id: "usr_admin".to_string(),
        };
        fs::write(
            env.app_data_dir.join(RESTORE_PENDING_FILE),
            serde_json::to_string(&marker).unwrap(),
        )
        .unwrap();

        let res = service.execute_pending_restore_at_startup(10).await;
        assert!(
            res.is_err(),
            "Restauração deve falhar quando SHA-256 for divergente"
        );

        // Banco original deve ser preservado intacto pelo rollback
        let current_db_bytes = fs::read(&env.db_path).unwrap();
        assert_eq!(
            current_db_bytes, initial_db_bytes,
            "Rollback deve garantir que banco de dados continue intacto"
        );
    }

    #[tokio::test]
    async fn test_backup_ignores_non_canonical_media_and_restores_successfully() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // Insere arquivos não-canônicos e espúrios na pasta de mídia
        fs::write(
            env.media_dir.join("Thumbs.db"),
            b"fake windows thumbs cache",
        )
        .unwrap();
        fs::write(env.media_dir.join(".DS_Store"), b"fake macos ds_store").unwrap();
        fs::write(
            env.media_dir.join("backup_residue.restore_tmp"),
            b"leftover restore tmp",
        )
        .unwrap();
        fs::write(
            env.media_dir.join("transient.tmp.48912"),
            b"leftover upload tmp",
        )
        .unwrap();

        let tenant_folder = env.media_dir.join("tenant_test");
        fs::create_dir_all(&tenant_folder).unwrap();
        fs::write(
            tenant_folder.join("random_document.pdf"),
            b"not a canonical media",
        )
        .unwrap();

        // Arquivo que até tem 64 hex no nome, mas o SHA-256 real do conteúdo não bate
        let hex_folder = tenant_folder.join("aa");
        fs::create_dir_all(&hex_folder).unwrap();
        let forged_name = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        fs::write(hex_folder.join(forged_name), b"corrupted or forged content").unwrap();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();

        let (manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .expect("Backup deve suceder mesmo com arquivos espúrios na pasta de mídia");
        pool.close().await;

        // Deve ter ignorado os 6 arquivos não-canônicos
        assert_eq!(
            manifest.skipped_media_files, 6,
            "Deve contabilizar exatamente os 6 arquivos não-canônicos ignorados"
        );

        // O backup deve ser 100% íntegro e válido na auto-verificação e verificação sob demanda
        let verify_res = service
            .verify_backup_file(&backup_path, 10)
            .expect("Verificação deve rodar com sucesso");
        assert!(
            verify_res.valid,
            "Backup com mídia não-canônica ignorada deve ser considerado válido: {:?}",
            verify_res.error_message
        );

        // Restauração do backup deve funcionar sem falhar por causa dos arquivos espúrios
        service
            .schedule_restore(&backup_path, "usr_admin", 10)
            .expect("Agendamento de restauração deve ser aceito");

        let restore_res = service.execute_pending_restore_at_startup(10).await;
        assert!(
            restore_res.is_ok(),
            "Restauração deve concluir com sucesso sem ser bloqueada por arquivos ignorados: {:?}",
            restore_res.err()
        );
    }

    #[tokio::test]
    async fn test_restore_rollback_preserves_preexisting_media_when_restoration_fails() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // Identifica o arquivo de mídia preexistente criado no setup
        let sample_media_files: Vec<PathBuf> = fs::read_dir(env.media_dir.join("default"))
            .unwrap()
            .filter_map(|e| e.ok())
            .flat_map(|sub| {
                fs::read_dir(sub.path())
                    .unwrap()
                    .filter_map(|e| e.ok().map(|ent| ent.path()))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            !sample_media_files.is_empty(),
            "Mídia preexistente deve existir"
        );
        let preexisting_media_path = sample_media_files[0].clone();
        let preexisting_bytes = fs::read(&preexisting_media_path).unwrap();

        let conn_str = format!("sqlite://{}?mode=ro", env.db_path.display());
        let pool = SqlitePoolOptions::new().connect(&conn_str).await.unwrap();
        let (_manifest, backup_path) = service
            .run_backup(&env.backup_dest, &pool, "0.10.0-fase2n")
            .await
            .expect("Backup deve ser criado");
        pool.close().await;

        // Cria uma cópia adulterada do zip onde um arquivo extra de mídia falhará na conferência de SHA
        let tampered_zip = env.backup_dest.join("tampered_rollback.eventrent-backup");
        {
            let src = File::open(&backup_path).unwrap();
            let mut archive = ZipArchive::new(src).unwrap();
            let dst = File::create(&tampered_zip).unwrap();
            let mut writer = ZipWriter::new(dst);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            let bad_media_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
            let bad_media_entry = format!("media/default/bb/{bad_media_hash}");

            for i in 0..archive.len() {
                let mut e = archive.by_index(i).unwrap();
                let name = e.name().to_string();
                let mut d = Vec::new();
                e.read_to_end(&mut d).unwrap();
                if name == "manifest.json" {
                    let mut mf: BackupManifest = serde_json::from_slice(&d).unwrap();
                    // Adiciona entrada fraudulenta ao manifesto
                    mf.files.push(BackupFileEntry {
                        path: bad_media_entry.clone(),
                        byte_size: 15,
                        sha256: bad_media_hash.to_string(),
                    });
                    d = serde_json::to_vec_pretty(&mf).unwrap();
                }
                writer.start_file(name, opt).unwrap();
                writer.write_all(&d).unwrap();
            }

            // Grava o arquivo com bytes divergentes do sha declarado
            writer.start_file(bad_media_entry, opt).unwrap();
            writer.write_all(b"wrong sha bytes").unwrap();
            writer.finish().unwrap();
        }

        // Agenda e executa restauração que falhará na conferência do arquivo fraudulento
        let mut hasher = Sha256::new();
        let b = fs::read(&tampered_zip).unwrap();
        hasher.update(&b);
        let marker = RestorePendingMarker {
            backup_file_path: tampered_zip.to_string_lossy().to_string(),
            backup_sha256: format!("{:x}", hasher.finalize()),
            scheduled_at: Utc::now().to_rfc3339(),
            scheduled_by_user_id: "usr_admin".to_string(),
        };
        fs::write(
            env.app_data_dir.join(RESTORE_PENDING_FILE),
            serde_json::to_string(&marker).unwrap(),
        )
        .unwrap();

        let res = service.execute_pending_restore_at_startup(10).await;
        assert!(
            res.is_err(),
            "Restauração deve falhar devido à divergência de SHA no arquivo corrompido"
        );

        // Ponto Crítico da Tarefa 0.2:
        // A mídia preexistente DEVE continuar existindo e intacta, NÃO pode ter sido deletada pelo rollback!
        assert!(
            preexisting_media_path.exists(),
            "Mídia preexistente não deve ser deletada no rollback de restauração"
        );
        let current_bytes = fs::read(&preexisting_media_path).unwrap();
        assert_eq!(
            current_bytes, preexisting_bytes,
            "Conteúdo da mídia preexistente deve permanecer rigorosamente inalterado"
        );
    }

    #[tokio::test]
    async fn test_backup_and_restore_with_special_characters_in_path() {
        let _test_guard = TEST_LOCK.lock().await;

        let base_tmp =
            std::env::temp_dir().join(format!("eventrent_special_test_{}", uuid::Uuid::now_v7()));
        let root = base_tmp.join("Área #1 100% Espaço & Acentuação");
        let app_data_dir = root.join("AppData Local");
        let db_path = app_data_dir.join("eventrent_banco #1.db");
        let config_path = app_data_dir.join("configuração & tema.json");
        let media_dir = app_data_dir.join("mídias #100%");
        let backup_dest = root.join("Destino de Backups 100%");

        fs::create_dir_all(&app_data_dir).unwrap();
        fs::create_dir_all(&media_dir).unwrap();
        fs::create_dir_all(&backup_dest).unwrap();

        fs::write(&config_path, r#"{"theme": "system"}"#).unwrap();

        let img_bytes = b"special_path_media_bytes";
        let mut hasher = Sha256::new();
        hasher.update(img_bytes);
        let sha = format!("{:x}", hasher.finalize());
        let sample_dir = media_dir.join("default").join(&sha[..2]);
        fs::create_dir_all(&sample_dir).unwrap();
        let sample_media = sample_dir.join(&sha);
        fs::write(&sample_media, img_bytes).unwrap();

        let conn_opts = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(conn_opts)
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE test_special (id INTEGER PRIMARY KEY, valor TEXT);
             INSERT INTO test_special (id, valor) VALUES (1, 'acentuação e % #');",
        )
        .execute(&pool)
        .await
        .unwrap();

        let service = BackupFsService::new(
            app_data_dir.clone(),
            db_path.clone(),
            media_dir.clone(),
            config_path.clone(),
        );

        let (manifest, backup_path) = service
            .run_backup(&backup_dest, &pool, "0.10.0-fase2n")
            .await
            .expect("Backup deve suceder mesmo com caminhos contendo acentos, %, # e espaços");
        pool.close().await;

        assert_eq!(manifest.files.len(), 3); // db, config, media

        let verify = service
            .verify_backup_file(&backup_path, 10)
            .expect("Verificação sob demanda");
        assert!(
            verify.valid,
            "Backup em pasta com caracteres especiais deve ser válido: {:?}",
            verify.error_message
        );

        // Altera o banco para testar restauração
        let rw_opts = SqliteConnectOptions::new()
            .filename(&db_path)
            .read_only(false);
        let pool_rw = SqlitePoolOptions::new()
            .connect_with(rw_opts)
            .await
            .unwrap();
        sqlx::query("DELETE FROM test_special WHERE id = 1")
            .execute(&pool_rw)
            .await
            .unwrap();
        pool_rw.close().await;

        service
            .schedule_restore(&backup_path, "usr_special", 10)
            .expect("Agendamento de restauração");

        let report = service
            .execute_pending_restore_at_startup(10)
            .await
            .expect("Execução da restauração a frio");
        assert!(report.is_some(), "Relatório de restauração gerado");

        // Confere restauração dos dados via SqliteConnectOptions::filename
        let ro_opts = SqliteConnectOptions::new()
            .filename(&db_path)
            .read_only(true);
        let pool_check = SqlitePoolOptions::new()
            .connect_with(ro_opts)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM test_special WHERE id = 1")
            .fetch_one(&pool_check)
            .await
            .unwrap();
        pool_check.close().await;
        assert_eq!(
            count, 1,
            "Dados originais devem ter sido restaurados com sucesso"
        );

        let _ = fs::remove_dir_all(&base_tmp);
    }
}
