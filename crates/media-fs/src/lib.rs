use async_trait::async_trait;
use eventrent_core::{
    build_media_storage_key, compute_sha256_hex, validate_sha256_hex,
    validate_tenant_storage_segment, CoreError, MediaStore, Result,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Implementação de `MediaStore` sobre o sistema de arquivos local (ADR-0013).
///
/// Layout de diretório:
/// `<root_dir>/<tenant_id>/<2_primeiros_hex>/<sha256>`
///
/// Garantias de segurança:
/// - Caminho derivado exclusivamente de `tenant_id` validado e `sha256` (64 hex), nunca de nome enviado pelo usuário;
/// - Verificação explícita de contenção dentro de `root_dir`;
/// - Escrita atômica via arquivo temporário no mesmo subdiretório + `sync_all()` (`fsync`) + `rename`;
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
        let parent_dir = target_path
            .parent()
            .ok_or_else(|| CoreError::Internal("Diretório pai de mídia inválido.".to_string()))?;

        fs::create_dir_all(parent_dir).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao criar subdiretório de mídia ({}): {e}",
                parent_dir.display()
            ))
        })?;

        let canonical_parent = fs::canonicalize(parent_dir).map_err(|e| {
            CoreError::Internal(format!("Falha ao validar diretório de mídia: {e}"))
        })?;
        if !canonical_parent.starts_with(&self.root_dir) {
            return Err(CoreError::Validation(
                "Violação de fronteira no diretório de mídia.".to_string(),
            ));
        }

        // Idempotência: se o arquivo já existe com o mesmo tamanho e hash válido, não reescreve
        if target_path.exists() {
            if let Ok(existing_bytes) = fs::read(&target_path) {
                if existing_bytes.len() == bytes.len()
                    && compute_sha256_hex(&existing_bytes) == clean_sha
                {
                    return Ok(storage_key);
                }
            }
        }

        let tmp_name = format!("{}.tmp.{}", clean_sha, Uuid::now_v7());
        let tmp_path = parent_dir.join(tmp_name);

        let write_result = (|| -> std::io::Result<()> {
            let mut file = File::create(&tmp_path)?;
            file.write_all(bytes)?;
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
    }

    async fn get_blob(&self, tenant_id: &str, sha256: &str) -> Result<Vec<u8>> {
        let clean_sha = validate_sha256_hex(sha256)?;
        let (storage_key, target_path) = self.resolve_blob_path(tenant_id, &clean_sha)?;

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
    }

    async fn delete_blob(&self, tenant_id: &str, sha256: &str) -> Result<()> {
        let (_storage_key, target_path) = self.resolve_blob_path(tenant_id, sha256)?;
        if target_path.exists() {
            fs::remove_file(&target_path).map_err(|e| {
                CoreError::Internal(format!("Falha ao remover arquivo de mídia do disco: {e}"))
            })?;
        }
        Ok(())
    }

    async fn exists_blob(&self, tenant_id: &str, sha256: &str) -> Result<bool> {
        let (_storage_key, target_path) = self.resolve_blob_path(tenant_id, sha256)?;
        Ok(target_path.exists())
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

        // 2. Idempotência / deduplicação
        let key2 = store.put_blob("tenant-1", &sha, payload).await.unwrap();
        assert_eq!(key1, key2);

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
}
