-- Migração 0007: Locais de evento, espaços, contatos e configurações do tenant (Fase 1D / ADR-0012)
-- Regras:
-- - SQL portável (SQLite + PostgreSQL)
-- - Isolamento multi-tenant com chaves estrangeiras compostas (tenant_id, venue_id)
-- - Índices únicos parciais com WHERE deleted_at IS NULL
-- - Nenhum dado inicial inserido na migração (defaults de tenant_setting garantidos por código idempotente)

CREATE TABLE venue (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    street TEXT,
    number TEXT,
    complement TEXT,
    neighborhood TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    country TEXT NOT NULL DEFAULT 'BR',
    parking_notes TEXT,
    loading_dock_notes TEXT,
    access_hours TEXT,
    internal_transfer_minutes INTEGER CHECK (internal_transfer_minutes IS NULL OR (internal_transfer_minutes >= 0 AND internal_transfer_minutes <= 240)),
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE INDEX idx_venue_tenant_sort
    ON venue (tenant_id, name_sort, id)
    WHERE deleted_at IS NULL;

CREATE TABLE venue_space (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_normalized TEXT NOT NULL,
    capacity_people INTEGER,
    area_sqm REAL CHECK (area_sqm IS NULL OR area_sqm > 0),
    ceiling_height_m REAL CHECK (ceiling_height_m IS NULL OR ceiling_height_m > 0),
    power_available TEXT,
    loading_access TEXT,
    rigging_points TEXT,
    acoustics_notes TEXT,
    setup_hours_notes TEXT,
    noise_restrictions_notes TEXT,
    load_dimensions_notes TEXT,
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, venue_id) REFERENCES venue(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE INDEX idx_venue_space_tenant_venue
    ON venue_space (tenant_id, venue_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_venue_space_name_active
    ON venue_space (tenant_id, venue_id, name_normalized)
    WHERE deleted_at IS NULL;

CREATE TABLE venue_contact (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    role TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, venue_id) REFERENCES venue(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE INDEX idx_venue_contact_tenant_venue
    ON venue_contact (tenant_id, venue_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_venue_contact_primary_active
    ON venue_contact (tenant_id, venue_id)
    WHERE is_primary = 1 AND deleted_at IS NULL;

CREATE TABLE tenant_setting (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    setting_key TEXT NOT NULL,
    setting_value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (updated_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, setting_key)
);
