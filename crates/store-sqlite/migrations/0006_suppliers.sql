-- Migração 0006: Suporte a Fornecedores no cadastro unificado de terceiros (customer)
-- Adiciona papéis (is_customer, is_supplier) com CHECK (is_customer = 1 OR is_supplier = 1),
-- índice de filtragem por papel e tabela de categorias de fornecedor com FK composta multitenant.

CREATE TABLE _mig06_customer (
    id TEXT PRIMARY KEY NOT NULL,
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

INSERT INTO _mig06_customer (
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, search_text, name_sort, created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, search_text, name_sort, created_at, updated_at, deleted_at, created_by
FROM customer;

CREATE TABLE _mig06_customer_contact (
    id TEXT PRIMARY KEY NOT NULL,
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

INSERT INTO _mig06_customer_contact (
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
FROM customer_contact;

CREATE TABLE _mig06_customer_address (
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
    country TEXT NOT NULL,
    is_primary INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT
);

INSERT INTO _mig06_customer_address (
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
FROM customer_address;

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
    is_customer INTEGER NOT NULL DEFAULT 1,
    is_supplier INTEGER NOT NULL DEFAULT 0,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    CHECK (is_customer = 1 OR is_supplier = 1),
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
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
    FOREIGN KEY (tenant_id, customer_id) REFERENCES customer(tenant_id, id) ON DELETE CASCADE,
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
    FOREIGN KEY (tenant_id, customer_id) REFERENCES customer(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE TABLE customer_supplier_category (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('sublocacao', 'manutencao', 'transporte', 'compras', 'servicos', 'outros')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, customer_id) REFERENCES customer(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

INSERT INTO customer (
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, is_customer, is_supplier, search_text, name_sort,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, person_type, name, trade_name,
    document_type, document_number, state_registration, municipal_registration,
    notes, active, 1, 0, search_text, name_sort,
    created_at, updated_at, deleted_at, created_by
FROM _mig06_customer;

INSERT INTO customer_contact (
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, customer_id, name, email, phone, role, is_primary,
    created_at, updated_at, deleted_at, created_by
FROM _mig06_customer_contact;

INSERT INTO customer_address (
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
)
SELECT
    id, tenant_id, customer_id, label, street, number, complement,
    neighborhood, city, state, postal_code, country, is_primary,
    created_at, updated_at, deleted_at, created_by
FROM _mig06_customer_address;

DROP TABLE _mig06_customer_contact;
DROP TABLE _mig06_customer_address;
DROP TABLE _mig06_customer;

CREATE UNIQUE INDEX idx_customer_tenant_doc_active
    ON customer (tenant_id, document_type, document_number)
    WHERE document_number IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX idx_customer_tenant_sort
    ON customer (tenant_id, name_sort, id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_customer_tenant_roles
    ON customer (tenant_id, is_customer, is_supplier)
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

CREATE UNIQUE INDEX idx_customer_supplier_category_active
    ON customer_supplier_category (tenant_id, customer_id, category)
    WHERE deleted_at IS NULL;
