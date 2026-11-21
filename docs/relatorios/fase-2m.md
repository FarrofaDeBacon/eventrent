# Relatório da Fase 2M — Correções da 2A e Mídia de Modelos

**Branch:** `fase-2m-midia`
**Base:** `main` (`336df9f` / `v0.8.0-fase2a`)
**Status:** Concluída. `pnpm verify` 10/10 verde em todos os commits; `e2e/catalog-media.spec.ts` passando. Sem merge e sem tag.

---

## 1. Resumo

### Tarefa 0 — Correções da 2A (um commit por item)

| Item | Commit    | Resultado                                                                                                           |
| ---- | --------- | ------------------------------------------------------------------------------------------------------------------- |
| 0.1  | `7257798` | `data_type` travado quando há valores; opção `select` em uso não pode ser removida (`Conflict` com contagem/lista). |
| 0.2  | `506a37a` | `field_key` único na cadeia de ancestrais e descendentes, inclusive ao mover subárvore.                             |
| 0.3  | `a7ed4d6` | Renomear marca ou renomear/mover categoria recalcula `search_text` na mesma transação.                              |
| 0.4  | `f9cce8b` | Valores de atributo atualizados no lugar; 50 edições mantêm a contagem de linhas constante.                         |
| 0.5  | `d10d201` | `list_models`: padrão 50, teto 100; `%`, `_` e `\` escapados com `ESCAPE`.                                          |
| 0.6  | `ea262f8` | Marca/categoria inativa bloqueada em modelo novo ou em troca.                                                       |
| 0.7  | `75e4727` | Raízes sugeridas "DJ" e "Acessórios" (idempotentes).                                                                |
| 0.8  | `3bc3c1c` | Erro de SKU/código duplicado informa o nome do modelo existente.                                                    |

### Tarefa 1 — ADR-0013 (`e23f72f`)

[`docs/decisions/0013-midia-e-anexos.md`](../decisions/0013-midia-e-anexos.md).

### Tarefa 2 — Mídia

| Item | Commit    | Conteúdo                                                                                                     |
| ---- | --------- | ------------------------------------------------------------------------------------------------------------ |
| 2.1  | `24e2af4` | Migração `0009_media.sql`.                                                                                   |
| 2.2  | `5920984` | Trait `MediaStore` + `crates/media-fs` (`FsMediaStore`).                                                     |
| 2.3  | `99b1465` | Validações puras no core (magic bytes, dimensões, EXIF, PDF, limites, nome de arquivo).                      |
| 2.4  | `cee427c` | Store SQLite, serviços transacionais com auditoria, GC manual, comandos Tauri binários e temporários de PDF. |
| 2.5  | `421471f` | Cache LRU de Blob URLs e repositórios (Tauri/mock) com frame binário.                                        |
| 2.6  | `3e48a01` | Abas Fotos/Documentos, redimensionamento no navegador, miniatura na lista virtualizada.                      |
| 2.7  | `ce4a740` | `/catalog/import-photos` (SKU → código → EAN).                                                               |
| 2.8  | `0f64937` | Testes: contrato SQLite/serviços/GC, Vitest e Playwright.                                                    |
| 2.9  | `5c7862b` | Docs: modelo de dados, regras, segurança, roadmap.                                                           |

### Pontos de atenção (honestos)

- **Dependências:** nenhuma crate externa nova. O `src-tauri` passou a declarar `uuid` (já presente no workspace via `eventrent-core`) e a crate interna `eventrent-media-fs`. SHA-256 é implementação própria FIPS 180-4, testada contra o vetor "abc".
- **Colunas `updated_by`:** a migração `0009` segue os campos padrão do projeto (`created_by`, sem `updated_by`); os structs de domínio têm `updated_by`, mas o store não o persiste.
- **E2E:** roda no navegador com o repositório mock (como os E2E anteriores); o caminho binário Tauri real (`Request`/`Response`) é coberto por testes Rust do decodificador de frame, não por E2E no app nativo.
- **Redimensionamento:** o fallback sintético de WebP (quando `canvas.toBlob` não existe) só serve ao ambiente headless; no app real o canvas sempre está disponível.
- **Abrir/Salvar PDF** depende de `tauri-plugin-opener`/`tauri-plugin-dialog` e não foi exercitado por teste automatizado em janela nativa.

---

## 2. Migração `0009_media.sql` (cópia integral)

```sql
-- Migração 0009: Mídia e anexos (fotos e documentos de modelos de equipamento — Fase 2M / ADR-0013)
-- Regras:
-- - SQL portável (SQLite + PostgreSQL)
-- - Isolamento multi-tenant com chaves estrangeiras compostas (tenant_id, *_id)
-- - Índices únicos parciais com WHERE deleted_at IS NULL
-- - Nenhum binário armazenado no banco (endereçamento por conteúdo SHA-256 via MediaStore)
-- - Nenhum dado de negócio inserido na migração

