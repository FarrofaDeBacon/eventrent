-- Migração 0001: Criação das tabelas tenant e tenant_branding
-- Regra de Arquitetura: Migrações contêm apenas estrutura (DDL), nunca dados de negócio (seed).
-- Regra de Unicidade: Índices únicos parciais WHERE deleted_at IS NULL em tabelas com exclusão lógica.

CREATE TABLE IF NOT EXISTS tenant (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    document_number TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tenant_slug_active
    ON tenant (slug)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS tenant_branding (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
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

CREATE UNIQUE INDEX IF NOT EXISTS idx_tenant_branding_tenant_id_active
    ON tenant_branding (tenant_id)
    WHERE deleted_at IS NULL;
