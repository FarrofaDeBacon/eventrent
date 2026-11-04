-- Migração 0001: Criação das tabelas tenant e tenant_branding

CREATE TABLE IF NOT EXISTS tenant (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    document_number TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT
);

CREATE TABLE IF NOT EXISTS tenant_branding (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL UNIQUE,
    company_name TEXT NOT NULL,
    primary_color TEXT NOT NULL,
    secondary_color TEXT NOT NULL,
    accent_color TEXT NOT NULL,
    logo_url TEXT,
    favicon_url TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

-- Inserção do tenant inicial para operação Desktop se não existir
INSERT OR IGNORE INTO tenant (id, tenant_id, name, slug, document_number, created_at, updated_at, deleted_at, created_by)
VALUES (
    '018f0000-0000-7000-8000-000000000001',
    '018f0000-0000-7000-8000-000000000001',
    'EventRent Padrão',
    'default',
    NULL,
    '2026-01-01T00:00:00Z',
    '2026-01-01T00:00:00Z',
    NULL,
    NULL
);

INSERT OR IGNORE INTO tenant_branding (id, tenant_id, company_name, primary_color, secondary_color, accent_color, logo_url, favicon_url, created_at, updated_at, deleted_at, created_by)
VALUES (
    '018f0000-0000-7000-8000-000000000002',
    '018f0000-0000-7000-8000-000000000001',
    'EventRent Audiovisual',
    '#2563eb',
    '#475569',
    '#7c3aed',
    NULL,
    NULL,
    '2026-01-01T00:00:00Z',
    '2026-01-01T00:00:00Z',
    NULL,
    NULL
);