CREATE TABLE media_blob (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    mime_type TEXT NOT NULL CHECK (mime_type IN ('image/webp', 'image/jpeg', 'image/png', 'application/pdf')),
    byte_size INTEGER NOT NULL CHECK (byte_size > 0),
    width INTEGER CHECK (width IS NULL OR width > 0),
    height INTEGER CHECK (height IS NULL OR height > 0),
    storage_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_media_blob_sha_mime_active
    ON media_blob (tenant_id, sha256, mime_type)
    WHERE deleted_at IS NULL;

CREATE TABLE media_asset (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('image', 'document')),
    blob_id TEXT NOT NULL,
    thumb_blob_id TEXT,
    title TEXT NOT NULL,
    doc_type TEXT CHECK (doc_type IS NULL OR doc_type IN ('manual', 'datasheet', 'certificate', 'other')),
    origin TEXT NOT NULL CHECK (origin IN ('manufacturer', 'own', 'website', 'other')),
    credit_note TEXT,
    language TEXT,
    version_note TEXT,
    original_filename TEXT NOT NULL,
    width INTEGER CHECK (width IS NULL OR width > 0),
    height INTEGER CHECK (height IS NULL OR height > 0),
    byte_size INTEGER NOT NULL CHECK (byte_size > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, blob_id) REFERENCES media_blob(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, thumb_blob_id) REFERENCES media_blob(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE INDEX idx_media_asset_tenant_blob
    ON media_asset (tenant_id, blob_id)
    WHERE deleted_at IS NULL;

CREATE TABLE equipment_model_media (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, model_id) REFERENCES equipment_model(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, asset_id) REFERENCES media_asset(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_equipment_model_media_link_active
    ON equipment_model_media (tenant_id, model_id, asset_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_equipment_model_media_primary_active
    ON equipment_model_media (tenant_id, model_id)
    WHERE is_primary = 1 AND deleted_at IS NULL;

CREATE INDEX idx_equipment_model_media_model
    ON equipment_model_media (tenant_id, model_id, sort_order)
    WHERE deleted_at IS NULL;
```

---

## 3. Trechos exatos do código

### Rejeição de EXIF no JPEG

`crates/core/src/domain/media.rs` (linhas 532–608)

```rust
fn inspect_jpeg_no_exif(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 4 || bytes[0..2] != [0xFF, 0xD8] {
        return Err(CoreError::Validation(
            "Arquivo JPEG truncado ou sem marcador SOI.".to_string(),
        ));
    }

    let mut pos = 2;
    let mut saw_frame_or_scan = false;

    while pos < bytes.len() {
        if bytes[pos] != 0xFF {
            return Err(CoreError::Validation(
                "Arquivo JPEG malformado: marcador de segmento inválido.".to_string(),
            ));
        }

        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo JPEG truncado após prefixo de marcador.".to_string(),
            ));
        }

        let marker = bytes[pos];
        pos += 1;

        if marker == 0xD9 {
            // EOI
            return Ok(());
        }
        if marker == 0xDA {
            // SOS (início dos dados comprimidos da imagem)
            saw_frame_or_scan = true;
            break;
        }
        if (0xD0..=0xD8).contains(&marker) || marker == 0x01 {
            continue;
        }

        if pos + 2 > bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo JPEG truncado ao ler tamanho do segmento.".to_string(),
            ));
        }
        let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo JPEG truncado: segmento excede o tamanho do arquivo.".to_string(),
            ));
        }

        let seg_data = &bytes[pos + 2..pos + seg_len];
        if marker == 0xE1 && (seg_data.starts_with(b"Exif\0\0") || seg_data.starts_with(b"Exif\0"))
        {
            return Err(CoreError::Validation(
                "Imagem JPEG contém metadados EXIF (segmento APP1 Exif), que são proibidos. Remova o EXIF antes de enviar.".to_string(),
            ));
        }

        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            saw_frame_or_scan = true;
        }

        pos += seg_len;
    }

    if !saw_frame_or_scan {
        return Err(CoreError::Validation(
            "Arquivo JPEG truncado: nenhum quadro de imagem encontrado.".to_string(),
        ));
    }

    Ok(())
}
```

### Validação comum de imagem (magic bytes, EXIF, dimensões)

`crates/core/src/domain/media.rs` (linhas 665–737)

```rust
fn validate_image_common(
    role_label: &str,
    declared_mime: &str,
    bytes: &[u8],
    max_bytes: usize,
    max_dimension: u32,
) -> Result<ValidatedImagePayload> {
    if bytes.is_empty() {
        return Err(CoreError::Validation(format!(
            "O arquivo de {role_label} está vazio."
        )));
    }
    if bytes.len() > max_bytes {
        return Err(CoreError::Validation(format!(
            "O arquivo de {role_label} excede o tamanho máximo permitido de {} KB (recebido: {} bytes).",
            max_bytes / 1024,
            bytes.len()
        )));
    }

    let parsed_mime = MediaMimeType::parse_str(declared_mime)?;
    if !parsed_mime.is_image() {
        return Err(CoreError::Validation(format!(
            "Tipo MIME inválido para {role_label}: '{}'. Esperado image/webp, image/jpeg ou image/png.",
            parsed_mime.as_str()
        )));
    }

    let magic_mime = detect_magic_mime(bytes).ok_or_else(|| {
        CoreError::Validation(format!(
            "Cabeçalho binário (magic bytes) não reconhecido ou truncado para {role_label}."
        ))
    })?;

    if magic_mime != parsed_mime {
        return Err(CoreError::Validation(format!(
            "O conteúdo binário ({}) não corresponde ao tipo MIME declarado ({}) em {role_label}.",
            magic_mime.as_str(),
            parsed_mime.as_str()
        )));
    }

    reject_image_exif(parsed_mime, bytes)?;

    let size = imagesize::blob_size(bytes).map_err(|_| {
        CoreError::Validation(format!(
            "Não foi possível ler as dimensões do cabeçalho da imagem ({role_label}); arquivo truncado ou corrompido."
        ))
    })?;

    let width = u32::try_from(size.width).unwrap_or(0);
    let height = u32::try_from(size.height).unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(CoreError::Validation(format!(
            "Dimensões inválidas ({width}x{height}) para {role_label}."
        )));
    }

    let max_side = width.max(height);
    if max_side > max_dimension {
        return Err(CoreError::Validation(format!(
            "A imagem de {role_label} ({width}x{height} px) excede a dimensão máxima permitida de {max_dimension} px no maior lado."
        )));
    }

    Ok(ValidatedImagePayload {
        mime_type: parsed_mime,
        sha256: compute_sha256_hex(bytes),
        byte_size: bytes.len() as i64,
        width: width as i32,
        height: height as i32,
    })
}
```

### Validação de PDF

`crates/core/src/domain/media.rs` (linhas 764–811)

```rust
/// Valida documento PDF (magic "%PDF-", coerente com application/pdf, até 25 MB).
pub fn validate_pdf_document(declared_mime: &str, bytes: &[u8]) -> Result<ValidatedPdfPayload> {
    if bytes.is_empty() {
        return Err(CoreError::Validation(
            "O arquivo PDF está vazio.".to_string(),
        ));
    }
    if bytes.len() > MAX_PDF_DOCUMENT_BYTES {
        return Err(CoreError::Validation(format!(
            "O documento PDF excede o limite máximo de 25 MB (recebido: {} bytes).",
            bytes.len()
        )));
    }

    let parsed_mime = MediaMimeType::parse_str(declared_mime)?;
    if !parsed_mime.is_pdf() {
        return Err(CoreError::Validation(format!(
            "Tipo MIME inválido para documento: '{}'. Apenas 'application/pdf' é aceito.",
            parsed_mime.as_str()
        )));
    }

    let magic_mime = detect_magic_mime(bytes).ok_or_else(|| {
        CoreError::Validation(
            "Cabeçalho binário inválido: o arquivo não inicia com a assinatura '%PDF-'."
                .to_string(),
        )
    })?;

    if magic_mime != MediaMimeType::ApplicationPdf {
        return Err(CoreError::Validation(format!(
            "O conteúdo do arquivo ({}) não corresponde ao MIME 'application/pdf'.",
            magic_mime.as_str()
        )));
    }

    if bytes.len() < 8 {
        return Err(CoreError::Validation(
            "Arquivo PDF truncado ou incompleto.".to_string(),
        ));
    }

    Ok(ValidatedPdfPayload {
        mime_type: MediaMimeType::ApplicationPdf,
        sha256: compute_sha256_hex(bytes),
        byte_size: bytes.len() as i64,
    })
}
```

### Saneamento do nome de arquivo

`crates/core/src/domain/media.rs` (linhas 894–945)

```rust
/// Saneia e valida o nome original do arquivo (sem caminho, sem caracteres de controle, até 150 caracteres).
pub fn sanitize_media_filename(raw: &str) -> Result<String> {
    if raw.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(
            "O nome do arquivo não pode conter caracteres de controle.".to_string(),
        ));
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "O nome do arquivo não pode ser vazio.".to_string(),
        ));
    }

    if trimmed.contains("..") {
        return Err(CoreError::Validation(
            "O nome do arquivo contém sequência de caminho inválida ('..').".to_string(),
        ));
    }

    // Remove qualquer prefixo de diretório (Windows ou POSIX) para garantir apenas o nome base.
    let base = trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('.');

    if base.is_empty() {
        return Err(CoreError::Validation(
            "Nome de arquivo inválido após remoção de caminho.".to_string(),
        ));
    }

    let sanitized: String = base
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect();

    let char_count = sanitized.chars().count();
    if char_count == 0 || char_count > MAX_MEDIA_FILENAME_CHARS {
        return Err(CoreError::Validation(format!(
            "O nome do arquivo deve ter entre 1 e {MAX_MEDIA_FILENAME_CHARS} caracteres."
        )));
    }

    Ok(sanitized)
}
```

### Trait MediaStore

`crates/core/src/store/mod.rs` (linhas 441–448)

```rust
/// Trait de armazenamento físico de blobs de mídia (CAS por SHA-256 e tenant_id — ADR-0013).
#[async_trait]
pub trait MediaStore: Send + Sync {
    async fn put_blob(&self, tenant_id: &str, sha256: &str, bytes: &[u8]) -> Result<String>;
    async fn get_blob(&self, tenant_id: &str, sha256: &str) -> Result<Vec<u8>>;
    async fn delete_blob(&self, tenant_id: &str, sha256: &str) -> Result<()>;
    async fn exists_blob(&self, tenant_id: &str, sha256: &str) -> Result<bool>;
}
```

### Gravação atômica no FsMediaStore

`crates/media-fs/src/lib.rs` (linhas 67–135)

```rust
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
```

### Serviço remove_model_media (promoção da próxima principal)

`crates/core/src/services/media.rs` (linhas 781–853)

```rust
    pub async fn remove_model_media(
        &self,
        session: &SessionInfo,
        media_link_id: &str,
    ) -> Result<()> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let link_id = media_link_id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let link_id = link_id.clone();
            let now = now.clone();

            Box::pin(async move {
                let view = stores
                    .catalog()
                    .find_model_media_view_by_link_id(&session.tenant_id, &link_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Vínculo de mídia não encontrado.".to_string())
                    })?;

                stores
                    .catalog()
                    .soft_delete_model_media_link(
                        &session.tenant_id,
                        &link_id,
                        &now,
                        Some(&session.user_id),
                    )
                    .await?;

                // Se a foto removida era a principal, promove a próxima foto ativa por sort_order
                if view.kind == MediaKind::Image && view.is_primary {
                    let remaining = stores
                        .catalog()
                        .list_model_media_views(&session.tenant_id, &view.model_id)
                        .await?;
                    if let Some(next_photo) = remaining.iter().find(|v| v.kind == MediaKind::Image)
                    {
                        if let Some(mut next_link) = stores
                            .catalog()
                            .find_model_media_link_by_id(&session.tenant_id, &next_photo.id)
                            .await?
                        {
                            next_link.is_primary = true;
                            next_link.updated_at = now.clone();
                            next_link.updated_by = Some(session.user_id.clone());
                            stores.catalog().update_model_media_link(&next_link).await?;
                        }
                    }
                }

                let audit = build_audit_entry(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "catalog.model.media_removed",
                    "equipment_model_media",
                    &link_id,
                    &format!(
                        "Vínculo da mídia '{}' ({}) removido do modelo",
                        view.title, view.original_filename
                    ),
                );
                stores.audit().append(&audit).await?;

                Ok(())
            })
        })
        .await
    }
