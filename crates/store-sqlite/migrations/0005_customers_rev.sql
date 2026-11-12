-- Migração 0005: Reconstrução estrutural das tabelas de clientes (ADR-0011)
-- Preserva eventuais dados existentes seguindo o procedimento seguro do SQLite com FKs ativas:
-- 1) Copia dados para tabelas temporárias de migração
-- 2) Remove tabelas antigas em ordem filha -> pai
-- 3) Cria as novas tabelas com tenant_id, created_by, document_type, document_number nullable, active e country
-- 4) Restaura os dados e remove as tabelas temporárias
-- 5) Cria índices finais (removendo idx_customer_tenant_search não utilizado por LIKE '%termo%')

CREATE TABLE _mig_customer (
    id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    person_type TEXT NOT NULL,
    name TEXT NOT NULL,
    trade_name TEXT,
    document_type TEXT NOT NULL,
    document_number TEXT,
    state_registration TEXT,
    municipal_registration TEXT,
    notes TEXT,
    active INTEGER NOT NULL,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT
);

INSERT INTO _mig_customer (
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, search_text, name_sort, created_at, updated_at, deleted_at, created_by
)
SELECT
    id,
    tenant_id,
    person_type,
    name,
    trade_name,
    CASE
        WHEN document_number IS NULL OR trim(document_number) = '' THEN 'none'
        WHEN length(trim(document_number)) = 11 THEN 'cpf'
        WHEN length(trim(document_number)) = 14 THEN 'cnpj'
        ELSE 'foreign'
    END,
    NULLIF(trim(document_number), ''),
    state_registration,
    municipal_registration,
    notes,
    1,
    search_text,
    name_sort,
    created_at,
    updated_at,
    deleted_at,
    created_by
FROM customer;

CREATE TABLE _mig_customer_contact (
    id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    role TEXT,
    is_primary INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT
);

INSERT INTO _mig_customer_contact (
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    cc.id,
    c.tenant_id,
    cc.customer_id,
    cc.name,
    cc.email,
    cc.phone,
    cc.role,
    cc.is_primary,
    cc.created_at,
    cc.updated_at,
    cc.deleted_at,
    c.created_by
FROM customer_contact cc
INNER JOIN customer c ON c.id = cc.customer_id;

CREATE TABLE _mig_customer_address (
    id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    label TEXT NOT NULL,
    street TEXT NOT NULL,
    number TEXT NOT NULL,
    complement TEXT,
    neighborhood TEXT NOT NULL,
    city TEXT NOT NULL,
    state TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    country TEXT NOT NULL,
    is_primary INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT
);

INSERT INTO _mig_customer_address (
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    ca.id,
    c.tenant_id,
    ca.customer_id,
    ca.label,
    ca.street,
    ca.number,
    ca.complement,
    ca.neighborhood,
    ca.city,
    ca.state,
    ca.postal_code,
    'BR',
    ca.is_primary,
    ca.created_at,
    ca.updated_at,
    ca.deleted_at,
    c.created_by
FROM customer_address ca
INNER JOIN customer c ON c.id = ca.customer_id;

DROP TABLE customer_contact;
DROP TABLE customer_address;
DROP TABLE customer;

CREATE TABLE customer (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    person_type TEXT NOT NULL CHECK (person_type IN ('individual', 'legal')),
    name TEXT NOT NULL,
    trade_name TEXT,
    document_type TEXT NOT NULL DEFAULT 'none' CHECK (document_type IN ('cpf', 'cnpj', 'foreign', 'none')),
    document_number TEXT,
    state_registration TEXT,
    municipal_registration TEXT,
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE TABLE customer_contact (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
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
    FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE TABLE customer_address (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    label TEXT NOT NULL,
    street TEXT NOT NULL,
    number TEXT NOT NULL,
    complement TEXT,
    neighborhood TEXT NOT NULL,
    city TEXT NOT NULL,
    state TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    country TEXT NOT NULL DEFAULT 'BR',
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

INSERT INTO customer (
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, search_text, name_sort, created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, search_text, name_sort, created_at, updated_at, deleted_at, created_by
FROM _mig_customer;

INSERT INTO customer_contact (
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
FROM _mig_customer_contact;

INSERT INTO customer_address (
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
FROM _mig_customer_address;

DROP TABLE _mig_customer_contact;
DROP TABLE _mig_customer_address;
DROP TABLE _mig_customer;

CREATE UNIQUE INDEX idx_customer_tenant_doc_active
    ON customer (tenant_id, document_type, document_number)
    WHERE document_number IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX idx_customer_tenant_sort
    ON customer (tenant_id, name_sort, id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_customer_contact_tenant_customer
    ON customer_contact (tenant_id, customer_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_customer_contact_primary_active
    ON customer_contact (tenant_id, customer_id)
    WHERE is_primary = 1 AND deleted_at IS NULL;

CREATE INDEX idx_customer_address_tenant_customer
    ON customer_address (tenant_id, customer_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_customer_address_primary_active
    ON customer_address (tenant_id, customer_id)
    WHERE is_primary = 1 AND deleted_at IS NULL;

