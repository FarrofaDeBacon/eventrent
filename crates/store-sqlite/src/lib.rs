use async_trait::async_trait;
use chrono::Utc;
use eventrent_core::{
    run_in_tx, AuditLogEntry, AuditStore, BrandingStore, CoreError, Result, Role, RoleStore,
    RoleWithPermissions, StoreBundle, Tenant, TenantBranding, TenantStore, TransactionManager,
    TransactionPolicy, TxStoreBundle, UpdateBrandingInput, UserRecord, UserStore, UserWithRole,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub type DbPool = Pool<Sqlite>;

fn map_sqlx_err(err: sqlx::Error) -> CoreError {
    match &err {
        sqlx::Error::Database(db_err) => {
            if let Some(code) = db_err.code() {
                if code == "5" || code == "6" || code == "261" || code == "517" {
                    return CoreError::TransientConflict(format!(
                        "SQLite ocupado ou bloqueado (código {code}): {}",
                        db_err.message()
                    ));
                }
            }
            let msg = db_err.message();
            let lower = msg.to_lowercase();
            if lower.contains("busy") || lower.contains("locked") {
                return CoreError::TransientConflict(format!(
                    "SQLite conflito de concorrência: {msg}"
                ));
            }
            CoreError::Database(err.to_string())
        }
        sqlx::Error::PoolTimedOut => {
            CoreError::TransientConflict("Timeout ao adquirir conexão do pool".to_string())
        }
        _ => CoreError::Database(err.to_string()),
    }
}

/// Inicializa o banco de dados SQLite em arquivo usando `SqliteConnectOptions::new().filename(path)`,
/// garantindo compatibilidade com caminhos contendo espaços e acentuação no Windows, macOS e Linux.
pub async fn init_db(db_path: &Path) -> Result<DbPool> {
    let connection_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await
        .map_err(map_sqlx_err)?;

    run_migrations(&pool).await?;

    Ok(pool)
}

/// Inicializa um banco SQLite em memória isolado para testes automatizados.
pub async fn init_memory_db() -> Result<DbPool> {
    let connection_options = SqliteConnectOptions::from_str("sqlite::memory:")
        .map_err(map_sqlx_err)?
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connection_options)
        .await
        .map_err(map_sqlx_err)?;

    run_migrations(&pool).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| CoreError::Migration(e.to_string()))?;
    Ok(())
}