```

### Coleta de lixo manual

`crates/core/src/services/media.rs` (linhas 855–947)

```rust
    /// Coleta de lixo manual: remove ativos e blobs sem nenhum vínculo ativo no tenant
    /// e apaga os arquivos físicos correspondentes do `MediaStore`.
    pub async fn collect_unreferenced_media(
        &self,
        session: &SessionInfo,
    ) -> Result<MediaGarbageCollectionResult> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let session_clone = session.clone();

        let (deleted_assets, unreferenced_blobs) = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                Box::pin(async move {
                    // 1. Limpa vínculos apagados logicamente para liberar FKs ON DELETE RESTRICT
                    stores
                        .catalog()
                        .purge_soft_deleted_model_media_links(&session.tenant_id)
                        .await?;

                    // 2. Remove media_asset sem nenhum vínculo ativo em equipment_model_media
                    let orphan_assets = stores
                        .catalog()
                        .list_unreferenced_media_assets(&session.tenant_id)
                        .await?;
                    for asset in &orphan_assets {
                        stores
                            .catalog()
                            .hard_delete_media_asset(&session.tenant_id, &asset.id)
                            .await?;
                    }

                    // 3. Remove media_blob sem nenhum media_asset ativo referenciando blob_id ou thumb_blob_id
                    let orphan_blobs = stores
                        .catalog()
                        .list_unreferenced_media_blobs(&session.tenant_id)
                        .await?;
                    for blob in &orphan_blobs {
                        stores
                            .catalog()
                            .hard_delete_media_blob(&session.tenant_id, &blob.id)
                            .await?;
                    }

                    let audit = build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.media.garbage_collected",
                        "media_blob",
                        &session.tenant_id,
                        &format!(
                            "Coleta de lixo de mídia executada: {} ativos órfãos e {} blobs órfãos removidos",
                            orphan_assets.len(),
                            orphan_blobs.len()
                        ),
                    );
                    stores.audit().append(&audit).await?;

                    Ok((orphan_assets.len(), orphan_blobs))
                })
            })
            .await?;

        // 4. Apaga os arquivos físicos do MediaStore para cada sha256 coletado
        let mut deleted_files = 0usize;
        let mut seen_sha = HashSet::new();
        for blob in &unreferenced_blobs {
            let _ = build_media_storage_key(&session.tenant_id, &blob.sha256)?;
            if seen_sha.insert(blob.sha256.clone()) {
                let still_referenced = self
                    .stores
                    .catalog
                    .find_media_blob_by_sha256_and_mime(
                        &session.tenant_id,
                        &blob.sha256,
                        blob.mime_type,
                    )
                    .await?
                    .is_some();
                if !still_referenced {
                    self.media_store
                        .delete_blob(&session.tenant_id, &blob.sha256)
                        .await?;
                    deleted_files += 1;
                }
            }
        }

        Ok(MediaGarbageCollectionResult {
            deleted_assets,
            deleted_blobs: unreferenced_blobs.len(),
            deleted_files,
        })
    }
