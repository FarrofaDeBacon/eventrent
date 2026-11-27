-- Migração 0011: Unidades Físicas, Estoque Multidepósito e Rastreabilidade (Fase 2B / ADR-0018)
-- Regras:
-- - SQL portável (SQLite + PostgreSQL)
-- - Isolamento multi-tenant com chaves estrangeiras compostas (tenant_id, *_id)
-- - Índices únicos parciais com WHERE deleted_at IS NULL
-- - Livro-razão imutável append-only (stock_movement) protegido por triggers
-- - Integridade de saldo de estoque não-negativo em stock_balance

-- 1. Depósitos (warehouse)
CREATE TABLE warehouse (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    address_line1 TEXT,
    address_line2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_warehouse_tenant_code_active
    ON warehouse (tenant_id, code)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_warehouse_single_default
    ON warehouse (tenant_id)
    WHERE is_default = 1 AND deleted_at IS NULL;

-- 2. Sequência de Patrimônio por Tenant (equipment_patrimony_seq)
CREATE TABLE equipment_patrimony_seq (
    tenant_id TEXT PRIMARY KEY NOT NULL,
    last_number INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

-- 3. Unidades Físicas de Equipamento Serializado (equipment_unit)
CREATE TABLE equipment_unit (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    warehouse_id TEXT NOT NULL,
    patrimony_tag TEXT NOT NULL,
    serial_number TEXT,
    status TEXT NOT NULL DEFAULT 'available' CHECK (status IN ('available', 'maintenance', 'damaged', 'lost', 'decommissioned')),
    physical_condition TEXT NOT NULL DEFAULT 'good' CHECK (physical_condition IN ('new', 'good', 'fair', 'poor', 'damaged')),
    notes TEXT,
    acquisition_date TEXT,
    warranty_expires_at TEXT,
    acquisition_cost_cents INTEGER CHECK (acquisition_cost_cents IS NULL OR acquisition_cost_cents >= 0),
    supplier_id TEXT,
    invoice_number TEXT,
    origin TEXT NOT NULL DEFAULT 'own' CHECK (origin IN ('own', 'third_party')),
    return_by TEXT,
    usage_hours INTEGER NOT NULL DEFAULT 0 CHECK (usage_hours >= 0),
    location_note TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, equipment_model_id) REFERENCES equipment_model(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, warehouse_id) REFERENCES warehouse(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, supplier_id) REFERENCES customer(tenant_id, id) ON DELETE SET NULL,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_equipment_unit_tenant_patrimony_active
    ON equipment_unit (tenant_id, patrimony_tag)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_equipment_unit_tenant_model_serial_active
    ON equipment_unit (tenant_id, equipment_model_id, serial_number)
    WHERE serial_number IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX idx_equipment_unit_tenant_model_status
    ON equipment_unit (tenant_id, equipment_model_id, status)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_equipment_unit_tenant_warehouse
    ON equipment_unit (tenant_id, warehouse_id)
    WHERE deleted_at IS NULL;

-- 4. Saldo de Estoque Agregado por Depósito (stock_balance)
CREATE TABLE stock_balance (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    warehouse_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    quantity_on_hand INTEGER NOT NULL DEFAULT 0,
    quantity_maintenance INTEGER NOT NULL DEFAULT 0,
    min_quantity INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, warehouse_id) REFERENCES warehouse(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, equipment_model_id) REFERENCES equipment_model(tenant_id, id) ON DELETE RESTRICT,
    UNIQUE (tenant_id, id),
    CONSTRAINT chk_stock_balance_positive CHECK (
        quantity_on_hand >= 0 AND
        quantity_maintenance >= 0 AND
        min_quantity >= 0
    )
);

CREATE UNIQUE INDEX idx_stock_balance_warehouse_model
    ON stock_balance (tenant_id, warehouse_id, equipment_model_id);

-- 5. Livro-Razão Imutável de Movimentações de Estoque (stock_movement)
CREATE TABLE stock_movement (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    equipment_unit_id TEXT,
    from_warehouse_id TEXT,
    to_warehouse_id TEXT,
    movement_type TEXT NOT NULL CHECK (movement_type IN (
        'purchase_in',
        'return_in',
        'transfer_out',
        'transfer_in',
        'status_change',
        'loss_out',
        'decommission_out',
        'adjustment'
    )),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    reason TEXT NOT NULL,
    reference_id TEXT,
    performed_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, equipment_model_id) REFERENCES equipment_model(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, equipment_unit_id) REFERENCES equipment_unit(tenant_id, id) ON DELETE SET NULL,
    FOREIGN KEY (tenant_id, from_warehouse_id) REFERENCES warehouse(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, to_warehouse_id) REFERENCES warehouse(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (performed_by_user_id) REFERENCES app_user(id) ON DELETE RESTRICT,
    UNIQUE (tenant_id, id)
);

CREATE INDEX idx_stock_movement_tenant_model_created
    ON stock_movement (tenant_id, equipment_model_id, created_at);

CREATE INDEX idx_stock_movement_tenant_unit
    ON stock_movement (tenant_id, equipment_unit_id);

-- Triggers de Imutabilidade (Append-Only) para stock_movement
CREATE TRIGGER trg_stock_movement_no_update
BEFORE UPDATE ON stock_movement
BEGIN
    SELECT RAISE(ABORT, 'Operação proibida: stock_movement é imutável (append-only)');
END;

CREATE TRIGGER trg_stock_movement_no_delete
BEFORE DELETE ON stock_movement
BEGIN
    SELECT RAISE(ABORT, 'Operação proibida: registros de stock_movement não podem ser excluídos');
END;