#[derive(Clone)]
pub enum SqliteExecutor {
    Pool(DbPool),
    Tx(Arc<tokio::sync::Mutex<Option<sqlx::Transaction<'static, Sqlite>>>>),
}

impl SqliteExecutor {
    pub async fn fetch_optional<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<Option<sqlx::sqlite::SqliteRow>> {
        match self {
            Self::Pool(pool) => query.fetch_optional(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.fetch_optional(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }

    pub async fn fetch_one<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::sqlite::SqliteRow> {
        match self {
            Self::Pool(pool) => query.fetch_one(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.fetch_one(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }

    pub async fn fetch_all<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        match self {
            Self::Pool(pool) => query.fetch_all(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.fetch_all(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }

    pub async fn execute<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::sqlite::SqliteQueryResult> {
        match self {
            Self::Pool(pool) => query.execute(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.execute(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }
}

#[derive(Clone)]
pub struct SqliteTxManager {
    pub pool: DbPool,
}

#[async_trait]
impl TransactionManager for SqliteTxManager {
    async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>> {
        let tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(map_sqlx_err)?;
        let tx_mutex = Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let store = SqliteStore {
            executor: SqliteExecutor::Tx(tx_mutex.clone()),
        };
        let arc = Arc::new(store);
        Ok(Box::new(SqliteTxStoreBundle {
            tx: tx_mutex,
            tenants: arc.clone(),
            branding: arc.clone(),
            roles: arc.clone(),
            users: arc.clone(),
            audit: arc,
        }))
    }
}

pub struct SqliteTxStoreBundle {
    pub(crate) tx: Arc<tokio::sync::Mutex<Option<sqlx::Transaction<'static, Sqlite>>>>,
    pub tenants: Arc<dyn TenantStore>,
    pub branding: Arc<dyn BrandingStore>,
    pub roles: Arc<dyn RoleStore>,
    pub users: Arc<dyn UserStore>,
    pub audit: Arc<dyn AuditStore>,
}

#[async_trait]
impl TxStoreBundle for SqliteTxStoreBundle {
    fn tenants(&self) -> &dyn TenantStore {
        self.tenants.as_ref()
    }

    fn branding(&self) -> &dyn BrandingStore {
        self.branding.as_ref()
    }

    fn roles(&self) -> &dyn RoleStore {
        self.roles.as_ref()
    }

    fn users(&self) -> &dyn UserStore {
        self.users.as_ref()
    }

    fn audit(&self) -> &dyn AuditStore {
        self.audit.as_ref()
    }

    async fn commit(self: Box<Self>) -> Result<()> {
        let mut guard = self.tx.lock().await;
        if let Some(tx) = guard.take() {
            tx.commit().await.map_err(map_sqlx_err)?;
        }
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<()> {
        let mut guard = self.tx.lock().await;
        if let Some(tx) = guard.take() {
            tx.rollback().await.map_err(map_sqlx_err)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct SqliteStore {
    executor: SqliteExecutor,
}

impl SqliteStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            executor: SqliteExecutor::Pool(pool),
        }
    }

    pub fn from_executor(executor: SqliteExecutor) -> Self {
        Self { executor }
    }

    pub fn pool(&self) -> Option<&DbPool> {
        match &self.executor {
            SqliteExecutor::Pool(p) => Some(p),
            SqliteExecutor::Tx(_) => None,
        }
    }

    pub fn executor(&self) -> &SqliteExecutor {
        &self.executor
    }

    pub fn into_bundle(self) -> StoreBundle {
        let tx_manager: Option<Arc<dyn TransactionManager>> = match &self.executor {
            SqliteExecutor::Pool(pool) => Some(Arc::new(SqliteTxManager { pool: pool.clone() })),
            SqliteExecutor::Tx(_) => None,
        };
        let arc = Arc::new(self);
        StoreBundle {
            tenants: arc.clone(),
            branding: arc.clone(),
            roles: arc.clone(),
            users: arc.clone(),
            audit: arc,
            tx_manager,
        }
    }

    pub async fn fetch_optional<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<Option<sqlx::sqlite::SqliteRow>> {
        self.executor.fetch_optional(query).await
    }

    pub async fn fetch_one<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::sqlite::SqliteRow> {
        self.executor.fetch_one(query).await
    }

    pub async fn fetch_all<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        self.executor.fetch_all(query).await
    }

    pub async fn execute<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::sqlite::SqliteQueryResult> {
        self.executor.execute(query).await
    }

    async fn load_permissions_for_role(&self, role_id: &str) -> Result<Vec<String>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT permission
                    FROM role_permission
                    WHERE role_id = ? AND deleted_at IS NULL
                    ORDER BY permission ASC
                    "#,
                )
                .bind(role_id),
            )
            .await?;

        Ok(rows.into_iter().map(|r| r.get("permission")).collect())
    }
}

#[async_trait]
impl TenantStore for SqliteStore {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, slug, document_number,
                           created_at, updated_at, deleted_at, created_by
                    FROM tenant
                    WHERE id = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(id),
            )
            .await?;

        Ok(row.map(|r| Tenant {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            name: r.get("name"),
            slug: r.get("slug"),
            document_number: r.get("document_number"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        }))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, slug, document_number,
                           created_at, updated_at, deleted_at, created_by
                    FROM tenant
                    WHERE slug = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(slug),
            )
            .await?;

        Ok(row.map(|r| Tenant {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            name: r.get("name"),
            slug: r.get("slug"),
            document_number: r.get("document_number"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        }))
    }

    async fn create(&self, tenant: &Tenant) -> Result<Tenant> {
        self.execute(
            sqlx::query(
                r#"
                INSERT INTO tenant (
                    id, tenant_id, name, slug, document_number,
                    created_at, updated_at, deleted_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&tenant.id)
            .bind(&tenant.tenant_id)
            .bind(&tenant.name)
            .bind(&tenant.slug)
            .bind(&tenant.document_number)
            .bind(&tenant.created_at)
            .bind(&tenant.updated_at)
            .bind(&tenant.deleted_at)
            .bind(&tenant.created_by),
        )
        .await?;

        Ok(tenant.clone())
    }

    async fn update_name(&self, id: &str, name: &str, updated_at: &str) -> Result<Tenant> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE tenant
                SET name = ?, updated_at = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(name)
            .bind(updated_at)
            .bind(id),
        )
        .await?;

        TenantStore::find_by_id(self, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Tenant {id}")))
    }
}

#[async_trait]
impl BrandingStore for SqliteStore {
    async fn get_by_tenant_id(&self, tenant_id: &str) -> Result<Option<TenantBranding>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, company_name, primary_color, secondary_color,
                           accent_color, logo_url, favicon_url, created_at, updated_at,
                           deleted_at, created_by
                    FROM tenant_branding
                    WHERE tenant_id = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id),
            )
            .await?;

        Ok(row.map(|r| TenantBranding {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            company_name: r.get("company_name"),
            primary_color: r.get("primary_color"),
            secondary_color: r.get("secondary_color"),
            accent_color: r.get("accent_color"),
            logo_url: r.get("logo_url"),
            favicon_url: r.get("favicon_url"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        }))
    }

    async fn save(&self, tenant_id: &str, input: &UpdateBrandingInput) -> Result<TenantBranding> {
        let existing = self.get_by_tenant_id(tenant_id).await?;
        let now = Utc::now().to_rfc3339();

        if let Some(mut branding) = existing {
            self.execute(
                sqlx::query(
                    r#"
                    UPDATE tenant_branding
                    SET company_name = ?, primary_color = ?, secondary_color = ?,
                        accent_color = ?, logo_url = ?, updated_at = ?
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(&input.company_name)
                .bind(&input.primary_color)
                .bind(&input.secondary_color)
                .bind(&input.accent_color)
                .bind(&input.logo_url)
                .bind(&now)
                .bind(&branding.id),
            )
            .await?;

            branding.company_name = input.company_name.clone();
            branding.primary_color = input.primary_color.clone();
            branding.secondary_color = input.secondary_color.clone();
            branding.accent_color = input.accent_color.clone();
            branding.logo_url = input.logo_url.clone();
            branding.updated_at = now;

            Ok(branding)
        } else {
            let new_id = Uuid::now_v7().to_string();

            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO tenant_branding (
                        id, tenant_id, company_name, primary_color, secondary_color,
                        accent_color, logo_url, favicon_url, created_at, updated_at,
                        deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, NULL, NULL)
                    "#,
                )
                .bind(&new_id)
                .bind(tenant_id)
                .bind(&input.company_name)
                .bind(&input.primary_color)
                .bind(&input.secondary_color)
                .bind(&input.accent_color)
                .bind(&input.logo_url)
                .bind(&now)
                .bind(&now),
            )
            .await?;

            self.get_by_tenant_id(tenant_id)
                .await?
                .ok_or_else(|| CoreError::NotFound(format!("Tenant branding for {tenant_id}")))
        }
    }
}

#[async_trait]
impl RoleStore for SqliteStore {
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<RoleWithPermissions>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, slug, description, is_system,
                           created_at, updated_at, deleted_at, created_by
                    FROM role
                    WHERE tenant_id = ? AND deleted_at IS NULL
                    ORDER BY is_system DESC, name ASC
                    "#,
                )
                .bind(tenant_id),
            )
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let role_id: String = r.get("id");
            let permissions = self.load_permissions_for_role(&role_id).await?;
            let is_system_int: i64 = r.get("is_system");
            out.push(RoleWithPermissions {
                id: role_id,
                tenant_id: r.get("tenant_id"),
                name: r.get("name"),
                slug: r.get("slug"),
                description: r.get("description"),
                is_system: is_system_int != 0,
                permissions,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                deleted_at: r.get("deleted_at"),
                created_by: r.get("created_by"),
            });
        }

        Ok(out)
    }

    async fn find_by_id(
        &self,
        tenant_id: &str,
        role_id: &str,
    ) -> Result<Option<RoleWithPermissions>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, slug, description, is_system,
                           created_at, updated_at, deleted_at, created_by
                    FROM role
                    WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(role_id),
            )
            .await?;

        let Some(r) = row else {
            return Ok(None);
        };

