-- Migração 0008: Catálogo de equipamentos, marcas, categorias em árvore e atributos personalizáveis (Fase 2A)
-- Regras:
-- - SQL portável (SQLite + PostgreSQL)
-- - Isolamento multi-tenant com chaves estrangeiras compostas (tenant_id, *_id)
-- - Índices únicos parciais com WHERE deleted_at IS NULL
-- - Valores monetários sempre em inteiros de centavos (>= 0)
-- - Nenhum dado de negócio inserido na migração

CREATE TABLE brand (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_normalized TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_brand_name_active
    ON brand (tenant_id, name_normalized)
    WHERE deleted_at IS NULL;

CREATE TABLE equipment_category (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    parent_id TEXT,
    name TEXT NOT NULL,
    name_normalized TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, parent_id) REFERENCES equipment_category(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_equipment_category_name_active
    ON equipment_category (tenant_id, COALESCE(parent_id, ''), name_normalized)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_equipment_category_parent
    ON equipment_category (tenant_id, parent_id)
    WHERE deleted_at IS NULL;

CREATE TABLE attribute_definition (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    category_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    label TEXT NOT NULL,
    data_type TEXT NOT NULL CHECK (data_type IN ('text', 'number', 'boolean', 'select')),
    unit TEXT,
    choices_json TEXT,
    is_required INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, category_id) REFERENCES equipment_category(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_attribute_definition_key_active
    ON attribute_definition (tenant_id, category_id, field_key)
    WHERE deleted_at IS NULL;

CREATE TABLE equipment_model (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    category_id TEXT NOT NULL,
    brand_id TEXT,
    name TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    model_code TEXT,
    model_code_normalized TEXT,
    sku TEXT,
    sku_normalized TEXT,
    ean TEXT,
    ncm TEXT,
    description TEXT,
    control_mode TEXT NOT NULL CHECK (control_mode IN ('serialized', 'quantity')),
    weight_kg REAL CHECK (weight_kg IS NULL OR weight_kg > 0),
    length_cm REAL CHECK (length_cm IS NULL OR length_cm > 0),
    width_cm REAL CHECK (width_cm IS NULL OR width_cm > 0),
    height_cm REAL CHECK (height_cm IS NULL OR height_cm > 0),
    power_watts INTEGER CHECK (power_watts IS NULL OR power_watts >= 0),
    voltage TEXT CHECK (voltage IS NULL OR voltage IN ('110', '220', 'bivolt')),
    amperage REAL CHECK (amperage IS NULL OR amperage > 0),
    requires_case INTEGER NOT NULL DEFAULT 0,
    units_per_case INTEGER CHECK (units_per_case IS NULL OR units_per_case > 0),
    daily_rate_cents INTEGER CHECK (daily_rate_cents IS NULL OR daily_rate_cents >= 0),
    replacement_value_cents INTEGER CHECK (replacement_value_cents IS NULL OR replacement_value_cents >= 0),
    cost_price_cents INTEGER CHECK (cost_price_cents IS NULL OR cost_price_cents >= 0),
    active INTEGER NOT NULL DEFAULT 1,
    search_text TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, category_id) REFERENCES equipment_category(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, brand_id) REFERENCES brand(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_equipment_model_sku_active
    ON equipment_model (tenant_id, sku_normalized)
    WHERE deleted_at IS NULL AND sku_normalized IS NOT NULL;

CREATE UNIQUE INDEX idx_equipment_model_brand_code_active
    ON equipment_model (tenant_id, brand_id, model_code_normalized)
    WHERE deleted_at IS NULL AND brand_id IS NOT NULL AND model_code_normalized IS NOT NULL;

CREATE INDEX idx_equipment_model_tenant_sort
    ON equipment_model (tenant_id, name_sort, id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_equipment_model_category
    ON equipment_model (tenant_id, category_id)
    WHERE deleted_at IS NULL;

CREATE TABLE equipment_model_attribute_value (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    definition_id TEXT NOT NULL,
    value_text TEXT,
    value_number REAL,
    value_bool INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, model_id) REFERENCES equipment_model(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, definition_id) REFERENCES attribute_definition(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX idx_model_attribute_value_active
    ON equipment_model_attribute_value (tenant_id, model_id, definition_id)
    WHERE deleted_at IS NULL;