```

### SQL: ativos sem vínculo (store-sqlite)

`crates/store-sqlite/src/media.rs` (linhas 494–518)

```rust
    pub(crate) async fn media_list_unreferenced_assets(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<MediaAsset>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT ma.id, ma.tenant_id, ma.kind, ma.blob_id, ma.thumb_blob_id, ma.title,
                            ma.doc_type, ma.origin, ma.credit_note, ma.language, ma.version_note,
                            ma.original_filename, ma.width, ma.height, ma.byte_size,
                            ma.created_at, ma.updated_at, ma.deleted_at, ma.created_by
                     FROM media_asset ma
                     WHERE ma.tenant_id = ?
                       AND NOT EXISTS (
                           SELECT 1 FROM equipment_model_media emm
                           WHERE emm.tenant_id = ma.tenant_id
                             AND emm.asset_id = ma.id
                             AND emm.deleted_at IS NULL
                       )",
                )
                .bind(tenant_id),
            )
            .await?;
        rows.iter().map(parse_media_asset_row).collect()
    }
```

### Limite de tamanho no Rust antes de processar (frame binário)

`src-tauri/src/commands/media.rs` (linhas 53–114)

```rust
pub fn decode_photo_ipc_frame(raw: &[u8]) -> Result<AttachModelPhotoInput, String> {
    if raw.len() < 4 {
        return Err("Payload binário de foto inválido ou truncado.".to_string());
    }
    if raw.len() > MAX_PHOTO_IPC_FRAME_BYTES {
        return Err(format!(
            "Payload binário de foto excede o limite máximo permitido no canal IPC ({} KB).",
            MAX_PHOTO_IPC_FRAME_BYTES / 1024
        ));
    }

    let header_len = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
    if header_len == 0 || header_len > MAX_FRAME_HEADER_BYTES || 4 + header_len > raw.len() {
        return Err("Cabeçalho de metadados da foto inválido ou excede 16 KB.".to_string());
    }

    let header: PhotoFrameHeader = serde_json::from_slice(&raw[4..4 + header_len])
        .map_err(|e| format!("Falha ao decodificar metadados da foto: {e}"))?;

    if header.display_byte_len == 0 || header.display_byte_len > MAX_DISPLAY_IMAGE_BYTES {
        return Err(format!(
            "A imagem de exibição excede o limite de {} KB antes do processamento.",
            MAX_DISPLAY_IMAGE_BYTES / 1024
        ));
    }
    if header.thumb_byte_len == 0 || header.thumb_byte_len > MAX_THUMB_IMAGE_BYTES {
        return Err(format!(
            "A miniatura excede o limite de {} KB antes do processamento.",
            MAX_THUMB_IMAGE_BYTES / 1024
        ));
    }

    let expected_total = 4usize
        .checked_add(header_len)
        .and_then(|v| v.checked_add(header.display_byte_len))
        .and_then(|v| v.checked_add(header.thumb_byte_len))
        .ok_or_else(|| "Tamanho total de frame de foto inválido.".to_string())?;

    if raw.len() != expected_total {
        return Err(format!(
            "Tamanho do frame binário de foto ({}) diverge do esperado ({}).",
            raw.len(),
            expected_total
        ));
    }

    let display_start = 4 + header_len;
    let display_end = display_start + header.display_byte_len;
    let thumb_end = display_end + header.thumb_byte_len;

    Ok(AttachModelPhotoInput {
        model_id: header.model_id,
        title: header.title,
        origin: header.origin,
        credit_note: header.credit_note,
        original_filename: header.original_filename,
        display_mime_type: header.display_mime_type,
        display_bytes: raw[display_start..display_end].to_vec(),
        thumb_mime_type: header.thumb_mime_type,
        thumb_bytes: raw[display_end..thumb_end].to_vec(),
    })
}
```

### Comando attach_model_photo (canal raw do Tauri v2)

`src-tauri/src/commands/media.rs` (linhas 199–221)

```rust
#[tauri::command]
pub async fn attach_model_photo(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EquipmentModelMediaView, String> {
    let session = require_session(&state)?;
    let raw_bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        InvokeBody::Json(_) => {
            return Err(
                "Transferência de foto exige canal binário raw (Uint8Array/ArrayBuffer) do Tauri v2."
                    .to_string(),
            );
        }
    };

    let input = decode_photo_ipc_frame(raw_bytes)?;
    state
        .services
        .attach_model_photo(&session, &input)
        .await
        .map_err(crate::map_core_err)
}
```

### Cópia temporária saneada para abrir PDF no leitor do sistema

`src-tauri/src/commands/media.rs` (linhas 311–336)

```rust
/// Grava uma cópia temporária com o nome saneado na pasta temporária do app
/// e retorna o caminho completo do arquivo temporário gerado.
pub fn write_sanitized_temp_document(
    temp_media_root: &Path,
    original_filename: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf, String> {
    let sanitized = sanitize_media_filename(original_filename).map_err(crate::map_core_err)?;
    let slot_dir = temp_media_root.join(Uuid::now_v7().to_string());
    std::fs::create_dir_all(&slot_dir).map_err(|e| {
        format!(
            "Falha ao criar pasta temporária de visualização ({}): {e}",
            slot_dir.display()
        )
    })?;

    let target_path = slot_dir.join(&sanitized);
    std::fs::write(&target_path, bytes).map_err(|e| {
        format!(
            "Falha ao gravar cópia temporária do documento ({}): {e}",
            target_path.display()
        )
    })?;

    Ok(target_path)
}
```

---

## 4. Testes

- Rust core: `test_media_core_validators_magic_exif_limits_and_filenames` (WebP/JPEG/PNG válidos, EXIF nos três formatos, limites de dimensão e bytes, MIME falso, truncados, PDF, quotas, metadados, nome de arquivo).
- `crates/media-fs`: `test_fs_media_store_atomic_write_dedup_and_missing_file`.
- SQLite: `test_store_media_fks_dedup_primary_rbac_quotas_audit_and_gc` (FK composta cross-tenant, índice de uma principal, deduplicação por SHA-256, concorrência otimista, RBAC do Técnico, isolamento, 12 fotos/20 documentos, GC preservando blob compartilhado, auditoria sem bytes).
- Tauri: `test_media_ipc_frame_decoders_and_temp_cleanup`.
- Vitest: `src/lib/utils/media-and-matcher.spec.ts` (10 testes).
- Playwright: `e2e/catalog-media.spec.ts`.

---

## 5. Fase 2M-rev — Fortalecimento de Segurança, Concorrência e Menor Privilégio

Esta seção consolida as implementações da revisão **Fase 2M-rev** na branch `fase-2m-midia` (PR #11).

### 5.1 Registro de Commits da Revisão

| Item        | Commit    | Descrição                                                                                                                                                                                                                      |
| ----------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **R1 & R2** | `d304da1` | Extensão fixa `.pdf` ao abrir/salvar, sanitização de stem com fallback seguro, normalização de `original_filename` ao anexar e rejeição de caracteres invisíveis/direção bidi.                                                 |
| **R3**      | `93a2e93` | Limpeza de `temp_media_dir` ao iniciar (subpastas > 1h) e ao encerrar normalmente.                                                                                                                                             |
| **R4**      | `808b459` | Menor privilégio no Tauri (`capabilities/default.json` contendo apenas `["core:default"]`), teste automatizado e ADR-0014.                                                                                                     |
| **R5**      | `040b81c` | Substituição do SHA-256 artesanal pela crate consolidada `sha2` com `spawn_blocking`, vetores NIST canônicos e ADR-0015.                                                                                                       |
| **R6**      | `981d71c` | Rejeição abrangente de metadados: JPEG APP1 (qualquer), APP13 (Photoshop/IPTC) e COM; WebP chunk "XMP " e flag VP8X (0x04); PNG tEXt, iTXt e zTXt.                                                                             |
| **R7**      | `e2496a9` | `media-fs` assíncrono em `spawn_blocking`, varredura de `*.tmp.*` > 24h ao iniciar, conciliação de arquivos órfãos em disco (> 1h), proteção de blobs recentes no GC (< 10 min) e regravação pós-transação em caso de corrida. |
| **R8**      | _este_    | Atualização deste relatório com evidências exatas de código, ADR-0013 final e resultados de testes.                                                                                                                            |

### 5.2 Decisões de Arquitetura (ADRs)

- [ADR-0013: Arquitetura de Mídia e Anexos](../decisions/0013-midia-e-anexos.md) (Finalizado com proteções R1–R7)
- [ADR-0014: Menor Privilégio nas Capabilities do Tauri Desktop](../decisions/0014-menor-privilegio-capabilities-tauri.md)
- [ADR-0015: Adoção da Crate sha2 e Validação com Vetores Oficiais do NIST](../decisions/0015-adotacao-crate-sha2-e-vetores-nist.md)

---

## 6. Evidências Exatas de Código da Revisão

### 6.1 Coleta de Lixo: Verificação de `blob_id` e `thumb_blob_id` no Store SQLite

Cópia exata de `crates/store-sqlite/src/media.rs` (linhas 494–560):

```rust
    pub(crate) async fn media_list_unreferenced_assets(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<MediaAsset>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT ma.id, ma.tenant_id, ma.kind, ma.blob_id, ma.thumb_blob_id, ma.title,
                            ma.doc_type, ma.origin, ma.credit_note, ma.language, ma.version_note,
                            ma.original_filename, ma.width, ma.height, ma.byte_size,
                            ma.created_at, ma.updated_at, ma.deleted_at, ma.created_by
                     FROM media_asset ma
                     WHERE ma.tenant_id = ?
                       AND NOT EXISTS (
                           SELECT 1 FROM equipment_model_media emm
                           WHERE emm.tenant_id = ma.tenant_id
                             AND emm.asset_id = ma.id
                             AND emm.deleted_at IS NULL
                       )",
                )
                .bind(tenant_id),
            )
            .await?;
        rows.iter().map(parse_media_asset_row).collect()
    }

    pub(crate) async fn media_hard_delete_asset(
        &self,
        tenant_id: &str,
        asset_id: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query("DELETE FROM media_asset WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(asset_id),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn media_list_unreferenced_blobs(
        &self,
        tenant_id: &str,
        created_before: &str,
    ) -> Result<Vec<MediaBlob>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT mb.id, mb.tenant_id, mb.sha256, mb.mime_type, mb.byte_size,
                            mb.width, mb.height, mb.storage_key,
                            mb.created_at, mb.updated_at, mb.deleted_at, mb.created_by
                     FROM media_blob mb
                     WHERE mb.tenant_id = ?
                       AND mb.created_at <= ?
                       AND NOT EXISTS (
                           SELECT 1 FROM media_asset ma
                           WHERE ma.tenant_id = mb.tenant_id
                             AND (ma.blob_id = mb.id OR ma.thumb_blob_id = mb.id)
                             AND ma.deleted_at IS NULL
                       )",
                )
                .bind(tenant_id)
                .bind(created_before),
            )
            .await?;
        rows.iter().map(parse_media_blob_row).collect()
    }
