use async_trait::async_trait;
use eventrent_core::{
    build_media_storage_key, compute_sha256_hex, validate_sha256_hex,
    validate_tenant_storage_segment, CoreError, MediaStore, Result,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Implementação de `MediaStore` sobre o sistema de arquivos local (ADR-0013).
///
/// Layout de diretório:
/// `<root_dir>/<tenant_id>/<2_primeiros_hex>/<sha256>`
///
/// Garantias de segurança e concorrência (R7):
/// - Caminho derivado exclusivamente de `tenant_id` validado e `sha256` (64 hex), nunca de nome enviado pelo usuário;
/// - Verificação explícita de contenção dentro de `root_dir`;
/// - Escrita atômica via arquivo temporário no mesmo subdiretório + `sync_all()` (`fsync`) + `rename`;
/// - Operações bloqueantes de disco executadas em `tokio::task::spawn_blocking` (I/O assíncrono para PDFs até 25 MB);
/// - Limpeza de arquivos `.tmp.*` antigos (> 24h) na inicialização;
/// - Reconciliação de coleta de lixo em disco com preservação de arquivos recentes (< 1h);
/// - Idempotente e com verificação de integridade SHA-256 na escrita e na leitura.
#[derive(Debug, Clone)]
pub struct FsMediaStore {
    root_dir: PathBuf,
}

impl FsMediaStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self> {
        let raw_root = root_dir.into();
        fs::create_dir_all(&raw_root).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao criar diretório raiz de mídia ({}): {e}",
                raw_root.display()
            ))
        })?;
        let canonical_root = fs::canonicalize(&raw_root).unwrap_or(raw_root);
        Ok(Self {
            root_dir: canonical_root,
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Resolve e valida o caminho físico do blob garantindo que permanece estritamente dentro de `self.root_dir`.
    pub fn resolve_blob_path(&self, tenant_id: &str, sha256: &str) -> Result<(String, PathBuf)> {
        let clean_tenant = validate_tenant_storage_segment(tenant_id)?;
        let clean_sha = validate_sha256_hex(sha256)?;
        let storage_key = build_media_storage_key(&clean_tenant, &clean_sha)?;
        let prefix = &clean_sha[..2];

        let dir_path = self.root_dir.join(&clean_tenant).join(prefix);
        let file_path = dir_path.join(&clean_sha);

        if !file_path.starts_with(&self.root_dir) {
            return Err(CoreError::Validation(
                "Caminho de armazenamento de mídia fora do diretório permitido.".to_string(),
            ));
        }

        Ok((storage_key, file_path))
    }

    /// Limpeza ao iniciar (R7.2): varredura em `media_dir` removendo arquivos temporários `*.tmp.*` com mais de `max_age`.
    pub async fn cleanup_stale_temp_writes(&self, max_age: Duration) -> Result<usize> {
        let root = self.root_dir.clone();
        tokio::task::spawn_blocking(move || {
            let mut removed = 0usize;
            let now = SystemTime::now();

            fn scan_and_clean(dir: &Path, max_age: Duration, now: SystemTime, removed: &mut usize) {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Ok(ft) = entry.file_type() {
                            if ft.is_dir() {
                                scan_and_clean(&path, max_age, now, removed);
                            } else if ft.is_file() {
                                let name = entry.file_name();
                                let name_str = name.to_string_lossy();
                                if name_str.contains(".tmp.") {
                                    if let Ok(meta) = entry.metadata() {
                                        if let Ok(modified) = meta.modified() {
                                            if let Ok(elapsed) = now.duration_since(modified) {
                                                if elapsed >= max_age
                                                    && fs::remove_file(&path).is_ok()
                                                {
                                                    *removed += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            scan_and_clean(&root, max_age, now, &mut removed);
            Ok(removed)
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao executar limpeza de temporários em spawn_blocking: {e}"
            ))
        })?
    }
}

#[async_trait]
impl MediaStore for FsMediaStore {
    async fn put_blob(&self, tenant_id: &str, sha256: &str, bytes: &[u8]) -> Result<String> {
        if bytes.is_empty() {
            return Err(CoreError::Validation(
                "Arquivo de mídia não pode ser vazio.".to_string(),
            ));
        }

        let clean_sha = validate_sha256_hex(sha256)?;
        let computed = compute_sha256_hex(bytes);
        if computed != clean_sha {
            return Err(CoreError::Validation(
                "Integridade de mídia inválida: hash SHA-256 não confere com o conteúdo enviado."
                    .to_string(),
            ));
        }

        let (storage_key, target_path) = self.resolve_blob_path(tenant_id, &clean_sha)?;
        let root_dir = self.root_dir.clone();
        let bytes_vec = bytes.to_vec();

        tokio::task::spawn_blocking(move || {
            let parent_dir = target_path.parent().ok_or_else(|| {
                CoreError::Internal("Diretório pai de mídia inválido.".to_string())
            })?;

            fs::create_dir_all(parent_dir).map_err(|e| {
                CoreError::Internal(format!(
                    "Falha ao criar subdiretório de mídia ({}): {e}",
                    parent_dir.display()
                ))
            })?;

            let canonical_parent = fs::canonicalize(parent_dir).map_err(|e| {
                CoreError::Internal(format!("Falha ao validar diretório de mídia: {e}"))
            })?;
            if !canonical_parent.starts_with(&root_dir) {
                return Err(CoreError::Validation(
                    "Violação de fronteira no diretório de mídia.".to_string(),
                ));
            }

            // Idempotência: se o arquivo já existe com o mesmo tamanho e hash válido, renova a data de modificação
            // (mtime) para SystemTime::now() para que a coleta de lixo (GC) não o trate como órfão obsoleto (B3) e retorna a chave.
            if target_path.exists() {
                if let Ok(existing_bytes) = fs::read(&target_path) {
                    if existing_bytes.len() == bytes_vec.len()
                        && compute_sha256_hex(&existing_bytes) == clean_sha
                    {
                        if let Ok(file) = fs::OpenOptions::new().write(true).open(&target_path) {
                            let _ = file.set_modified(SystemTime::now());
                        }
                        return Ok(storage_key);
                    }
                }
            }

            let tmp_name = format!("{}.tmp.{}", clean_sha, Uuid::now_v7());
            let tmp_path = parent_dir.join(tmp_name);

            let write_result = (|| -> std::io::Result<()> {
                let mut file = File::create(&tmp_path)?;
                file.write_all(&bytes_vec)?;
                file.sync_all()?;
                drop(file);
                fs::rename(&tmp_path, &target_path)?;
                Ok(())
            })();

            if let Err(e) = write_result {
                let _ = fs::remove_file(&tmp_path);
                return Err(CoreError::Internal(format!(
                    "Falha na gravação atômica do blob de mídia: {e}"
                )));
            }

            Ok(storage_key)
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!("Falha ao executar put_blob em spawn_blocking: {e}"))
        })?
    }

    async fn get_blob(&self, tenant_id: &str, sha256: &str) -> Result<Vec<u8>> {
        let clean_sha = validate_sha256_hex(sha256)?;
        let (storage_key, target_path) = self.resolve_blob_path(tenant_id, &clean_sha)?;

        tokio::task::spawn_blocking(move || {
            if !target_path.exists() {
                return Err(CoreError::NotFound(format!(
                    "Arquivo de mídia '{}' não encontrado no disco.",
                    storage_key
                )));
            }

            let bytes = fs::read(&target_path).map_err(|e| {
                CoreError::Internal(format!(
                    "Falha ao ler arquivo de mídia '{}': {e}",
                    storage_key
                ))
            })?;

            if compute_sha256_hex(&bytes) != clean_sha {
                return Err(CoreError::Internal(format!(
                    "Corrupção detectada no arquivo de mídia '{}': hash SHA-256 divergente.",
                    storage_key
                )));
            }

            Ok(bytes)
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!("Falha ao executar get_blob em spawn_blocking: {e}"))
        })?
    }

    async fn delete_blob(&self, tenant_id: &str, sha256: &str) -> Result<()> {
        let (_storage_key, target_path) = self.resolve_blob_path(tenant_id, sha256)?;
        tokio::task::spawn_blocking(move || {
            if target_path.exists() {
                fs::remove_file(&target_path).map_err(|e| {
                    CoreError::Internal(format!("Falha ao remover arquivo de mídia do disco: {e}"))
                })?;
            }
            Ok(())
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao executar delete_blob em spawn_blocking: {e}"
            ))
        })?
    }

    async fn exists_blob(&self, tenant_id: &str, sha256: &str) -> Result<bool> {
        let (_storage_key, target_path) = self.resolve_blob_path(tenant_id, sha256)?;
        tokio::task::spawn_blocking(move || Ok(target_path.exists()))
            .await
            .map_err(|e| {
                CoreError::Internal(format!(
                    "Falha ao executar exists_blob em spawn_blocking: {e}"
                ))
            })?
    }

    /// Conciliação na coleta de lixo (R7.3):
    /// Varre os arquivos físicos em disco no diretório do tenant e remove aqueles que não possuem
    /// linha correspondente na tabela `media_blob` HÁ MAIS DE `min_age` (evita apagar arquivos de
    /// upload concorrente que ainda não fecharam a transação).
    async fn reconcile_unreferenced_blobs(
        &self,
        tenant_id: &str,
        valid_shas: &std::collections::HashSet<String>,
        min_age: Duration,
    ) -> Result<usize> {
        let clean_tenant = validate_tenant_storage_segment(tenant_id)?;
        let tenant_root = self.root_dir.join(&clean_tenant);
        if !tenant_root.exists() {
            return Ok(0);
        }

        let valid_shas_clone = valid_shas.clone();
        let root_dir_clone = self.root_dir.clone();

        tokio::task::spawn_blocking(move || {
            let mut removed = 0usize;
            let now = SystemTime::now();

            if let Ok(prefix_entries) = fs::read_dir(&tenant_root) {
                for prefix_entry in prefix_entries.flatten() {
                    let prefix_path = prefix_entry.path();
                    if let Ok(ft) = prefix_entry.file_type() {
                        if ft.is_dir() {
                            if let Ok(file_entries) = fs::read_dir(&prefix_path) {
                                for file_entry in file_entries.flatten() {
                                    let file_path = file_entry.path();
                                    if !file_path.starts_with(&root_dir_clone) {
                                        continue;
                                    }
                                    if let Ok(file_ft) = file_entry.file_type() {
                                        if file_ft.is_file() {
                                            let file_name = file_entry.file_name();
                                            let name_str = file_name.to_string_lossy();
                                            // Apenas arquivos cujo nome seja um hash SHA-256 de 64 caracteres hexadecimais
                                            if name_str.len() == 64
                                                && validate_sha256_hex(&name_str).is_ok()
                                            {
                                                if let Ok(meta) = file_entry.metadata() {
                                                    if let Ok(modified) = meta.modified() {
                                                        if let Ok(elapsed) =
                                                            now.duration_since(modified)
                                                        {
                                                            // R7.3: Preserva arquivos com idade menor que min_age (ex.: 1h)
                                                            if elapsed >= min_age
                                                                && !valid_shas_clone
                                                                    .contains(name_str.as_ref())
                                                                && fs::remove_file(&file_path)
                                                                    .is_ok()
                                                            {
                                                                removed += 1;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(removed)
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao executar reconcile_unreferenced_blobs em spawn_blocking: {e}"
            ))
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_media_dir() -> PathBuf {
        std::env::temp_dir().join(format!("eventrent_media_fs_test_{}", Uuid::now_v7()))
    }

    #[tokio::test]
    async fn test_fs_media_store_atomic_write_dedup_and_missing_file() {
        let dir = temp_media_dir();
        let store = FsMediaStore::new(&dir).unwrap();

        let payload = b"conteudo de teste para midia CAS";
        let sha = compute_sha256_hex(payload);

        // 1. Gravação atômica e leitura
        let key1 = store.put_blob("tenant-1", &sha, payload).await.unwrap();
        assert_eq!(key1, format!("tenant-1/{}/{}", &sha[..2], sha));
        assert!(store.exists_blob("tenant-1", &sha).await.unwrap());

        // Caminho físico sempre dentro de root_dir
        let (_, resolved) = store.resolve_blob_path("tenant-1", &sha).unwrap();
        assert!(resolved.starts_with(store.root_dir()));
        assert!(resolved.exists());

        // 2. Idempotência / deduplicação e renovação de mtime (B3)
        // Envelhece o blob preexistente para 2 horas atrás
        let two_hours_ago = SystemTime::now() - Duration::from_secs(7200);
        let f = fs::OpenOptions::new().write(true).open(&resolved).unwrap();
        f.set_modified(two_hours_ago).unwrap();
        drop(f);
        let mtime_before = fs::metadata(&resolved).unwrap().modified().unwrap();
        assert!(mtime_before < SystemTime::now() - Duration::from_secs(3600));

        // Reenvio do mesmo blob: deve manter a mesma chave e renovar mtime para agora
        let key2 = store.put_blob("tenant-1", &sha, payload).await.unwrap();
        assert_eq!(key1, key2);

        let mtime_after = fs::metadata(&resolved).unwrap().modified().unwrap();
        assert!(
            mtime_after > SystemTime::now() - Duration::from_secs(60),
            "Reenvio de blob preexistente deve renovar mtime para evitar descarte por GC"
        );

        let read_back = store.get_blob("tenant-1", &sha).await.unwrap();
        assert_eq!(read_back, payload);

        // 3. Rejeita hash divergente do conteúdo
        let wrong_sha = "a".repeat(64);
        assert!(matches!(
            store.put_blob("tenant-1", &wrong_sha, payload).await,
            Err(CoreError::Validation(_))
        ));

        // 4. Rejeita tentativas de path traversal em tenant_id ou sha256
        assert!(matches!(
            store.resolve_blob_path("../etc", &sha),
            Err(CoreError::Validation(_))
        ));
        assert!(matches!(
            store.resolve_blob_path("tenant-1", "../passwd"),
            Err(CoreError::Validation(_))
        ));

        // 5. Arquivo ausente retorna erro claro NotFound
        let missing_sha = "b".repeat(64);
        match store.get_blob("tenant-1", &missing_sha).await {
            Err(CoreError::NotFound(msg)) => {
                assert!(msg.contains("não encontrado no disco"));
            }
            other => panic!("Esperava NotFound para arquivo ausente, obteve {other:?}"),
        }

        // 6. Exclusão física
        store.delete_blob("tenant-1", &sha).await.unwrap();
        assert!(!store.exists_blob("tenant-1", &sha).await.unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_fs_media_store_gc_reconciliation_and_temp_cleanup() {
        let dir = temp_media_dir();
        let store = FsMediaStore::new(&dir).unwrap();
        let tenant = "tenant-gc-test";

        let payload_old = b"conteudo antigo orfao";
        let sha_old = compute_sha256_hex(payload_old);
        store.put_blob(tenant, &sha_old, payload_old).await.unwrap();

        let payload_recent = b"conteudo recente concorrente";
        let sha_recent = compute_sha256_hex(payload_recent);
        store
            .put_blob(tenant, &sha_recent, payload_recent)
            .await
            .unwrap();

        let (_, old_file_path) = store.resolve_blob_path(tenant, &sha_old).unwrap();
        let (_, recent_file_path) = store.resolve_blob_path(tenant, &sha_recent).unwrap();

        // Envelhece artificialmente o arquivo old para 2 horas atrás (> 1h)
        let two_hours_ago = SystemTime::now() - Duration::from_secs(7200);
        let f_old = fs::OpenOptions::new()
            .write(true)
            .open(&old_file_path)
            .unwrap();
        f_old.set_modified(two_hours_ago).unwrap();
        drop(f_old);

        // O arquivo recent foi acabado de criar (idade < 1h)

        // Criamos também arquivos temporários .tmp.*
        let prefix_dir = old_file_path.parent().unwrap();
        let old_tmp_path = prefix_dir.join(format!("{}.tmp.old", sha_old));
        fs::write(&old_tmp_path, b"temp data old").unwrap();
        let f_tmp_old = fs::OpenOptions::new()
            .write(true)
            .open(&old_tmp_path)
            .unwrap();
        let twenty_five_hours_ago = SystemTime::now() - Duration::from_secs(25 * 3600);
        f_tmp_old.set_modified(twenty_five_hours_ago).unwrap();
        drop(f_tmp_old);

        let fresh_tmp_path = prefix_dir.join(format!("{}.tmp.fresh", sha_recent));
        fs::write(&fresh_tmp_path, b"temp data fresh").unwrap();

        // 1. Reconciliação do disco (R7.3 & R7.6):
        // Lista de SHAs válidos no banco está vazia (ambos são órfãos no banco)
        let valid_shas = std::collections::HashSet::new();
        let removed = store
            .reconcile_unreferenced_blobs(tenant, &valid_shas, Duration::from_secs(3600))
            .await
            .unwrap();

        // old_file (> 1h) DEVE ter sido removido
        assert_eq!(removed, 1);
        assert!(!old_file_path.exists());

        // recent_file (< 1h) DEVE ter sido preservado
        assert!(recent_file_path.exists());

        // 2. Limpeza de temporários ao iniciar (R7.2):
        let tmp_removed = store
            .cleanup_stale_temp_writes(Duration::from_secs(24 * 3600))
            .await
            .unwrap();
        assert_eq!(tmp_removed, 1);
        assert!(!old_tmp_path.exists());
        assert!(fresh_tmp_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