        let id: String = r.get("id");
        let permissions = self.load_permissions_for_role(&id).await?;
        let is_system_int: i64 = r.get("is_system");

        Ok(Some(RoleWithPermissions {
            id,
            tenant_id: r.get("tenant_id"),
            name: r.get("name"),
            slug: r.get("slug"),
            description: r.get("description"),
            is_system: is_system_int != 0,
            permissions,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        }))
    }

    async fn find_by_slug(
        &self,
        tenant_id: &str,
        slug: &str,
    ) -> Result<Option<RoleWithPermissions>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, slug, description, is_system,
                           created_at, updated_at, deleted_at, created_by
                    FROM role
                    WHERE tenant_id = ? AND slug = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(slug),
            )
            .await?;

        let Some(r) = row else {
            return Ok(None);
        };

        let id: String = r.get("id");
        let permissions = self.load_permissions_for_role(&id).await?;
        let is_system_int: i64 = r.get("is_system");

        Ok(Some(RoleWithPermissions {
            id,
            tenant_id: r.get("tenant_id"),
            name: r.get("name"),
            slug: r.get("slug"),
            description: r.get("description"),
            is_system: is_system_int != 0,
            permissions,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        }))
    }

    async fn create(&self, role: &Role, permissions: &[String]) -> Result<RoleWithPermissions> {
        match &self.executor {
            SqliteExecutor::Pool(pool) => {
                let tx_mgr = SqliteTxManager { pool: pool.clone() };
                let role = role.clone();
                let permissions = permissions.to_vec();
                run_in_tx(&tx_mgr, &TransactionPolicy::default(), |stores| {
                    let role = role.clone();
                    let permissions = permissions.clone();
                    Box::pin(async move { stores.roles().create(&role, &permissions).await })
                })
                .await
            }
            SqliteExecutor::Tx(_) => {
                self.execute(
                    sqlx::query(
                        r#"
                        INSERT INTO role (
                            id, tenant_id, name, slug, description, is_system,
                            created_at, updated_at, deleted_at, created_by
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&role.id)
                    .bind(&role.tenant_id)
                    .bind(&role.name)
                    .bind(&role.slug)
                    .bind(&role.description)
                    .bind(if role.is_system { 1 } else { 0 })
                    .bind(&role.created_at)
                    .bind(&role.updated_at)
                    .bind(&role.deleted_at)
                    .bind(&role.created_by),
                )
                .await?;

                for perm in permissions {
                    let perm_id = Uuid::now_v7().to_string();
                    self.execute(
                        sqlx::query(
                            r#"
                            INSERT INTO role_permission (
                                id, tenant_id, role_id, permission,
                                created_at, updated_at, deleted_at, created_by
                            ) VALUES (?, ?, ?, ?, ?, ?, NULL, ?)
                            "#,
                        )
                        .bind(&perm_id)
                        .bind(&role.tenant_id)
                        .bind(&role.id)
                        .bind(perm)
                        .bind(&role.created_at)
                        .bind(&role.updated_at)
                        .bind(&role.created_by),
                    )
                    .await?;
                }

                Ok(RoleWithPermissions {
                    id: role.id.clone(),
                    tenant_id: role.tenant_id.clone(),
                    name: role.name.clone(),
                    slug: role.slug.clone(),
                    description: role.description.clone(),
                    is_system: role.is_system,
                    permissions: permissions.to_vec(),
                    created_at: role.created_at.clone(),
                    updated_at: role.updated_at.clone(),
                    deleted_at: role.deleted_at.clone(),
                    created_by: role.created_by.clone(),
                })
            }
        }
    }

    async fn set_permissions(
        &self,
        tenant_id: &str,
        role_id: &str,
        permissions: &[String],
        updated_at: &str,
    ) -> Result<()> {
        match &self.executor {
            SqliteExecutor::Pool(pool) => {
                let tx_mgr = SqliteTxManager { pool: pool.clone() };
                let tenant_id = tenant_id.to_string();
                let role_id = role_id.to_string();
                let permissions = permissions.to_vec();
                let updated_at = updated_at.to_string();
                run_in_tx(&tx_mgr, &TransactionPolicy::default(), |stores| {
                    let tenant_id = tenant_id.clone();
                    let role_id = role_id.clone();
                    let permissions = permissions.clone();
                    let updated_at = updated_at.clone();
                    Box::pin(async move {
                        stores
                            .roles()
                            .set_permissions(&tenant_id, &role_id, &permissions, &updated_at)
                            .await
                    })
                })
                .await
            }
            SqliteExecutor::Tx(_) => {
                self.execute(
                    sqlx::query(
                        r#"
                        UPDATE role_permission
                        SET deleted_at = ?, updated_at = ?
                        WHERE tenant_id = ? AND role_id = ? AND deleted_at IS NULL
                        "#,
                    )
                    .bind(updated_at)
                    .bind(updated_at)
                    .bind(tenant_id)
                    .bind(role_id),
                )
                .await?;

                for perm in permissions {
                    let perm_id = Uuid::now_v7().to_string();
                    self.execute(
                        sqlx::query(
                            r#"
                            INSERT INTO role_permission (
                                id, tenant_id, role_id, permission,
                                created_at, updated_at, deleted_at, created_by
                            ) VALUES (?, ?, ?, ?, ?, ?, NULL, NULL)
                            "#,
                        )
                        .bind(&perm_id)
                        .bind(tenant_id)
                        .bind(role_id)
                        .bind(perm)
                        .bind(updated_at)
                        .bind(updated_at),
                    )
                    .await?;
                }

                Ok(())
            }
        }
    }
}

#[async_trait]
impl UserStore for SqliteStore {
    async fn find_by_id(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserRecord>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, email, password_hash, recovery_key_hash,
                           role_id, active, last_login_at, failed_attempts, locked_until,
                           recovery_failed_attempts, recovery_locked_until,
                           created_at, updated_at, deleted_at, created_by
                    FROM app_user
                    WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(user_id),
            )
            .await?;

        Ok(row.map(|r| {
            let active_int: i64 = r.get("active");
            UserRecord {
                id: r.get("id"),
                tenant_id: r.get("tenant_id"),
                name: r.get("name"),
                email: r.get("email"),
                password_hash: r.get("password_hash"),
                recovery_key_hash: r.get("recovery_key_hash"),
                role_id: r.get("role_id"),
                active: active_int != 0,
                last_login_at: r.get("last_login_at"),
                failed_attempts: r.get("failed_attempts"),
                locked_until: r.get("locked_until"),
                recovery_failed_attempts: r.get("recovery_failed_attempts"),
                recovery_locked_until: r.get("recovery_locked_until"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                deleted_at: r.get("deleted_at"),
                created_by: r.get("created_by"),
            }
        }))
    }