```

### 6.2 Frontend: Redimensionamento em `<canvas>` e Cache LRU

#### Redimensionamento em `<canvas>`

Cópia exata de `src/lib/utils/image-resize.ts` (linhas 207–244):

```typescript
  async encodeVariant(
    file: Blob,
    targetWidth: number,
    targetHeight: number,
    _preferredMime: "image/webp" | "image/jpeg",
    quality: number,
  ): Promise<EncodedImageVariant> {
    const decoded = await decodeBrowserImage(file);
    try {
      const canvas = document.createElement("canvas");
      canvas.width = targetWidth;
      canvas.height = targetHeight;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        const clean = buildCleanWebpHeaderBytes(targetWidth, targetHeight);
        return {
          bytes: clean,
          mimeType: "image/webp",
          width: targetWidth,
          height: targetHeight,
        };
      }
      decoded.draw(ctx, targetWidth, targetHeight);
      const { blob, mimeType } = await canvasToBlobWithFallback(
        canvas,
        quality,
      );
      const buf = await blob.arrayBuffer();
      return {
        bytes: new Uint8Array(buf),
        mimeType,
        width: targetWidth,
        height: targetHeight,
      };
    } finally {
      decoded.close();
    }
  },
```

#### Cache LRU de Blob URLs

Cópia exata de `src/lib/utils/media-lru-cache.ts` (linhas 22–75):

```typescript
export class MediaBlobUrlLruCache {
  private readonly maxItems: number;
  private readonly maxBytes: number;
  private readonly createUrl: (blob: Blob) => string;
  private readonly revokeUrl: (url: string) => void;
  private readonly entries = new Map<string, CacheEntry>();
  private readonly inFlight = new Map<string, Promise<string>>();
  private totalBytes = 0;

