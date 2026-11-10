-- Migração 0004: Cadastro de Clientes, Contatos e Endereços (Fase 1B.2)
-- Suporte a Pessoa Física (CPF) e Pessoa Jurídica (CNPJ tradicional e alfanumérico).
-- Índices parciais WHERE deleted_at IS NULL para integridade e busca eficiente com paginação por cursor.

CREATE TABLE IF NOT EXISTS customer (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    person_type TEXT NOT NULL CHECK (person_type IN ('individual', 'legal')),
    name TEXT NOT NULL,
    trade_name TEXT,
    document_number TEXT NOT NULL,
    state_registration TEXT,
    municipal_registration TEXT,
    notes TEXT,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_customer_tenant_document_active
    ON customer (tenant_id, document_number)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_customer_tenant_sort
    ON customer (tenant_id, name_sort, id)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_customer_tenant_search
    ON customer (tenant_id, search_text)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS customer_contact (
    id TEXT PRIMARY KEY NOT NULL,
    customer_id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    role TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_customer_contact_customer
    ON customer_contact (customer_id)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS customer_address (
    id TEXT PRIMARY KEY NOT NULL,
    customer_id TEXT NOT NULL,
    label TEXT NOT NULL,
    street TEXT NOT NULL,
    number TEXT NOT NULL,
    complement TEXT,
    neighborhood TEXT NOT NULL,
    city TEXT NOT NULL,
    state TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_customer_address_customer
    ON customer_address (customer_id)
    WHERE deleted_at IS NULL;