    async fn find_by_email(&self, tenant_id: &str, email: &str) -> Result<Option<UserRecord>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, email, password_hash, recovery_key_hash,
                           role_id, active, last_login_at, failed_attempts, locked_until,
                           recovery_failed_attempts, recovery_locked_until,
                           created_at, updated_at, deleted_at, created_by
                    FROM app_user
                    WHERE tenant_id = ? AND email = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(email),
            )
            .await?;

        Ok(row.map(|r| {
            let active_int: i64 = r.get("active");
            UserRecord {
                id: r.get("id"),
                tenant_id: r.get("tenant_id"),
                name: r.get("name"),
                email: r.get("email"),
                password_hash: r.get("password_hash"),
                recovery_key_hash: r.get("recovery_key_hash"),
                role_id: r.get("role_id"),
                active: active_int != 0,
                last_login_at: r.get("last_login_at"),
                failed_attempts: r.get("failed_attempts"),
                locked_until: r.get("locked_until"),
                recovery_failed_attempts: r.get("recovery_failed_attempts"),
                recovery_locked_until: r.get("recovery_locked_until"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                deleted_at: r.get("deleted_at"),
                created_by: r.get("created_by"),
            }
        }))
    }

    async fn find_with_role(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserWithRole>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT u.id, u.tenant_id, u.name, u.email, u.role_id,
                           r.name AS role_name, r.slug AS role_slug,
                           u.active, u.last_login_at, u.failed_attempts, u.locked_until,
                           u.created_at, u.updated_at, u.deleted_at, u.created_by
                    FROM app_user u
                    INNER JOIN role r ON r.id = u.role_id AND r.deleted_at IS NULL
                    WHERE u.tenant_id = ? AND u.id = ? AND u.deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(user_id),
            )
            .await?;

        let Some(r) = row else {
            return Ok(None);
        };

        let role_id: String = r.get("role_id");
        let permissions = self.load_permissions_for_role(&role_id).await?;
        let active_int: i64 = r.get("active");

        Ok(Some(UserWithRole {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            name: r.get("name"),
            email: r.get("email"),
            role_id,
            role_name: r.get("role_name"),
            role_slug: r.get("role_slug"),
            permissions,
            active: active_int != 0,
            last_login_at: r.get("last_login_at"),
            failed_attempts: r.get("failed_attempts"),
            locked_until: r.get("locked_until"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        }))
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<UserWithRole>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT u.id, u.tenant_id, u.name, u.email, u.role_id,
                           r.name AS role_name, r.slug AS role_slug,
                           u.active, u.last_login_at, u.failed_attempts, u.locked_until,
                           u.created_at, u.updated_at, u.deleted_at, u.created_by
                    FROM app_user u
                    INNER JOIN role r ON r.id = u.role_id AND r.deleted_at IS NULL
                    WHERE u.tenant_id = ? AND u.deleted_at IS NULL
                    ORDER BY u.created_at ASC
                    "#,
                )
                .bind(tenant_id),
            )
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let role_id: String = r.get("role_id");
            let permissions = self.load_permissions_for_role(&role_id).await?;
            let active_int: i64 = r.get("active");
            out.push(UserWithRole {
                id: r.get("id"),
                tenant_id: r.get("tenant_id"),
                name: r.get("name"),
                email: r.get("email"),
                role_id,
                role_name: r.get("role_name"),
                role_slug: r.get("role_slug"),
                permissions,
                active: active_int != 0,
                last_login_at: r.get("last_login_at"),
                failed_attempts: r.get("failed_attempts"),
                locked_until: r.get("locked_until"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                deleted_at: r.get("deleted_at"),
                created_by: r.get("created_by"),
            });
        }

        Ok(out)
    }

    async fn count_active_by_role_slug(&self, tenant_id: &str, role_slug: &str) -> Result<i64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    r#"
                    SELECT COUNT(*) AS cnt
                    FROM app_user u
                    INNER JOIN role r ON r.id = u.role_id AND r.deleted_at IS NULL
                    WHERE u.tenant_id = ?
                      AND u.deleted_at IS NULL
                      AND u.active = 1
                      AND r.slug = ?
                    "#,
                )
                .bind(tenant_id)
                .bind(role_slug),
            )
            .await?;

        Ok(row.get("cnt"))
    }

    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    r#"
                    SELECT COUNT(*) AS cnt
                    FROM app_user
                    WHERE tenant_id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(tenant_id),
            )
            .await?;

        Ok(row.get("cnt"))
    }

    async fn create(&self, user: &UserRecord) -> Result<UserWithRole> {
        self.execute(
            sqlx::query(
                r#"
                INSERT INTO app_user (
                    id, tenant_id, name, email, password_hash, recovery_key_hash,
                    role_id, active, last_login_at, failed_attempts, locked_until,
                    recovery_failed_attempts, recovery_locked_until,
                    created_at, updated_at, deleted_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&user.id)
            .bind(&user.tenant_id)
            .bind(&user.name)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(&user.recovery_key_hash)
            .bind(&user.role_id)
            .bind(if user.active { 1 } else { 0 })
            .bind(&user.last_login_at)
            .bind(user.failed_attempts)
            .bind(&user.locked_until)
            .bind(user.recovery_failed_attempts)
            .bind(&user.recovery_locked_until)
            .bind(&user.created_at)
            .bind(&user.updated_at)
            .bind(&user.deleted_at)
            .bind(&user.created_by),
        )
        .await?;

        self.find_with_role(&user.tenant_id, &user.id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("User {}", user.id)))
    }

    async fn update(
        &self,
        tenant_id: &str,
        user_id: &str,
        name: &str,
        email: &str,
        role_id: &str,
        updated_at: &str,
    ) -> Result<UserWithRole> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET name = ?, email = ?, role_id = ?, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(name)
            .bind(email)
            .bind(role_id)
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        self.find_with_role(tenant_id, user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("User {user_id}")))
    }

    async fn set_active(
        &self,
        tenant_id: &str,
        user_id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<UserWithRole> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET active = ?, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(if active { 1 } else { 0 })
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        self.find_with_role(tenant_id, user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("User {user_id}")))
    }

    async fn update_password(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET password_hash = ?, failed_attempts = 0, locked_until = NULL, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(password_hash)
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        Ok(())
    }

    async fn record_login_success(
        &self,
        tenant_id: &str,
        user_id: &str,
        login_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET last_login_at = ?, failed_attempts = 0, locked_until = NULL, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(login_at)
            .bind(login_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        Ok(())
    }

    async fn increment_login_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        updated_at: &str,
    ) -> Result<i32> {
        let row = self
            .fetch_one(
                sqlx::query(
                    r#"
                    UPDATE app_user
                    SET failed_attempts = failed_attempts + 1, updated_at = ?
                    WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                    RETURNING failed_attempts
                    "#,
                )
                .bind(updated_at)
                .bind(tenant_id)
                .bind(user_id),
            )
            .await?;

        Ok(row.get("failed_attempts"))
    }

    async fn set_login_lockout(
        &self,
        tenant_id: &str,
        user_id: &str,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET locked_until = ?, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(locked_until)
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        Ok(())
    }

    async fn increment_recovery_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        updated_at: &str,
    ) -> Result<i32> {
        let row = self
            .fetch_one(
                sqlx::query(
                    r#"
                    UPDATE app_user
                    SET recovery_failed_attempts = recovery_failed_attempts + 1, updated_at = ?
                    WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                    RETURNING recovery_failed_attempts
                    "#,
                )
                .bind(updated_at)
                .bind(tenant_id)
                .bind(user_id),
            )
            .await?;

        Ok(row.get("recovery_failed_attempts"))
    }

    async fn set_recovery_lockout(
        &self,
        tenant_id: &str,
        user_id: &str,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET recovery_locked_until = ?, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(locked_until)
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        Ok(())
    }

    async fn update_password_and_recovery_key(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        recovery_key_hash: &str,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET password_hash = ?,
                    recovery_key_hash = ?,
                    failed_attempts = 0,
                    locked_until = NULL,
                    recovery_failed_attempts = 0,
                    recovery_locked_until = NULL,
                    updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(password_hash)
            .bind(recovery_key_hash)
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl AuditStore for SqliteStore {
    async fn append(&self, entry: &AuditLogEntry) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                INSERT INTO audit_log (
                    id, tenant_id, actor_user_id, action, entity_type,
                    entity_id, summary, created_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&entry.id)
            .bind(&entry.tenant_id)
            .bind(&entry.actor_user_id)
            .bind(&entry.action)
            .bind(&entry.entity_type)
            .bind(&entry.entity_id)
            .bind(&entry.summary)
            .bind(&entry.created_at)
            .bind(&entry.created_by),
        )
        .await?;

        Ok(())
    }

    async fn list_by_tenant(&self, tenant_id: &str, limit: usize) -> Result<Vec<AuditLogEntry>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, actor_user_id, action, entity_type,
                           entity_id, summary, created_at, created_by
                    FROM audit_log
                    WHERE tenant_id = ?
                    ORDER BY created_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(tenant_id)
                .bind(i64::try_from(limit).unwrap_or(100)),
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| AuditLogEntry {
                id: r.get("id"),
                tenant_id: r.get("tenant_id"),
                actor_user_id: r.get("actor_user_id"),
                action: r.get("action"),
                entity_type: r.get("entity_type"),
                entity_id: r.get("entity_id"),
                summary: r.get("summary"),
                created_at: r.get("created_at"),
                created_by: r.get("created_by"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eventrent_core::domain::{
        CreateUserInput, InitialSetupInput, LoginInput, LoginPolicy, OperationMode,
        RecoverPasswordInput,
    };
    use eventrent_core::services::{ensure_default_tenant, CoreServices};
    use eventrent_core::testing::{
        contract_audit_log, contract_default_seeds, contract_initial_setup_and_recovery_key,
        contract_last_owner_protection, contract_password_reset_clears_lockout,
        contract_recovery_key_rotation_and_lockout, contract_system_roles,
        contract_unique_partial_indexes, contract_user_creation_and_rbac, contract_user_lockout,
        run_store_contract_suite,
    };
    use std::sync::atomic::{AtomicBool, Ordering};

    async fn create_isolated_store() -> StoreBundle {
        let pool = init_memory_db()
            .await
            .expect("Falha ao criar banco SQLite em memória isolado");
        SqliteStore::new(pool).into_bundle()
    }

    #[tokio::test]
    async fn test_contract_default_seeds() {
        let store = create_isolated_store().await;
        contract_default_seeds(&store).await;
    }

    #[tokio::test]
    async fn test_contract_system_roles() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_contract_initial_setup_and_recovery_key() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_contract_last_owner_protection() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        let (setup, services) = contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
        contract_last_owner_protection(&services, &setup.session, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_contract_user_creation_and_rbac() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        let (setup, services) = contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
        contract_user_creation_and_rbac(&services, &setup.session, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_contract_user_lockout() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        let (setup, services) = contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
        let (_user, _session) =
            contract_user_creation_and_rbac(&services, &setup.session, &tenant_id).await;
        contract_user_lockout(&services, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_contract_password_reset_clears_lockout() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        let (setup, services) = contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
        let (user, _session) =
            contract_user_creation_and_rbac(&services, &setup.session, &tenant_id).await;
        contract_user_lockout(&services, &tenant_id).await;
        contract_password_reset_clears_lockout(&services, &setup.session, &tenant_id, &user.id)
            .await;
    }

    #[tokio::test]
    async fn test_contract_recovery_key_rotation_and_lockout() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        let (setup, services) = contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
        contract_recovery_key_rotation_and_lockout(&services, &tenant_id, &setup.recovery_key)
            .await;
    }

    #[tokio::test]
    async fn test_contract_audit_log() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        let (setup, services) = contract_initial_setup_and_recovery_key(&store, &tenant_id).await;
        let (user, _session) =
            contract_user_creation_and_rbac(&services, &setup.session, &tenant_id).await;
        contract_user_lockout(&services, &tenant_id).await;
        contract_password_reset_clears_lockout(&services, &setup.session, &tenant_id, &user.id)
            .await;
        contract_recovery_key_rotation_and_lockout(&services, &tenant_id, &setup.recovery_key)
            .await;
        contract_audit_log(&store, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_audit_log_triggers_abort_update_and_delete() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao criar banco SQLite em memória");
        let store = SqliteStore::new(pool.clone()).into_bundle();
        let tenant_id = contract_default_seeds(&store).await;

        let entry = AuditLogEntry {
            id: Uuid::now_v7().to_string(),
            tenant_id: tenant_id.clone(),
            actor_user_id: None,
            action: "test.event".to_string(),
            entity_type: "test".to_string(),
            entity_id: "123".to_string(),
            summary: "Registro original imutável".to_string(),
            created_at: Utc::now().to_rfc3339(),
            created_by: None,
        };
        store
            .audit
            .append(&entry)
            .await
            .expect("Falha ao inserir auditoria");

        // 1. Tentar fazer UPDATE direto via SQL deve falhar pelo trigger
        let update_res = sqlx::query("UPDATE audit_log SET summary = 'adulterado' WHERE id = ?")
            .bind(&entry.id)
            .execute(&pool)
            .await;
        assert!(
            update_res.is_err(),
            "Trigger de UPDATE em audit_log deve abortar a transação"
        );
        let update_err = update_res.unwrap_err().to_string();
        assert!(
            update_err.contains("audit_log é append-only: UPDATE não permitido"),
            "Erro inesperado no trigger de UPDATE: {update_err}"
        );

        // 2. Tentar fazer DELETE direto via SQL deve falhar pelo trigger
        let delete_res = sqlx::query("DELETE FROM audit_log WHERE id = ?")
            .bind(&entry.id)
            .execute(&pool)
            .await;
        assert!(
            delete_res.is_err(),
            "Trigger de DELETE em audit_log deve abortar a transação"
        );
        let delete_err = delete_res.unwrap_err().to_string();
        assert!(
            delete_err.contains("audit_log é append-only: DELETE não permitido"),
            "Erro inesperado no trigger de DELETE: {delete_err}"
        );
    }

    #[tokio::test]
    async fn test_contract_unique_partial_indexes() {
        let store = create_isolated_store().await;
        let tenant_id = contract_default_seeds(&store).await;
        contract_system_roles(&store, &tenant_id).await;
        contract_unique_partial_indexes(&store, &tenant_id).await;
    }

    #[tokio::test]
    async fn test_sqlite_store_contract_suite() {
        run_store_contract_suite(|| async {
            let pool = init_memory_db()
                .await
                .expect("Falha ao criar banco SQLite em memória");
            SqliteStore::new(pool).into_bundle()
        })
        .await;
    }

    #[tokio::test]
    async fn test_init_db_with_spaces_and_accents_in_path() {
        let temp_base = std::env::temp_dir().join(format!(
            "eventrent_test_{}_João Silva & Locação São Paulo",
            Uuid::now_v7()
        ));
        std::fs::create_dir_all(&temp_base).expect("Falha ao criar pasta temporária com acentos");

        let db_path = temp_base.join("dados de locação áudio & vídeo.db");

        let pool = init_db(&db_path)
            .await
            .expect("Falha ao inicializar banco em caminho com espaços e acentos");
        let store = SqliteStore::new(pool.clone()).into_bundle();

        let tenant_id_first =
            ensure_default_tenant(store.tenants.as_ref(), store.branding.as_ref())
                .await
                .expect("Falha ao garantir tenant padrão");
        let tenant_id_second =
            ensure_default_tenant(store.tenants.as_ref(), store.branding.as_ref())
                .await
                .expect("Segunda chamada idempotente a ensure_default_tenant falhou");

        assert_eq!(tenant_id_first, tenant_id_second);
        assert!(Uuid::parse_str(&tenant_id_first).is_ok());

        let branding = store
            .branding
            .get_by_tenant_id(&tenant_id_first)
            .await
            .expect("Falha ao ler branding no caminho com acentos")
            .expect("Branding padrão deve existir após ensure_default_tenant");

        assert_eq!(branding.company_name, "Minha empresa");
        assert!(db_path.exists());

        pool.close().await;
        let _ = std::fs::remove_dir_all(&temp_base);
    }

    struct FailingAuditStore {
        inner: Arc<dyn AuditStore>,
        should_fail: Arc<AtomicBool>,
    }

    #[async_trait]
    impl AuditStore for FailingAuditStore {
        async fn append(&self, entry: &AuditLogEntry) -> Result<()> {
            if self.should_fail.load(Ordering::SeqCst) {
                return Err(CoreError::Internal(
                    "Falha injetada no append de auditoria para teste de rollback".to_string(),
                ));
            }
            self.inner.append(entry).await
        }

        async fn list_by_tenant(
            &self,
            tenant_id: &str,
            limit: usize,
        ) -> Result<Vec<AuditLogEntry>> {
            self.inner.list_by_tenant(tenant_id, limit).await
        }
    }

    struct FailingTxManager {
        pool: DbPool,
        should_fail_audit: Arc<AtomicBool>,
    }

    #[async_trait]
    impl TransactionManager for FailingTxManager {
        async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>> {
            let tx = self
                .pool
                .begin_with("BEGIN IMMEDIATE")
                .await
                .map_err(map_sqlx_err)?;
            let tx_mutex = Arc::new(tokio::sync::Mutex::new(Some(tx)));
            let store = Arc::new(SqliteStore {
                executor: SqliteExecutor::Tx(tx_mutex.clone()),
            });
            let failing_audit = Arc::new(FailingAuditStore {
                inner: store.clone(),
                should_fail: self.should_fail_audit.clone(),
            });

            Ok(Box::new(SqliteTxStoreBundle {
                tx: tx_mutex,
                tenants: store.clone(),
                branding: store.clone(),
                roles: store.clone(),
                users: store.clone(),
                audit: failing_audit,
            }))
        }
    }

    #[tokio::test]
    async fn test_c1_transaction_rollback_guarantee() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao inicializar banco em memória");
        let should_fail_audit = Arc::new(AtomicBool::new(false));
        let tx_manager = Arc::new(FailingTxManager {
            pool: pool.clone(),
            should_fail_audit: should_fail_audit.clone(),
        });
        let normal_store = SqliteStore::new(pool.clone());
        let bundle = StoreBundle {
            tenants: Arc::new(normal_store.clone()),
            branding: Arc::new(normal_store.clone()),
            roles: Arc::new(normal_store.clone()),
            users: Arc::new(normal_store.clone()),
            audit: Arc::new(normal_store.clone()),
            tx_manager: Some(tx_manager),
        };
        let services = CoreServices::new(bundle.clone());
        let tenant_id = ensure_default_tenant(bundle.tenants.as_ref(), bundle.branding.as_ref())
            .await
            .expect("Falha ao criar tenant padrão");

        // 1. Falha injetada no complete_initial_setup
        should_fail_audit.store(true, Ordering::SeqCst);
        let setup_input = InitialSetupInput {
            mode: OperationMode::Local,
            company_name: "Empresa Rollback LTDA".to_string(),
            owner_name: "Dono Teste".to_string(),
            owner_email: "dono.rollback@empresa.com".to_string(),
            owner_password: "SenhaSegura#2026".to_string(),
        };
        let setup_err = services
            .complete_initial_setup(&tenant_id, &setup_input, 30)
            .await;
        assert!(
            setup_err.is_err(),
            "Setup inicial deve falhar com erro injetado"
        );

        // Prova que NADA foi gravado:
        let users_count = bundle.users.count_by_tenant(&tenant_id).await.unwrap();
        assert_eq!(
            users_count, 0,
            "Nenhum usuário deve ter sido gravado após rollback"
        );

        let branding = bundle
            .branding
            .get_by_tenant_id(&tenant_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            branding.company_name, "Minha empresa",
            "Nome da empresa não deve ter sido alterado"
        );

        let audits = bundle.audit.list_by_tenant(&tenant_id, 10).await.unwrap();
        assert_eq!(
            audits.len(),
            0,
            "Nenhum registro de auditoria deve existir após rollback"
        );

        // 2. Agora permite o setup inicial ser concluído com sucesso
        should_fail_audit.store(false, Ordering::SeqCst);
        let setup_ok = services
            .complete_initial_setup(&tenant_id, &setup_input, 30)
            .await
            .expect("Setup deve suceder com auditoria habilitada");
        let session = setup_ok.session;

        // 3. Falha injetada no create_user
        should_fail_audit.store(true, Ordering::SeqCst);
        let roles = bundle.roles.list_by_tenant(&tenant_id).await.unwrap();
        let comm_role = roles.iter().find(|r| r.slug == "commercial").unwrap();

        let create_err = services
            .create_user(
                &session,
                &CreateUserInput {
                    name: "Vendedor Rollback".to_string(),
                    email: "vendedor.rollback@empresa.com".to_string(),
                    password: "SenhaVendedor#2026".to_string(),
                    role_id: comm_role.id.clone(),
                },
            )
            .await;
        assert!(
            create_err.is_err(),
            "create_user deve falhar com erro injetado"
        );

        // Prova que NADA foi gravado no create_user:
        let created_user = bundle
            .users
            .find_by_email(&tenant_id, "vendedor.rollback@empresa.com")
            .await
            .unwrap();
        assert!(
            created_user.is_none(),
            "Usuário NÃO deve existir no banco após rollback"
        );

        let audits_after_create_fail = bundle.audit.list_by_tenant(&tenant_id, 20).await.unwrap();
        assert!(
            !audits_after_create_fail
                .iter()
                .any(|a| a.action == "user.created" && a.summary.contains("Vendedor Rollback")),
            "Auditoria de user.created NÃO deve existir após rollback"
        );

        // 4. Falha injetada no recover_password
        let original_password = "SenhaSegura#2026";
        let new_password = "NovaSenhaTentada#2026";
        should_fail_audit.store(true, Ordering::SeqCst);

        let recover_err = services
            .recover_password(
                &tenant_id,
                &RecoverPasswordInput {
                    email: session.email.clone(),
                    recovery_key: setup_ok.recovery_key.clone(),
                    new_password: new_password.to_string(),
                },
            )
            .await;
        assert!(
            recover_err.is_err(),
            "recover_password deve falhar com erro injetado"
        );

        // Desativa a falha injetada para permitir verificar as credenciais normalmente
        should_fail_audit.store(false, Ordering::SeqCst);

        // Prova que NADA foi gravado no recover_password:
        let login_with_new = services
            .login(
                &tenant_id,
                &LoginInput {
                    email: session.email.clone(),
                    password: new_password.to_string(),
                },
                &LoginPolicy::default(),
                30,
            )
            .await;
        assert!(matches!(login_with_new, Err(CoreError::InvalidCredentials)));

        // Login com a senha original ainda funciona
        let login_with_original = services
            .login(
                &tenant_id,
                &LoginInput {
                    email: session.email.clone(),
                    password: original_password.to_string(),
                },
                &LoginPolicy::default(),
                30,
            )
            .await;
        assert!(
            login_with_original.is_ok(),
            "Senha original deve continuar intacta após rollback"
        );

        let audits_after_recover_fail = bundle.audit.list_by_tenant(&tenant_id, 50).await.unwrap();
        assert!(
            !audits_after_recover_fail
                .iter()
                .any(|a| a.action == "user.password_recovered"),
            "Auditoria de password_recovered NÃO deve existir após rollback"
        );
    }

    #[tokio::test]
    async fn test_c2_login_failure_persists_counter_and_lockout_despite_returning_err() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao inicializar banco em memória");
        let store = SqliteStore::new(pool).into_bundle();
        let services = CoreServices::new(store.clone());
        let tenant_id = ensure_default_tenant(store.tenants.as_ref(), store.branding.as_ref())
            .await
            .unwrap();

        let setup = services
            .complete_initial_setup(
                &tenant_id,
                &InitialSetupInput {
                    mode: OperationMode::Local,
                    company_name: "Empresa Login Test".to_string(),
                    owner_name: "Dono".to_string(),
                    owner_email: "dono.login@empresa.com".to_string(),
                    owner_password: "SenhaCerta#2026".to_string(),
                },
                30,
            )
            .await
            .unwrap();

        let policy = LoginPolicy {
            reveal_account_locked: true,
        };

        // Tentativa 1 com senha incorreta
        let err1 = services
            .login(
                &tenant_id,
                &LoginInput {
                    email: setup.session.email.clone(),
                    password: "SenhaIncorreta#1".to_string(),
                },
                &policy,
                30,
            )
            .await;
        assert!(matches!(err1, Err(CoreError::InvalidCredentials)));

        // Prova que failed_attempts foi gravado (1) e auditoria gerada
        let user = store
            .users
            .find_by_email(&tenant_id, &setup.session.email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.failed_attempts, 1);
        assert!(user.locked_until.is_none());
        let audits = store.audit.list_by_tenant(&tenant_id, 10).await.unwrap();
        assert!(audits.iter().any(|a| a.action == "auth.login_failed"));

        // Tentativas 2, 3 e 4
        for i in 2..=4 {
            let res = services
                .login(
                    &tenant_id,
                    &LoginInput {
                        email: setup.session.email.clone(),
                        password: format!("SenhaIncorreta#{i}"),
                    },
                    &policy,
                    30,
                )
                .await;
            assert!(matches!(res, Err(CoreError::InvalidCredentials)));
            let user = store
                .users
                .find_by_email(&tenant_id, &setup.session.email)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(user.failed_attempts, i);
            assert!(user.locked_until.is_none());
        }

        // Tentativa 5: atinge o limite e bloqueia a conta
        let err5 = services
            .login(
                &tenant_id,
                &LoginInput {
                    email: setup.session.email.clone(),
                    password: "SenhaIncorreta#5".to_string(),
                },
                &policy,
                30,
            )
            .await;
        assert!(matches!(err5, Err(CoreError::AccountLocked { .. })));

        let user_locked = store
            .users
            .find_by_email(&tenant_id, &setup.session.email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user_locked.failed_attempts, 5);
        assert!(
            user_locked.locked_until.is_some(),
            "locked_until deve estar preenchido"
        );

        let audits_locked = store.audit.list_by_tenant(&tenant_id, 20).await.unwrap();
        assert!(audits_locked
            .iter()
            .any(|a| a.action == "auth.account_locked"));
    }

    #[tokio::test]
    async fn test_c3_concurrency_with_multiple_connections_on_temp_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("eventrent_concurrency_{}", Uuid::now_v7()));
        std::fs::create_dir_all(&temp_dir).expect("Falha ao criar pasta temporária");
        let db_path = temp_dir.join("concurrency_test.db");

        let pool = init_db(&db_path)
            .await
            .expect("Falha ao inicializar banco em arquivo com múltiplas conexões");
        let store = SqliteStore::new(pool.clone()).into_bundle();
        let services = Arc::new(CoreServices::new(store.clone()));

        let tenant_id = ensure_default_tenant(store.tenants.as_ref(), store.branding.as_ref())
            .await
            .unwrap();

        let setup = services
            .complete_initial_setup(
                &tenant_id,
                &InitialSetupInput {
                    mode: OperationMode::Local,
                    company_name: "Locadora Concorrente".to_string(),
                    owner_name: "Proprietário".to_string(),
                    owner_email: "dono@concorrencia.com".to_string(),
                    owner_password: "SenhaForte#2026".to_string(),
                },
                30,
            )
            .await
            .unwrap();

        let num_tasks = 10;
        let mut handles = Vec::with_capacity(num_tasks);

        for i in 0..num_tasks {
            let services = services.clone();
            let tenant_id = tenant_id.clone();
            let email = setup.session.email.clone();
            let wrong_pw = format!("SenhaErradaConcorrente#{i}");

            handles.push(tokio::spawn(async move {
                services
                    .login(
                        &tenant_id,
                        &LoginInput {
                            email,
                            password: wrong_pw,
                        },
                        &LoginPolicy {
                            reveal_account_locked: false,
                        },
                        30,
                    )
                    .await
            }));
        }

        for handle in handles {
            let res = handle.await.expect("Task tokio em pânico");
            assert!(
                matches!(res, Err(CoreError::InvalidCredentials)),
                "Todas as tarefas concorrentes devem retornar InvalidCredentials sem erro de lock/busy: {:?}",
                res
            );
        }

        let user = store
            .users
            .find_by_email(&tenant_id, &setup.session.email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            user.failed_attempts, 10,
            "Contagem exata de 10 tentativas falhas esperada sem perda por concorrência"
        );
        assert!(
            user.locked_until.is_some(),
            "Usuário deve ter sido bloqueado"
        );

        let audits = store.audit.list_by_tenant(&tenant_id, 50).await.unwrap();
        let failure_audits: Vec<_> = audits
            .iter()
            .filter(|a| a.action == "auth.login_failed" || a.action == "auth.account_locked")
            .collect();
        assert_eq!(
            failure_audits.len(),
            10,
            "Auditoria deve conter exatamente 10 registros de falha/bloqueio persistidos"
        );

        pool.close().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_c5_contract_suite_running_inside_transaction() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao criar banco SQLite em memória");

        let tx = pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .expect("Falha ao iniciar BEGIN IMMEDIATE");
        let tx_mutex = Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let store = Arc::new(SqliteStore {
            executor: SqliteExecutor::Tx(tx_mutex.clone()),
        });

        let bundle = StoreBundle {
            tenants: store.clone(),
            branding: store.clone(),
            roles: store.clone(),
            users: store.clone(),
            audit: store.clone(),
            tx_manager: None,
        };

        let tenant_id = contract_default_seeds(&bundle).await;
        contract_system_roles(&bundle, &tenant_id).await;

        let (setup_result, services) =
            contract_initial_setup_and_recovery_key(&bundle, &tenant_id).await;

        contract_last_owner_protection(&services, &setup_result.session, &tenant_id).await;

        let (commercial_user, _commercial_session) =
            contract_user_creation_and_rbac(&services, &setup_result.session, &tenant_id).await;

        contract_user_lockout(&services, &tenant_id).await;

        contract_password_reset_clears_lockout(
            &services,
            &setup_result.session,
            &tenant_id,
            &commercial_user.id,
        )
        .await;

        contract_recovery_key_rotation_and_lockout(
            &services,
            &tenant_id,
            &setup_result.recovery_key,
        )
        .await;

        contract_audit_log(&bundle, &tenant_id).await;

        contract_unique_partial_indexes(&bundle, &tenant_id).await;

        let mut guard = tx_mutex.lock().await;
        let tx_handle = guard.take().expect("Transação deve estar aberta");
        tx_handle
            .commit()
            .await
            .expect("Falha ao comitar transação da suíte de contrato");
    }
}
