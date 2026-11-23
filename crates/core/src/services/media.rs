use chrono::Utc;
use std::collections::HashSet;
use uuid::Uuid;

use crate::domain::{
    build_media_storage_key, normalize_media_filename, validate_display_image,
    validate_media_title, validate_model_media_quota, validate_optional_media_language,
    validate_optional_media_note, validate_pdf_document, validate_thumbnail_image,
    AttachModelDocumentInput, AttachModelPhotoInput, EquipmentModelMedia, EquipmentModelMediaView,
    MediaAsset, MediaBlob, MediaGarbageCollectionResult, MediaKind, MediaMimeType,
    ReorderModelMediaInput, SessionInfo, UpdateModelMediaInput, PERM_CATALOG_MANAGE,
    PERM_CATALOG_READ,
};
use crate::error::{CoreError, Result};
use crate::services::{build_audit_entry, CoreServices};

impl CoreServices {
    pub async fn list_model_media(
        &self,
        session: &SessionInfo,
        model_id: &str,
    ) -> Result<Vec<EquipmentModelMediaView>> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        let _model = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, model_id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
            })?;

        self.stores
            .catalog
            .list_model_media_views(&session.tenant_id, model_id)
            .await
    }

    pub async fn get_media_blob_bytes(
        &self,
        session: &SessionInfo,
        blob_id: &str,
    ) -> Result<(MediaBlob, Vec<u8>)> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        let blob = self
            .stores
            .catalog
            .find_media_blob_by_id(&session.tenant_id, blob_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Blob de mídia não encontrado.".to_string()))?;

        let bytes = self
            .media_store
            .get_blob(&session.tenant_id, &blob.sha256)
            .await?;

        Ok((blob, bytes))
    }

    pub async fn get_model_document_for_external(
        &self,
        session: &SessionInfo,
        media_link_id: &str,
    ) -> Result<(EquipmentModelMediaView, Vec<u8>)> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        let view = self
            .stores
            .catalog
            .find_model_media_view_by_link_id(&session.tenant_id, media_link_id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Documento do modelo não encontrado.".to_string())
            })?;

        if view.kind != MediaKind::Document {
            return Err(CoreError::Validation(
                "O item solicitado não é um documento PDF.".to_string(),
            ));
        }

        let bytes = self
            .media_store
            .get_blob(&session.tenant_id, &view.blob_sha256)
            .await?;

        Ok((view, bytes))
    }

    pub async fn attach_model_photo(
        &self,
        session: &SessionInfo,
        input: &AttachModelPhotoInput,
    ) -> Result<EquipmentModelMediaView> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;

        let title = validate_media_title(&input.title)?;
        let credit_note =
            validate_optional_media_note("credit_note", input.credit_note.as_deref())?;
        let display = validate_display_image(&input.display_mime_type, &input.display_bytes)?;
        let thumb = validate_thumbnail_image(&input.thumb_mime_type, &input.thumb_bytes)?;
        let original_filename =
            normalize_media_filename(&input.original_filename, display.mime_type)?;

        // Verifica existência do modelo e quota antes de gravar no MediaStore
        let _model = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, &input.model_id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
            })?;

        let existing_photos = self
            .stores
            .catalog
            .count_model_media_by_kind(&session.tenant_id, &input.model_id, MediaKind::Image)
            .await?;
        validate_model_media_quota(MediaKind::Image, existing_photos)?;

        // Gravação atômica e idempotente dos blobs no MediaStore antes de abrir a transação SQLite
        let display_storage_key = self
            .media_store
            .put_blob(&session.tenant_id, &display.sha256, &input.display_bytes)
            .await?;
        let thumb_storage_key = self
            .media_store
            .put_blob(&session.tenant_id, &thumb.sha256, &input.thumb_bytes)
            .await?;

        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let model_id = input.model_id.clone();
        let origin = input.origin;

        let created_view = self.run_transaction(|stores| {
            let session = session_clone.clone();
            let model_id = model_id.clone();
            let title = title.clone();
            let credit_note = credit_note.clone();
            let original_filename = original_filename.clone();
            let display = display.clone();
            let thumb = thumb.clone();
            let display_storage_key = display_storage_key.clone();
            let thumb_storage_key = thumb_storage_key.clone();
            let now = now.clone();

            Box::pin(async move {
                let model = stores
                    .catalog()
                    .find_model_by_id(&session.tenant_id, &model_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                    })?;

                let current_photos = stores
                    .catalog()
                    .count_model_media_by_kind(&session.tenant_id, &model_id, MediaKind::Image)
                    .await?;
                validate_model_media_quota(MediaKind::Image, current_photos)?;

                let display_blob = if let Some(existing) = stores
                    .catalog()
                    .find_media_blob_by_sha256_and_mime(
                        &session.tenant_id,
                        &display.sha256,
                        display.mime_type,
                    )
                    .await?
                {
                    existing
                } else {
                    let blob = MediaBlob {
                        id: Uuid::now_v7().to_string(),
                        tenant_id: session.tenant_id.clone(),
                        sha256: display.sha256.clone(),
                        mime_type: display.mime_type,
                        byte_size: display.byte_size,
                        width: Some(display.width),
                        height: Some(display.height),
                        storage_key: display_storage_key,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        created_by: Some(session.user_id.clone()),
                        updated_by: Some(session.user_id.clone()),
                        deleted_at: None,
                    };
                    stores.catalog().create_media_blob(&blob).await?;
                    blob
                };

                let thumb_blob = if let Some(existing) = stores
                    .catalog()
                    .find_media_blob_by_sha256_and_mime(
                        &session.tenant_id,
                        &thumb.sha256,
                        thumb.mime_type,
                    )
                    .await?
                {
                    existing
                } else {
                    let blob = MediaBlob {
                        id: Uuid::now_v7().to_string(),
                        tenant_id: session.tenant_id.clone(),
                        sha256: thumb.sha256.clone(),
                        mime_type: thumb.mime_type,
                        byte_size: thumb.byte_size,
                        width: Some(thumb.width),
                        height: Some(thumb.height),
                        storage_key: thumb_storage_key,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        created_by: Some(session.user_id.clone()),
                        updated_by: Some(session.user_id.clone()),
                        deleted_at: None,
                    };
                    stores.catalog().create_media_blob(&blob).await?;
                    blob
                };

                let asset_id = Uuid::now_v7().to_string();
                let asset = MediaAsset {
                    id: asset_id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    kind: MediaKind::Image,
                    blob_id: display_blob.id.clone(),
                    thumb_blob_id: Some(thumb_blob.id.clone()),
                    title: title.clone(),
                    doc_type: None,
                    origin,
                    credit_note: credit_note.clone(),
                    language: None,
                    version_note: None,
                    original_filename: original_filename.clone(),
                    width: Some(display.width),
                    height: Some(display.height),
                    byte_size: display.byte_size,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    created_by: Some(session.user_id.clone()),
                    updated_by: Some(session.user_id.clone()),
                    deleted_at: None,
                };
                stores.catalog().create_media_asset(&asset).await?;

                // A primeira foto anexada ao modelo torna-se automaticamente a principal
                let existing_views = stores
                    .catalog()
                    .list_model_media_views(&session.tenant_id, &model_id)
                    .await?;
                let existing_images: Vec<&EquipmentModelMediaView> = existing_views
                    .iter()
                    .filter(|v| v.kind == MediaKind::Image)
                    .collect();
                let is_primary = !existing_images.iter().any(|v| v.is_primary);
                let next_sort_order = existing_images
                    .iter()
                    .map(|v| v.sort_order)
                    .max()
                    .map(|m| m + 1)
                    .unwrap_or(0);

                let link_id = Uuid::now_v7().to_string();
                let link = EquipmentModelMedia {
                    id: link_id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    model_id: model_id.clone(),
                    asset_id: asset_id.clone(),
                    is_primary,
                    sort_order: next_sort_order,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    created_by: Some(session.user_id.clone()),
                    updated_by: Some(session.user_id.clone()),
                    deleted_at: None,
                };
                stores.catalog().create_model_media_link(&link).await?;

                let audit = build_audit_entry(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "catalog.model.photo_attached",
                    "equipment_model_media",
                    &link_id,
                    &format!(
                        "Foto '{}' ({}) anexada ao modelo '{}' (sha256: {}, {} bytes, {}x{} px, principal: {})",
                        title,
                        original_filename,
                        model.name,
                        display.sha256,
                        display.byte_size,
                        display.width,
                        display.height,
                        is_primary
                    ),
                );
                stores.audit().append(&audit).await?;

                stores
                    .catalog()
                    .find_model_media_view_by_link_id(&session.tenant_id, &link_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Internal(
                            "Falha ao recarregar foto anexada ao modelo.".to_string(),
                        )
                    })
            })
        })
        .await?;

        // R7.5: Se o arquivo no disco não existir após a transação (corrida com GC antigo), regravar a partir do buffer em memória
        if !self
            .media_store
            .exists_blob(&session.tenant_id, &display.sha256)
            .await?
        {
            tracing::warn!(
                "Display blob ausente após transação de foto; regravando no MediaStore."
            );
            self.media_store
                .put_blob(&session.tenant_id, &display.sha256, &input.display_bytes)
                .await?;
        }
        if !self
            .media_store
            .exists_blob(&session.tenant_id, &thumb.sha256)
            .await?
        {
            tracing::warn!("Thumb blob ausente após transação de foto; regravando no MediaStore.");
            self.media_store
                .put_blob(&session.tenant_id, &thumb.sha256, &input.thumb_bytes)
                .await?;
        }

        Ok(created_view)
    }

    pub async fn attach_model_document(
        &self,
        session: &SessionInfo,
        input: &AttachModelDocumentInput,
    ) -> Result<EquipmentModelMediaView> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;

        let title = validate_media_title(&input.title)?;
        let credit_note =
            validate_optional_media_note("credit_note", input.credit_note.as_deref())?;
        let version_note =
            validate_optional_media_note("version_note", input.version_note.as_deref())?;
        let language = validate_optional_media_language(input.language.as_deref())?;
        let original_filename =
            normalize_media_filename(&input.original_filename, MediaMimeType::ApplicationPdf)?;
        let pdf = validate_pdf_document(&input.mime_type, &input.bytes).await?;

        let _model = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, &input.model_id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
            })?;

        let existing_docs = self
            .stores
            .catalog
            .count_model_media_by_kind(&session.tenant_id, &input.model_id, MediaKind::Document)
            .await?;
        validate_model_media_quota(MediaKind::Document, existing_docs)?;

        let storage_key = self
            .media_store
            .put_blob(&session.tenant_id, &pdf.sha256, &input.bytes)
            .await?;

        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let model_id = input.model_id.clone();
        let doc_type = input.doc_type;
        let origin = input.origin;

        let created_view = self.run_transaction(|stores| {
            let session = session_clone.clone();
            let model_id = model_id.clone();
            let title = title.clone();
            let credit_note = credit_note.clone();
            let version_note = version_note.clone();
            let language = language.clone();
            let original_filename = original_filename.clone();
            let pdf = pdf.clone();
            let storage_key = storage_key.clone();
            let now = now.clone();

            Box::pin(async move {
                let model = stores
                    .catalog()
                    .find_model_by_id(&session.tenant_id, &model_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                    })?;

                let current_docs = stores
                    .catalog()
                    .count_model_media_by_kind(&session.tenant_id, &model_id, MediaKind::Document)
                    .await?;
                validate_model_media_quota(MediaKind::Document, current_docs)?;

                let blob = if let Some(existing) = stores
                    .catalog()
                    .find_media_blob_by_sha256_and_mime(
                        &session.tenant_id,
                        &pdf.sha256,
                        pdf.mime_type,
                    )
                    .await?
                {
                    existing
                } else {
                    let new_blob = MediaBlob {
                        id: Uuid::now_v7().to_string(),
                        tenant_id: session.tenant_id.clone(),
                        sha256: pdf.sha256.clone(),
                        mime_type: pdf.mime_type,
                        byte_size: pdf.byte_size,
                        width: None,
                        height: None,
                        storage_key,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        created_by: Some(session.user_id.clone()),
                        updated_by: Some(session.user_id.clone()),
                        deleted_at: None,
                    };
                    stores.catalog().create_media_blob(&new_blob).await?;
                    new_blob
                };

                let asset_id = Uuid::now_v7().to_string();
                let asset = MediaAsset {
                    id: asset_id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    kind: MediaKind::Document,
                    blob_id: blob.id.clone(),
                    thumb_blob_id: None,
                    title: title.clone(),
                    doc_type: Some(doc_type),
                    origin,
                    credit_note,
                    language,
                    version_note,
                    original_filename: original_filename.clone(),
                    width: None,
                    height: None,
                    byte_size: pdf.byte_size,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    created_by: Some(session.user_id.clone()),
                    updated_by: Some(session.user_id.clone()),
                    deleted_at: None,
                };
                stores.catalog().create_media_asset(&asset).await?;

                let existing_views = stores
                    .catalog()
                    .list_model_media_views(&session.tenant_id, &model_id)
                    .await?;
                let next_sort_order = existing_views
                    .iter()
                    .filter(|v| v.kind == MediaKind::Document)
                    .map(|v| v.sort_order)
                    .max()
                    .map(|m| m + 1)
                    .unwrap_or(0);

                let link_id = Uuid::now_v7().to_string();
                let link = EquipmentModelMedia {
                    id: link_id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    model_id: model_id.clone(),
                    asset_id,
                    is_primary: false,
                    sort_order: next_sort_order,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    created_by: Some(session.user_id.clone()),
                    updated_by: Some(session.user_id.clone()),
                    deleted_at: None,
                };
                stores.catalog().create_model_media_link(&link).await?;

                let audit = build_audit_entry(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "catalog.model.document_attached",
                    "equipment_model_media",
                    &link_id,
                    &format!(
                        "Documento '{}' ({}, tipo: {}) anexado ao modelo '{}' (sha256: {}, {} bytes)",
                        title,
                        original_filename,
                        doc_type.as_str(),
                        model.name,
                        pdf.sha256,
                        pdf.byte_size
                    ),
                );
                stores.audit().append(&audit).await?;

                stores
                    .catalog()
                    .find_model_media_view_by_link_id(&session.tenant_id, &link_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Internal(
                            "Falha ao recarregar documento anexado ao modelo.".to_string(),
                        )
                    })
            })
        })
        .await?;

        // R7.5: Se o arquivo no disco não existir após a transação (corrida com GC antigo), regravar a partir do buffer em memória
        if !self
            .media_store
            .exists_blob(&session.tenant_id, &pdf.sha256)
            .await?
        {
            tracing::warn!(
                "Document blob ausente após transação de documento; regravando no MediaStore."
            );
            self.media_store
                .put_blob(&session.tenant_id, &pdf.sha256, &input.bytes)
                .await?;
        }

        Ok(created_view)
    }

    pub async fn update_model_media_metadata(
        &self,
        session: &SessionInfo,
        input: &UpdateModelMediaInput,
    ) -> Result<EquipmentModelMediaView> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;

        let title = validate_media_title(&input.title)?;
        let credit_note =
            validate_optional_media_note("credit_note", input.credit_note.as_deref())?;
        let version_note =
            validate_optional_media_note("version_note", input.version_note.as_deref())?;
        let language = validate_optional_media_language(input.language.as_deref())?;

        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let input_clone = input.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let input = input_clone.clone();
            let title = title.clone();
            let credit_note = credit_note.clone();
            let version_note = version_note.clone();
            let language = language.clone();
            let now = now.clone();

            Box::pin(async move {
                let mut link = stores
                    .catalog()
                    .find_model_media_link_by_id(&session.tenant_id, &input.media_link_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Mídia do modelo não encontrada.".to_string())
                    })?;

                if link.updated_at != input.expected_updated_at {
                    return Err(CoreError::Conflict(
                        "Os metadados desta mídia foram alterados por outra operação. Recarregue antes de salvar.".to_string(),
                    ));
                }

                let mut asset = stores
                    .catalog()
                    .find_media_asset_by_id(&session.tenant_id, &link.asset_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Ativo de mídia não encontrado.".to_string())
                    })?;

                let resolved_doc_type = match asset.kind {
                    MediaKind::Image => None,
                    MediaKind::Document => Some(input.doc_type.ok_or_else(|| {
                        CoreError::Validation(
                            "O tipo de documento (doc_type) é obrigatório para documentos."
                                .to_string(),
                        )
                    })?),
                };

                asset.title = title.clone();
                asset.doc_type = resolved_doc_type;
                asset.origin = input.origin;
                asset.credit_note = credit_note;
                asset.language = if asset.kind == MediaKind::Document {
                    language
                } else {
                    None
                };
                asset.version_note = if asset.kind == MediaKind::Document {
                    version_note
                } else {
                    None
                };
                asset.updated_at = now.clone();
                asset.updated_by = Some(session.user_id.clone());

                stores.catalog().update_media_asset(&asset).await?;

                link.updated_at = now.clone();
                link.updated_by = Some(session.user_id.clone());
                stores.catalog().update_model_media_link(&link).await?;

                let audit = build_audit_entry(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "catalog.model.media_updated",
                    "equipment_model_media",
                    &link.id,
                    &format!(
                        "Metadados da mídia '{}' atualizados (origem: {})",
                        title,
                        input.origin.as_str()
                    ),
                );
                stores.audit().append(&audit).await?;

                stores
                    .catalog()
                    .find_model_media_view_by_link_id(&session.tenant_id, &link.id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Internal("Falha ao recarregar mídia atualizada.".to_string())
                    })
            })
        })
        .await
    }

    pub async fn reorder_model_media(
        &self,
        session: &SessionInfo,
        input: &ReorderModelMediaInput,
    ) -> Result<Vec<EquipmentModelMediaView>> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        if input.ordered_link_ids.is_empty() {
            return self.list_model_media(session, &input.model_id).await;
        }

        let unique_ids: HashSet<&String> = input.ordered_link_ids.iter().collect();
        if unique_ids.len() != input.ordered_link_ids.len() {
            return Err(CoreError::Validation(
                "A lista de ordenação de mídia contém identificadores duplicados.".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let input_clone = input.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let input = input_clone.clone();
            let now = now.clone();

            Box::pin(async move {
                let _model = stores
                    .catalog()
                    .find_model_by_id(&session.tenant_id, &input.model_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                    })?;

                let existing_views = stores
                    .catalog()
                    .list_model_media_views(&session.tenant_id, &input.model_id)
                    .await?;
                let kind_views: Vec<&EquipmentModelMediaView> = existing_views
                    .iter()
                    .filter(|v| v.kind == input.kind)
                    .collect();

                if kind_views.len() != input.ordered_link_ids.len() {
                    return Err(CoreError::Validation(
                        "A lista de ordenação deve conter todos os itens ativos do tipo informado para este modelo.".to_string(),
                    ));
                }

                let existing_ids: HashSet<&str> = kind_views.iter().map(|v| v.id.as_str()).collect();
                for id in &input.ordered_link_ids {
                    if !existing_ids.contains(id.as_str()) {
                        return Err(CoreError::Validation(format!(
                            "O vínculo de mídia '{id}' não pertence a este modelo ou tipo."
                        )));
                    }
                }

                for (idx, link_id) in input.ordered_link_ids.iter().enumerate() {
                    let mut link = stores
                        .catalog()
                        .find_model_media_link_by_id(&session.tenant_id, link_id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound(
                                "Vínculo de mídia não encontrado durante reordenação.".to_string(),
                            )
                        })?;
                    link.sort_order = idx as i32;
                    link.updated_at = now.clone();
                    link.updated_by = Some(session.user_id.clone());
                    stores.catalog().update_model_media_link(&link).await?;
                }

                let audit = build_audit_entry(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "catalog.model.media_reordered",
                    "equipment_model",
                    &input.model_id,
                    &format!(
                        "Mídias ({}) do modelo reordenadas ({} itens)",
                        input.kind.as_str(),
                        input.ordered_link_ids.len()
                    ),
                );
                stores.audit().append(&audit).await?;

                stores
                    .catalog()
                    .list_model_media_views(&session.tenant_id, &input.model_id)
                    .await
            })
        })
        .await
    }

    pub async fn set_primary_model_photo(
        &self,
        session: &SessionInfo,
        media_link_id: &str,
    ) -> Result<EquipmentModelMediaView> {
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
                        CoreError::NotFound("Foto do modelo não encontrada.".to_string())
                    })?;

                if view.kind != MediaKind::Image {
                    return Err(CoreError::Validation(
                        "Apenas fotos podem ser definidas como imagem principal do modelo."
                            .to_string(),
                    ));
                }

                let mut link = stores
                    .catalog()
                    .find_model_media_link_by_id(&session.tenant_id, &link_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Vínculo de foto não encontrado.".to_string())
                    })?;

                stores
                    .catalog()
                    .clear_primary_model_photo(
                        &session.tenant_id,
                        &link.model_id,
                        &now,
                        Some(&session.user_id),
                    )
                    .await?;

                link.is_primary = true;
                link.updated_at = now.clone();
                link.updated_by = Some(session.user_id.clone());
                stores.catalog().update_model_media_link(&link).await?;

                let audit = build_audit_entry(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "catalog.model.primary_photo_set",
                    "equipment_model_media",
                    &link.id,
                    &format!("Foto '{}' definida como principal do modelo", view.title),
                );
                stores.audit().append(&audit).await?;

                stores
                    .catalog()
                    .find_model_media_view_by_link_id(&session.tenant_id, &link.id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Internal("Falha ao recarregar foto principal.".to_string())
                    })
            })
        })
        .await
    }

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

    /// Coleta de lixo manual: remove ativos e blobs sem nenhum vínculo ativo no tenant
    /// e apaga os arquivos físicos correspondentes do `MediaStore`.
    pub async fn collect_unreferenced_media(
        &self,
        session: &SessionInfo,
    ) -> Result<MediaGarbageCollectionResult> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let session_clone = session.clone();

        // R7.4: Filtro created_at < now() - 10 minutos (blobs recém-criados nunca são coletados do banco)
        let cutoff = (Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        let cutoff_clone = cutoff.clone();

        let (deleted_assets, unreferenced_blobs) = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let cutoff = cutoff_clone.clone();
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
                    // E criado há mais de 10 minutos (R7.4)
                    let orphan_blobs = stores
                        .catalog()
                        .list_unreferenced_media_blobs(&session.tenant_id, &cutoff)
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

        // 5. R7.3 Conciliação na coleta de lixo: varre os arquivos em disco e remove
        // aqueles sem linha correspondente na tabela media_blob HÁ MAIS DE 1 HORA.
        let valid_shas = self
            .stores
            .catalog
            .list_active_media_blob_shas(&session.tenant_id)
            .await?;
        let reconciled_files = self
            .media_store
            .reconcile_unreferenced_blobs(
                &session.tenant_id,
                &valid_shas,
                std::time::Duration::from_secs(3600),
            )
            .await?;
        deleted_files += reconciled_files;

        Ok(MediaGarbageCollectionResult {
            deleted_assets,
            deleted_blobs: unreferenced_blobs.len(),
            deleted_files,
        })
    }
}
