-- Migração 0002: Controle de Acesso (RBAC), Usuários, Perfis, Permissões e Log de Auditoria
-- Regra de Unicidade: Índices únicos parciais WHERE deleted_at IS NULL em tabelas com exclusão lógica.

CREATE TABLE IF NOT EXISTS role (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    is_system INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_role_tenant_slug_active
    ON role (tenant_id, slug)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_role_tenant_name_active
    ON role (tenant_id, name)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS role_permission (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES role(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_role_permission_active
    ON role_permission (role_id, permission)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS app_user (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    recovery_key_hash TEXT,
    role_id TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    last_login_at TEXT,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TEXT,
    recovery_failed_attempts INTEGER NOT NULL DEFAULT 0,
    recovery_locked_until TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES role(id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_app_user_tenant_email_active
    ON app_user (tenant_id, email)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_app_user_role_id
    ON app_user (role_id);

-- audit_log: tabela append-only (sem updated_at nem deleted_at).
-- O trait AuditStore expõe apenas append e list_by_tenant, sem update nem delete.
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    actor_user_id TEXT,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_audit_log_tenant_created
    ON audit_log (tenant_id, created_at DESC);

-- Triggers no banco garantindo que audit_log seja estritamente append-only.
-- Qualquer tentativa de UPDATE ou DELETE é imediatamente abortada pela engine SQLite.
CREATE TRIGGER IF NOT EXISTS trg_audit_log_prevent_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log é append-only: UPDATE não permitido');
END;

CREATE TRIGGER IF NOT EXISTS trg_audit_log_prevent_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log é append-only: DELETE não permitido');
END;
