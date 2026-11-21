use eventrent_core::{
    CoreError, EquipmentModelMedia, EquipmentModelMediaView, MediaAsset, MediaBlob, MediaDocType,
    MediaKind, MediaMimeType, MediaOrigin, Result,
};
use sqlx::Row;

use crate::SqliteStore;

fn parse_media_blob_row(row: &sqlx::sqlite::SqliteRow) -> Result<MediaBlob> {
    let mime_raw: String = row.get("mime_type");
    let mime_type = MediaMimeType::parse_str(&mime_raw)?;

    Ok(MediaBlob {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        sha256: row.get("sha256"),
        mime_type,
        byte_size: row.get("byte_size"),
        width: row.get("width"),
        height: row.get("height"),
        storage_key: row.get("storage_key"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: None,
        deleted_at: row.get("deleted_at"),
    })
}

fn parse_media_asset_row(row: &sqlx::sqlite::SqliteRow) -> Result<MediaAsset> {
    let kind_raw: String = row.get("kind");
    let kind = MediaKind::parse_str(&kind_raw)?;
    let doc_type_raw: Option<String> = row.get("doc_type");
    let doc_type = match doc_type_raw {
        Some(v) => Some(MediaDocType::parse_str(&v)?),
        None => None,
    };
    let origin_raw: String = row.get("origin");
    let origin = MediaOrigin::parse_str(&origin_raw)?;

    Ok(MediaAsset {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        kind,
        blob_id: row.get("blob_id"),
        thumb_blob_id: row.get("thumb_blob_id"),
        title: row.get("title"),
        doc_type,
        origin,
        credit_note: row.get("credit_note"),
        language: row.get("language"),
        version_note: row.get("version_note"),
        original_filename: row.get("original_filename"),
        width: row.get("width"),
        height: row.get("height"),
        byte_size: row.get("byte_size"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: None,
        deleted_at: row.get("deleted_at"),
    })
}

fn parse_model_media_link_row(row: &sqlx::sqlite::SqliteRow) -> EquipmentModelMedia {
    EquipmentModelMedia {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        model_id: row.get("model_id"),
        asset_id: row.get("asset_id"),
        is_primary: row.get::<i64, _>("is_primary") != 0,
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: None,
        deleted_at: row.get("deleted_at"),
    }
}

fn parse_model_media_view_row(row: &sqlx::sqlite::SqliteRow) -> Result<EquipmentModelMediaView> {
    let kind_raw: String = row.get("kind");
    let kind = MediaKind::parse_str(&kind_raw)?;
    let mime_raw: String = row.get("mime_type");
    let mime_type = MediaMimeType::parse_str(&mime_raw)?;
    let thumb_mime_raw: Option<String> = row.get("thumb_mime_type");
    let thumb_mime_type = match thumb_mime_raw {
        Some(v) => Some(MediaMimeType::parse_str(&v)?),
        None => None,
    };
    let doc_type_raw: Option<String> = row.get("doc_type");
    let doc_type = match doc_type_raw {
        Some(v) => Some(MediaDocType::parse_str(&v)?),
        None => None,
    };
    let origin_raw: String = row.get("origin");
    let origin = MediaOrigin::parse_str(&origin_raw)?;

    Ok(EquipmentModelMediaView {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        model_id: row.get("model_id"),
        asset_id: row.get("asset_id"),
        kind,
        blob_id: row.get("blob_id"),
        blob_sha256: row.get("blob_sha256"),
        mime_type,
        thumb_blob_id: row.get("thumb_blob_id"),
        thumb_sha256: row.get("thumb_sha256"),
        thumb_mime_type,
        title: row.get("title"),
        doc_type,
        origin,
        credit_note: row.get("credit_note"),
        language: row.get("language"),
        version_note: row.get("version_note"),
        original_filename: row.get("original_filename"),
        width: row.get("width"),
        height: row.get("height"),
        byte_size: row.get("byte_size"),
        is_primary: row.get::<i64, _>("is_primary") != 0,
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

impl SqliteStore {
    pub(crate) async fn media_find_blob_by_id(
        &self,
        tenant_id: &str,
        blob_id: &str,
    ) -> Result<Option<MediaBlob>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, sha256, mime_type, byte_size, width, height, storage_key,
                            created_at, updated_at, deleted_at, created_by
                     FROM media_blob
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(blob_id),
            )
            .await?;
        row.as_ref().map(parse_media_blob_row).transpose()
    }

    pub(crate) async fn media_find_blob_by_sha256_and_mime(
        &self,
        tenant_id: &str,
        sha256: &str,
        mime_type: MediaMimeType,
    ) -> Result<Option<MediaBlob>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, sha256, mime_type, byte_size, width, height, storage_key,
                            created_at, updated_at, deleted_at, created_by
                     FROM media_blob
                     WHERE tenant_id = ? AND sha256 = ? AND mime_type = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(sha256)
                .bind(mime_type.as_str()),
            )
            .await?;
        row.as_ref().map(parse_media_blob_row).transpose()
    }

    pub(crate) async fn media_create_blob(&self, blob: &MediaBlob) -> Result<()> {
        self.execute(
            sqlx::query(
                "INSERT INTO media_blob (
                    id, tenant_id, sha256, mime_type, byte_size, width, height, storage_key,
                    created_at, updated_at, deleted_at, created_by
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&blob.id)
            .bind(&blob.tenant_id)
            .bind(&blob.sha256)
            .bind(blob.mime_type.as_str())
            .bind(blob.byte_size)
            .bind(blob.width)
            .bind(blob.height)
            .bind(&blob.storage_key)
            .bind(&blob.created_at)
            .bind(&blob.updated_at)
            .bind(&blob.deleted_at)
            .bind(&blob.created_by),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn media_find_asset_by_id(
        &self,
        tenant_id: &str,
        asset_id: &str,
    ) -> Result<Option<MediaAsset>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, kind, blob_id, thumb_blob_id, title, doc_type, origin,
                            credit_note, language, version_note, original_filename, width, height,
                            byte_size, created_at, updated_at, deleted_at, created_by
                     FROM media_asset
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(asset_id),
            )
            .await?;
        row.as_ref().map(parse_media_asset_row).transpose()
    }

    pub(crate) async fn media_create_asset(&self, asset: &MediaAsset) -> Result<()> {
        self.execute(
            sqlx::query(
                "INSERT INTO media_asset (
                    id, tenant_id, kind, blob_id, thumb_blob_id, title, doc_type, origin,
                    credit_note, language, version_note, original_filename, width, height,
                    byte_size, created_at, updated_at, deleted_at, created_by
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&asset.id)
            .bind(&asset.tenant_id)
            .bind(asset.kind.as_str())
            .bind(&asset.blob_id)
            .bind(&asset.thumb_blob_id)
            .bind(&asset.title)
            .bind(asset.doc_type.map(|d| d.as_str()))
            .bind(asset.origin.as_str())
            .bind(&asset.credit_note)
            .bind(&asset.language)
            .bind(&asset.version_note)
            .bind(&asset.original_filename)
            .bind(asset.width)
            .bind(asset.height)
            .bind(asset.byte_size)
            .bind(&asset.created_at)
            .bind(&asset.updated_at)
            .bind(&asset.deleted_at)
            .bind(&asset.created_by),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn media_update_asset(&self, asset: &MediaAsset) -> Result<()> {
        let rows = self
            .execute(
                sqlx::query(
                    "UPDATE media_asset
                     SET title = ?, doc_type = ?, origin = ?, credit_note = ?, language = ?,
                         version_note = ?, updated_at = ?
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(&asset.title)
                .bind(asset.doc_type.map(|d| d.as_str()))
                .bind(asset.origin.as_str())
                .bind(&asset.credit_note)
                .bind(&asset.language)
                .bind(&asset.version_note)
                .bind(&asset.updated_at)
                .bind(&asset.tenant_id)
                .bind(&asset.id),
            )
            .await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(
                "Ativo de mídia não encontrado para atualização.".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) async fn media_list_model_views(
        &self,
        tenant_id: &str,
        model_id: &str,
    ) -> Result<Vec<EquipmentModelMediaView>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT emm.id, emm.tenant_id, emm.model_id, emm.asset_id,
                            ma.kind, ma.blob_id, mb.sha256 AS blob_sha256, mb.mime_type,
                            ma.thumb_blob_id, tmb.sha256 AS thumb_sha256, tmb.mime_type AS thumb_mime_type,
                            ma.title, ma.doc_type, ma.origin, ma.credit_note, ma.language,
                            ma.version_note, ma.original_filename, ma.width, ma.height, ma.byte_size,
                            emm.is_primary, emm.sort_order, emm.created_at, emm.updated_at
                     FROM equipment_model_media emm
                     INNER JOIN media_asset ma
                        ON ma.tenant_id = emm.tenant_id AND ma.id = emm.asset_id
                     INNER JOIN media_blob mb
                        ON mb.tenant_id = ma.tenant_id AND mb.id = ma.blob_id
                     LEFT JOIN media_blob tmb
                        ON tmb.tenant_id = ma.tenant_id AND tmb.id = ma.thumb_blob_id
                     WHERE emm.tenant_id = ? AND emm.model_id = ?
                       AND emm.deleted_at IS NULL AND ma.deleted_at IS NULL AND mb.deleted_at IS NULL
                     ORDER BY emm.is_primary DESC, emm.sort_order ASC, emm.created_at ASC",
                )
                .bind(tenant_id)
                .bind(model_id),
            )
            .await?;
        rows.iter().map(parse_model_media_view_row).collect()
    }

    pub(crate) async fn media_find_link_by_id(
        &self,
        tenant_id: &str,
        link_id: &str,
    ) -> Result<Option<EquipmentModelMedia>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, model_id, asset_id, is_primary, sort_order,
                            created_at, updated_at, deleted_at, created_by
                     FROM equipment_model_media
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(link_id),
            )
            .await?;
        Ok(row.as_ref().map(parse_model_media_link_row))
    }

    pub(crate) async fn media_find_view_by_link_id(
        &self,
        tenant_id: &str,
        link_id: &str,
    ) -> Result<Option<EquipmentModelMediaView>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT emm.id, emm.tenant_id, emm.model_id, emm.asset_id,
                            ma.kind, ma.blob_id, mb.sha256 AS blob_sha256, mb.mime_type,
                            ma.thumb_blob_id, tmb.sha256 AS thumb_sha256, tmb.mime_type AS thumb_mime_type,
                            ma.title, ma.doc_type, ma.origin, ma.credit_note, ma.language,
                            ma.version_note, ma.original_filename, ma.width, ma.height, ma.byte_size,
                            emm.is_primary, emm.sort_order, emm.created_at, emm.updated_at
                     FROM equipment_model_media emm
                     INNER JOIN media_asset ma
                        ON ma.tenant_id = emm.tenant_id AND ma.id = emm.asset_id
                     INNER JOIN media_blob mb
                        ON mb.tenant_id = ma.tenant_id AND mb.id = ma.blob_id
                     LEFT JOIN media_blob tmb
                        ON tmb.tenant_id = ma.tenant_id AND tmb.id = ma.thumb_blob_id
                     WHERE emm.tenant_id = ? AND emm.id = ?
                       AND emm.deleted_at IS NULL AND ma.deleted_at IS NULL AND mb.deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(link_id),
            )
            .await?;
        row.as_ref().map(parse_model_media_view_row).transpose()
    }

    pub(crate) async fn media_count_by_model_and_kind(
        &self,
        tenant_id: &str,
        model_id: &str,
        kind: MediaKind,
    ) -> Result<usize> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) AS cnt
                     FROM equipment_model_media emm
                     INNER JOIN media_asset ma
                        ON ma.tenant_id = emm.tenant_id AND ma.id = emm.asset_id
                     WHERE emm.tenant_id = ? AND emm.model_id = ? AND ma.kind = ?
                       AND emm.deleted_at IS NULL AND ma.deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(model_id)
                .bind(kind.as_str()),
            )
            .await?;
        let cnt: i64 = row.get("cnt");
        Ok(cnt.max(0) as usize)
    }

    pub(crate) async fn media_create_link(&self, link: &EquipmentModelMedia) -> Result<()> {
        self.execute(
            sqlx::query(
                "INSERT INTO equipment_model_media (
                    id, tenant_id, model_id, asset_id, is_primary, sort_order,
                    created_at, updated_at, deleted_at, created_by
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&link.id)
            .bind(&link.tenant_id)
            .bind(&link.model_id)
            .bind(&link.asset_id)
            .bind(if link.is_primary { 1 } else { 0 })
            .bind(link.sort_order)
            .bind(&link.created_at)
            .bind(&link.updated_at)
            .bind(&link.deleted_at)
            .bind(&link.created_by),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn media_update_link(&self, link: &EquipmentModelMedia) -> Result<()> {
        let rows = self
            .execute(
                sqlx::query(
                    "UPDATE equipment_model_media
                     SET is_primary = ?, sort_order = ?, updated_at = ?
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(if link.is_primary { 1 } else { 0 })
                .bind(link.sort_order)
                .bind(&link.updated_at)
                .bind(&link.tenant_id)
                .bind(&link.id),
            )
            .await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(
                "Vínculo de mídia não encontrado.".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) async fn media_clear_primary_photo(
        &self,
        tenant_id: &str,
        model_id: &str,
        updated_at: &str,
        _updated_by: Option<&str>,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE equipment_model_media
                 SET is_primary = 0, updated_at = ?
                 WHERE tenant_id = ? AND model_id = ? AND is_primary = 1 AND deleted_at IS NULL",
            )
            .bind(updated_at)
            .bind(tenant_id)
            .bind(model_id),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn media_soft_delete_link(
        &self,
        tenant_id: &str,
        link_id: &str,
        deleted_at: &str,
        _updated_by: Option<&str>,
    ) -> Result<()> {
        let rows = self
            .execute(
                sqlx::query(
                    "UPDATE equipment_model_media
                     SET is_primary = 0, deleted_at = ?, updated_at = ?
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(deleted_at)
                .bind(deleted_at)
                .bind(tenant_id)
                .bind(link_id),
            )
            .await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(
                "Vínculo de mídia não encontrado para remoção.".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) async fn media_purge_soft_deleted_links(&self, tenant_id: &str) -> Result<usize> {
        let rows = self
            .execute(
                sqlx::query(
                    "DELETE FROM equipment_model_media
                     WHERE tenant_id = ? AND deleted_at IS NOT NULL",
                )
                .bind(tenant_id),
            )
            .await?;
        Ok(rows.rows_affected() as usize)
    }

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

    pub(crate) async fn media_list_active_blob_shas(
        &self,
        tenant_id: &str,
    ) -> Result<std::collections::HashSet<String>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT DISTINCT sha256
                     FROM media_blob
                     WHERE tenant_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id),
            )
            .await?;
        let mut set = std::collections::HashSet::new();
        for row in rows {
            let sha: String = row.get("sha256");
            set.insert(sha);
        }
        Ok(set)
    }

    pub(crate) async fn media_hard_delete_blob(
        &self,
        tenant_id: &str,
        blob_id: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query("DELETE FROM media_blob WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(blob_id),
        )
        .await?;
        Ok(())
    }
}