  constructor(options: MediaLruCacheOptions = {}) {
    this.maxItems = Math.max(1, options.maxItems ?? DEFAULT_MAX_ITEMS);
    this.maxBytes = Math.max(1, options.maxBytes ?? DEFAULT_MAX_BYTES);
    this.createUrl =
      options.createObjectUrl ?? ((blob: Blob) => URL.createObjectURL(blob));
    this.revokeUrl =
      options.revokeObjectUrl ?? ((url: string) => URL.revokeObjectURL(url));
  }

  get size(): number {
    return this.entries.size;
  }

  get currentBytes(): number {
    return this.totalBytes;
  }

  get(key: string): string | undefined {
    const existing = this.entries.get(key);
    if (!existing) {
      return undefined;
    }
    // Move para o final do Map (mais recentemente usado)
    this.entries.delete(key);
    this.entries.set(key, existing);
    return existing.url;
  }

  put(
    key: string,
    bytes: Uint8Array | ArrayBuffer,
    mimeType = "image/webp",
  ): string {
    const prev = this.entries.get(key);
    if (prev) {
      this.entries.delete(key);
      this.totalBytes = Math.max(0, this.totalBytes - prev.byteSize);
      this.revokeUrl(prev.url);
    }

    const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const byteSize = view.byteLength;

    while (
      this.entries.size > 0 &&
      (this.entries.size >= this.maxItems ||
        this.totalBytes + byteSize > this.maxBytes)
    ) {
      const oldestKey = this.entries.keys().next().value;
      if (!oldestKey) break;
      const entry = this.entries.get(oldestKey);
      if (entry) {
        this.totalBytes = Math.max(0, this.totalBytes - entry.byteSize);
        this.revokeUrl(entry.url);
      }
      this.entries.delete(oldestKey);
    }

    const blob = new Blob([view], { type: mimeType });
    const url = this.createUrl(blob);
    this.entries.set(key, { url, byteSize, key });
    this.totalBytes += byteSize;
    return url;
  }
```

### 6.3 Frontend: Tratamento Claro de Arquivo Ausente no Disco

Cópia exata de `src/lib/components/catalog-model-media.svelte` (linhas 114–130 e 557–578), provando que não há tela branca nem spinner infinito quando o arquivo não existe:

```typescript
try {
  const url = await mediaBlobUrlCache.getOrLoad(blobId, mimeType, () =>
    repo.getMediaBlobBytes(blobId),
  );
  blobUrls = { ...blobUrls, [blobId]: url };
} catch (err) {
  const msg = err instanceof Error ? err.message : String(err);
  failedBlobIds = {
    ...failedBlobIds,
    [blobId]: msg.includes("não encontrado")
      ? "Arquivo não encontrado no disco"
      : "Falha ao carregar imagem",
  };
}
```

E no template:

```svelte
{#if imgUrl}
  <img
    src={imgUrl}
    alt={modelName}
    class="max-h-full max-w-full object-contain"
  />
{:else if failedBlobIds[photo.blob_id] || (photo.thumb_blob_id && failedBlobIds[photo.thumb_blob_id])}
  <div
    class="flex flex-col items-center justify-center p-3 text-center text-xs text-destructive"
    data-testid="media-file-missing"
  >
    <AlertCircle class="mb-1 h-5 w-5" />
    <span class="font-medium">
      {failedBlobIds[photo.blob_id] ??
        (photo.thumb_blob_id
          ? failedBlobIds[photo.thumb_blob_id]
          : "Arquivo não encontrado no disco")}
    </span>
  </div>
{:else}
  <span class="text-xs text-muted-foreground">Carregando...</span>
{/if}
```

### 6.4 Evidência dos Testes Automatizados R1–R7

```text
running 20 tests (crates/core)
test testing::unit_tests::test_media_core_validators_magic_exif_limits_and_filenames ... ok
(Cobre: 17 casos perigosos R1 (.bat, .cmd, .ps1, .js, .vbs, .hta, .url, .lnk, .scr, .exe,
 a.pdf.bat, 'manual.pdf ', '.pdf', CON, con.pdf, nul.pdf, nome vazio, 300 chars);
 Caracteres bidi e invisíveis R2; Vetores canônicos NIST R5; Rejeição de metadados R6
 [JPEG APP1/APP13/COM, WebP XMP/VP8X, PNG tEXt/iTXt/zTXt]).

running 2 tests (crates/media-fs)
test tests::test_fs_media_store_atomic_write_dedup_and_missing_file ... ok
test tests::test_fs_media_store_gc_reconciliation_and_temp_cleanup ... ok
(Cobre: I/O assíncrono em spawn_blocking R7.1; Limpeza de temporários *.tmp.* > 24h R7.2;
 Reconciliação em disco excluindo órfãos > 1h e preservando arquivos < 1h R7.3 & R7.6).

running 31 tests (crates/store-sqlite)
test tests::test_store_media_fks_dedup_primary_rbac_quotas_audit_and_gc ... ok
test tests::test_media_attach_rewrites_blob_if_missing_after_transaction ... ok
(Cobre: Proteção de blob recente < 10 min no GC do banco R7.4 & R7.6;
 Regravação automática a partir de buffer em memória em caso de corrida R7.5).

running 2 tests (src-tauri)
test guards::test_tauri_capabilities_enforce_least_privilege ... ok
test commands::media::test_media_ipc_frame_decoders_and_temp_cleanup ... ok
(Cobre: Menor privilégio estrito no Tauri R4; Limpeza de temp-media ao iniciar e desligar R3;
 Prova de que os 17 nomes maliciosos resultam em arquivo seguro dentro de temp_media_root terminando em .pdf R1).
```

As URLs raw fixadas no hash do último commit estão na resposta final do agente.
