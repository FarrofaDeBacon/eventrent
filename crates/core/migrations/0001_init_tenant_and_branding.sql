-- Migração 0001: Criação das tabelas tenant e tenant_branding (SQL portável SQLite / PostgreSQL)
-- Nota: Migrações contêm apenas estrutura (DDL), nunca dados de negócio (seed).

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
