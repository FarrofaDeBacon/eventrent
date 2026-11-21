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
