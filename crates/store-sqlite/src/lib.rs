mod catalog;

use async_trait::async_trait;
use chrono::Utc;
use eventrent_core::{
    normalize_for_search, run_in_tx, AuditLogEntry, AuditStore, BrandingStore, CatalogStore,
    CoreError, Customer, CustomerAddress, CustomerContact, CustomerCursor, CustomerFilter,
    CustomerPage, CustomerStore, CustomerWithDetails, DocumentType, Result, Role, RoleStore,
    RoleWithPermissions, StoreBundle, Tenant, TenantBranding, TenantSetting, TenantSettingStore,
    TenantStore, TransactionManager, TransactionPolicy, TxStoreBundle, UpdateBrandingInput,
    UserRecord, UserStore, UserWithRole, Venue, VenueContact, VenueCursor, VenueFilter,
    VenueListItem, VenuePage, VenueSpace, VenueStore, VenueWithDetails,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub type DbPool = Pool<Sqlite>;

pub(crate) fn map_sqlx_err(err: sqlx::Error) -> CoreError {
    match &err {
        sqlx::Error::Database(db_err) => {
            if let Some(code) = db_err.code() {
                match code.as_ref() {
                    // SQLite BUSY (5) e códigos estendidos: 261, 517, 773
                    // SQLite LOCKED (6) e códigos estendidos: 262, 518
                    "5" | "6" | "261" | "517" | "773" | "262" | "518" => {
                        return CoreError::TransientConflict(format!(
                            "SQLite ocupado ou bloqueado (código {code}): {}",
                            db_err.message()
                        ));
                    }
                    // SQLite UNIQUE (2067) e PRIMARY KEY (1555)
                    "2067" | "1555" => {
                        return CoreError::Conflict("Registro conflitante já existe.".to_string());
                    }
                    // SQLite FOREIGN KEY (787)
                    "787" => {
                        return CoreError::Validation(
                            "Registro relacionado não encontrado ou inválido.".to_string(),
                        );
                    }
                    _ => {}
                }
            }
            // NUNCA expor texto cru do SQL para o usuário/frontend.
            // Registrar em log estruturado com tracing::error! e retornar mensagem genérica segura.
            tracing::error!(
                code = db_err.code().as_deref().unwrap_or("none"),
                message = %db_err.message(),
                "Erro interno de banco de dados SQLite não mapeado"
            );
            CoreError::Database("Erro interno de banco de dados. Operação cancelada.".to_string())
        }
        sqlx::Error::PoolTimedOut => {
            CoreError::TransientConflict("Timeout ao adquirir conexão do pool".to_string())
        }
        _ => {
            tracing::error!(error = %err, "Erro interno sqlx não mapeado");
            CoreError::Database("Erro interno de banco de dados. Operação cancelada.".to_string())
        }
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
            audit: arc.clone(),
            customers: arc.clone(),
            venues: arc.clone(),
            settings: arc.clone(),
            catalog: arc,
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
    pub customers: Arc<dyn CustomerStore>,
    pub venues: Arc<dyn VenueStore>,
    pub settings: Arc<dyn TenantSettingStore>,
    pub catalog: Arc<dyn CatalogStore>,
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

    fn customers(&self) -> &dyn CustomerStore {
        self.customers.as_ref()
    }

    fn venues(&self) -> &dyn VenueStore {
        self.venues.as_ref()
    }

    fn settings(&self) -> &dyn TenantSettingStore {
        self.settings.as_ref()
    }

    fn catalog(&self) -> &dyn CatalogStore {
        self.catalog.as_ref()
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
            audit: arc.clone(),
            customers: arc.clone(),
            venues: arc.clone(),
            settings: arc.clone(),
            catalog: arc,
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
                           accent_color, logo_url, logo_dark_url, icon_url, favicon_url,
                           created_at, updated_at, deleted_at, created_by
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
            logo_dark_url: r.get("logo_dark_url"),
            icon_url: r.get("icon_url"),
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
                        accent_color = ?, logo_url = ?, logo_dark_url = ?, icon_url = ?, updated_at = ?
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(&input.company_name)
                .bind(&input.primary_color)
                .bind(&input.secondary_color)
                .bind(&input.accent_color)
                .bind(&input.logo_url)
                .bind(&input.logo_dark_url)
                .bind(&input.icon_url)
                .bind(&now)
                .bind(&branding.id),
            )
            .await?;

            branding.company_name = input.company_name.clone();
            branding.primary_color = input.primary_color.clone();
            branding.secondary_color = input.secondary_color.clone();
            branding.accent_color = input.accent_color.clone();
            branding.logo_url = input.logo_url.clone();
            branding.logo_dark_url = input.logo_dark_url.clone();
            branding.icon_url = input.icon_url.clone();
            branding.updated_at = now;

            Ok(branding)
        } else {
            let new_id = Uuid::now_v7().to_string();

            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO tenant_branding (
                        id, tenant_id, company_name, primary_color, secondary_color,
                        accent_color, logo_url, logo_dark_url, icon_url, favicon_url,
                        created_at, updated_at, deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, NULL, NULL)
                    "#,
                )
                .bind(&new_id)
                .bind(tenant_id)
                .bind(&input.company_name)
                .bind(&input.primary_color)
                .bind(&input.secondary_color)
                .bind(&input.accent_color)
                .bind(&input.logo_url)
                .bind(&input.logo_dark_url)
                .bind(&input.icon_url)
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

fn row_to_customer(r: &sqlx::sqlite::SqliteRow) -> Result<Customer> {
    let person_type_str: String = r.get("person_type");
    let person_type = person_type_str.parse().map_err(CoreError::Database)?;
    let doc_type_str: String = r.get("document_type");
    let document_type = doc_type_str.parse().map_err(CoreError::Database)?;
    let active_int: i64 = r.get("active");
    let is_customer_int: i64 = r.get("is_customer");
    let is_supplier_int: i64 = r.get("is_supplier");
    Ok(Customer {
        id: r.get("id"),
        tenant_id: r.get("tenant_id"),
        person_type,
        name: r.get("name"),
        trade_name: r.get("trade_name"),
        document_type,
        document_number: r.get("document_number"),
        state_registration: r.get("state_registration"),
        municipal_registration: r.get("municipal_registration"),
        notes: r.get("notes"),
        active: active_int != 0,
        is_customer: is_customer_int != 0,
        is_supplier: is_supplier_int != 0,
        search_text: r.get("search_text"),
        name_sort: r.get("name_sort"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        deleted_at: r.get("deleted_at"),
        created_by: r.get("created_by"),
    })
}

#[async_trait]
impl CustomerStore for SqliteStore {
    async fn find_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<CustomerWithDetails>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, person_type, name, trade_name, document_type,
                           document_number, state_registration, municipal_registration, notes,
                           active, is_customer, is_supplier, search_text, name_sort,
                           created_at, updated_at, deleted_at, created_by
                    FROM customer
                    WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let customer = match row {
            Some(ref r) => row_to_customer(r)?,
            None => return Ok(None),
        };

        let contact_rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, customer_id, name, email, phone, role, is_primary,
                           created_at, updated_at, deleted_at, created_by
                    FROM customer_contact
                    WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL
                    ORDER BY is_primary DESC, name ASC
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let contacts = contact_rows
            .into_iter()
            .map(|r| {
                let is_primary_int: i64 = r.get("is_primary");
                CustomerContact {
                    id: r.get("id"),
                    tenant_id: r.get("tenant_id"),
                    customer_id: r.get("customer_id"),
                    name: r.get("name"),
                    email: r.get("email"),
                    phone: r.get("phone"),
                    role: r.get("role"),
                    is_primary: is_primary_int != 0,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                    deleted_at: r.get("deleted_at"),
                    created_by: r.get("created_by"),
                }
            })
            .collect();

        let address_rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, customer_id, label, street, number, complement,
                           neighborhood, city, state, postal_code, country, is_primary,
                           created_at, updated_at, deleted_at, created_by
                    FROM customer_address
                    WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL
                    ORDER BY is_primary DESC, label ASC
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let addresses = address_rows
            .into_iter()
            .map(|r| {
                let is_primary_int: i64 = r.get("is_primary");
                CustomerAddress {
                    id: r.get("id"),
                    tenant_id: r.get("tenant_id"),
                    customer_id: r.get("customer_id"),
                    label: r.get("label"),
                    street: r.get("street"),
                    number: r.get("number"),
                    complement: r.get("complement"),
                    neighborhood: r.get("neighborhood"),
                    city: r.get("city"),
                    state: r.get("state"),
                    postal_code: r.get("postal_code"),
                    country: r.get("country"),
                    is_primary: is_primary_int != 0,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                    deleted_at: r.get("deleted_at"),
                    created_by: r.get("created_by"),
                }
            })
            .collect();

        let cat_rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT category
                    FROM customer_supplier_category
                    WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL
                    ORDER BY category ASC
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let mut supplier_categories = Vec::with_capacity(cat_rows.len());
        for r in cat_rows {
            let cat_str: String = r.get("category");
            let cat: eventrent_core::domain::SupplierCategory =
                cat_str.parse().map_err(CoreError::Database)?;
            supplier_categories.push(cat);
        }

        Ok(Some(CustomerWithDetails {
            customer,
            contacts,
            addresses,
            supplier_categories,
        }))
    }

    async fn find_by_document(
        &self,
        tenant_id: &str,
        document_type: DocumentType,
        document_number: &str,
    ) -> Result<Option<Customer>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, person_type, name, trade_name, document_type,
                           document_number, state_registration, municipal_registration, notes,
                           active, is_customer, is_supplier, search_text, name_sort,
                           created_at, updated_at, deleted_at, created_by
                    FROM customer
                    WHERE tenant_id = ? AND document_type = ? AND document_number = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(document_type.as_str())
                .bind(document_number),
            )
            .await?;

        row.as_ref().map(row_to_customer).transpose()
    }

    async fn list(&self, tenant_id: &str, filter: &CustomerFilter) -> Result<CustomerPage> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 100);
        let fetch_limit = (limit + 1) as i64;

        let cursor_data = filter
            .cursor
            .as_deref()
            .map(CustomerCursor::decode)
            .transpose()?;

        let mut sql = String::from(
            r#"
            SELECT id, tenant_id, person_type, name, trade_name, document_type,
                   document_number, state_registration, municipal_registration, notes,
                   active, is_customer, is_supplier, search_text, name_sort,
                   created_at, updated_at, deleted_at, created_by
            FROM customer
            WHERE tenant_id = ? AND deleted_at IS NULL
            "#,
        );

        match filter.role_filter {
            eventrent_core::domain::CustomerRoleFilter::Customer => {
                sql.push_str(" AND is_customer = 1");
            }
            eventrent_core::domain::CustomerRoleFilter::Supplier => {
                sql.push_str(" AND is_supplier = 1");
            }
            eventrent_core::domain::CustomerRoleFilter::Both => {
                sql.push_str(" AND is_customer = 1 AND is_supplier = 1");
            }
            eventrent_core::domain::CustomerRoleFilter::All => {}
        }

        if let Some(pt) = filter.person_type {
            sql.push_str(" AND person_type = '");
            sql.push_str(&pt.to_string());
            sql.push('\'');
        }

        let query_words: Vec<String> = filter
            .query
            .as_deref()
            .map(|q| {
                normalize_for_search(q)
                    .split_whitespace()
                    .map(|w| format!("%{w}%"))
                    .collect()
            })
            .unwrap_or_default();

        for _ in &query_words {
            sql.push_str(" AND search_text LIKE ?");
        }

        if cursor_data.is_some() {
            sql.push_str(" AND (name_sort > ? OR (name_sort = ? AND id > ?))");
        }

        sql.push_str(" ORDER BY name_sort ASC, id ASC LIMIT ?");

        let mut query = sqlx::query(&sql).bind(tenant_id);

        for word_pattern in &query_words {
            query = query.bind(word_pattern);
        }

        if let Some(ref c) = cursor_data {
            query = query.bind(&c.name_sort).bind(&c.name_sort).bind(&c.id);
        }

        query = query.bind(fetch_limit);

        let rows = self.fetch_all(query).await?;

        let mut items = Vec::new();
        for r in &rows {
            items.push(row_to_customer(r)?);
        }

        let has_more = items.len() > limit as usize;
        if has_more {
            items.pop();
        }

        let next_cursor = if has_more {
            items
                .last()
                .map(|last| CustomerCursor::new(&last.name_sort, &last.id).encode())
        } else {
            None
        };

        Ok(CustomerPage {
            items,
            next_cursor,
            has_more,
            total_count: None,
        })
    }

    async fn create(
        &self,
        customer: &Customer,
        contacts: &[CustomerContact],
        addresses: &[CustomerAddress],
        supplier_categories: &[eventrent_core::domain::SupplierCategory],
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                INSERT INTO customer (
                    id, tenant_id, person_type, name, trade_name, document_type,
                    document_number, state_registration, municipal_registration, notes,
                    active, is_customer, is_supplier, search_text, name_sort,
                    created_at, updated_at, deleted_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(customer.person_type.to_string())
            .bind(&customer.name)
            .bind(&customer.trade_name)
            .bind(customer.document_type.as_str())
            .bind(&customer.document_number)
            .bind(&customer.state_registration)
            .bind(&customer.municipal_registration)
            .bind(&customer.notes)
            .bind(if customer.active { 1i64 } else { 0i64 })
            .bind(if customer.is_customer { 1i64 } else { 0i64 })
            .bind(if customer.is_supplier { 1i64 } else { 0i64 })
            .bind(&customer.search_text)
            .bind(&customer.name_sort)
            .bind(&customer.created_at)
            .bind(&customer.updated_at)
            .bind(&customer.deleted_at)
            .bind(&customer.created_by),
        )
        .await?;

        for c in contacts {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO customer_contact (
                        id, tenant_id, customer_id, name, email, phone, role, is_primary,
                        created_at, updated_at, deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&c.id)
                .bind(&c.tenant_id)
                .bind(&c.customer_id)
                .bind(&c.name)
                .bind(&c.email)
                .bind(&c.phone)
                .bind(&c.role)
                .bind(if c.is_primary { 1i64 } else { 0i64 })
                .bind(&c.created_at)
                .bind(&c.updated_at)
                .bind(&c.deleted_at)
                .bind(&c.created_by),
            )
            .await?;
        }

        for a in addresses {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO customer_address (
                        id, tenant_id, customer_id, label, street, number, complement,
                        neighborhood, city, state, postal_code, country, is_primary,
                        created_at, updated_at, deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&a.id)
                .bind(&a.tenant_id)
                .bind(&a.customer_id)
                .bind(&a.label)
                .bind(&a.street)
                .bind(&a.number)
                .bind(&a.complement)
                .bind(&a.neighborhood)
                .bind(&a.city)
                .bind(&a.state)
                .bind(&a.postal_code)
                .bind(&a.country)
                .bind(if a.is_primary { 1i64 } else { 0i64 })
                .bind(&a.created_at)
                .bind(&a.updated_at)
                .bind(&a.deleted_at)
                .bind(&a.created_by),
            )
            .await?;
        }

        for cat in supplier_categories {
            let cat_id = Uuid::now_v7().to_string();
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO customer_supplier_category (
                        id, tenant_id, customer_id, category, created_at, updated_at, deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, NULL, ?)
                    "#,
                )
                .bind(&cat_id)
                .bind(&customer.tenant_id)
                .bind(&customer.id)
                .bind(cat.as_str())
                .bind(&customer.created_at)
                .bind(&customer.updated_at)
                .bind(&customer.created_by),
            )
            .await?;
        }

        Ok(())
    }

    async fn update(
        &self,
        customer: &Customer,
        contacts: &[CustomerContact],
        addresses: &[CustomerAddress],
        supplier_categories: &[eventrent_core::domain::SupplierCategory],
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE customer
                SET person_type = ?, name = ?, trade_name = ?, document_type = ?, document_number = ?,
                    state_registration = ?, municipal_registration = ?, notes = ?,
                    active = ?, is_customer = ?, is_supplier = ?, search_text = ?, name_sort = ?, updated_at = ?
                WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(customer.person_type.to_string())
            .bind(&customer.name)
            .bind(&customer.trade_name)
            .bind(customer.document_type.as_str())
            .bind(&customer.document_number)
            .bind(&customer.state_registration)
            .bind(&customer.municipal_registration)
            .bind(&customer.notes)
            .bind(if customer.active { 1i64 } else { 0i64 })
            .bind(if customer.is_customer { 1i64 } else { 0i64 })
            .bind(if customer.is_supplier { 1i64 } else { 0i64 })
            .bind(&customer.search_text)
            .bind(&customer.name_sort)
            .bind(&customer.updated_at)
            .bind(&customer.id)
            .bind(&customer.tenant_id),
        )
        .await?;

        // Limpa flag primária temporariamente para permitir troca de principal sem violar índice único parcial
        self.execute(
            sqlx::query(
                "UPDATE customer_contact SET is_primary = 0 WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
            )
            .bind(&customer.tenant_id)
            .bind(&customer.id),
        )
        .await?;

        // Exclusão lógica (soft delete) dos contatos removidos
        if !contacts.is_empty() {
            let placeholders = vec!["?"; contacts.len()].join(", ");
            let sql_soft_del = format!(
                "UPDATE customer_contact SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL AND id NOT IN ({placeholders})"
            );
            let mut q = sqlx::query(&sql_soft_del)
                .bind(&customer.updated_at)
                .bind(&customer.updated_at)
                .bind(&customer.tenant_id)
                .bind(&customer.id);
            for c in contacts {
                q = q.bind(&c.id);
            }
            self.execute(q).await?;
        }

        // Upsert preservando IDs existentes
        for c in contacts {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO customer_contact (
                        id, tenant_id, customer_id, name, email, phone, role, is_primary,
                        created_at, updated_at, deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        email = excluded.email,
                        phone = excluded.phone,
                        role = excluded.role,
                        is_primary = excluded.is_primary,
                        updated_at = excluded.updated_at,
                        deleted_at = NULL
                    WHERE customer_contact.tenant_id = excluded.tenant_id
                      AND customer_contact.customer_id = excluded.customer_id
                    "#,
                )
                .bind(&c.id)
                .bind(&c.tenant_id)
                .bind(&c.customer_id)
                .bind(&c.name)
                .bind(&c.email)
                .bind(&c.phone)
                .bind(&c.role)
                .bind(if c.is_primary { 1i64 } else { 0i64 })
                .bind(&c.created_at)
                .bind(&c.updated_at)
                .bind(&c.deleted_at)
                .bind(&c.created_by),
            )
            .await?;
        }

        self.execute(
            sqlx::query(
                "UPDATE customer_address SET is_primary = 0 WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
            )
            .bind(&customer.tenant_id)
            .bind(&customer.id),
        )
        .await?;

        if !addresses.is_empty() {
            let placeholders = vec!["?"; addresses.len()].join(", ");
            let sql_soft_del = format!(
                "UPDATE customer_address SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL AND id NOT IN ({placeholders})"
            );
            let mut q = sqlx::query(&sql_soft_del)
                .bind(&customer.updated_at)
                .bind(&customer.updated_at)
                .bind(&customer.tenant_id)
                .bind(&customer.id);
            for a in addresses {
                q = q.bind(&a.id);
            }
            self.execute(q).await?;
        } else {
            self.execute(
                sqlx::query(
                    "UPDATE customer_address SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
                )
                .bind(&customer.updated_at)
                .bind(&customer.updated_at)
                .bind(&customer.tenant_id)
                .bind(&customer.id),
            )
            .await?;
        }

        for a in addresses {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO customer_address (
                        id, tenant_id, customer_id, label, street, number, complement,
                        neighborhood, city, state, postal_code, country, is_primary,
                        created_at, updated_at, deleted_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        label = excluded.label,
                        street = excluded.street,
                        number = excluded.number,
                        complement = excluded.complement,
                        neighborhood = excluded.neighborhood,
                        city = excluded.city,
                        state = excluded.state,
                        postal_code = excluded.postal_code,
                        country = excluded.country,
                        is_primary = excluded.is_primary,
                        updated_at = excluded.updated_at,
                        deleted_at = NULL
                    WHERE customer_address.tenant_id = excluded.tenant_id
                      AND customer_address.customer_id = excluded.customer_id
                    "#,
                )
                .bind(&a.id)
                .bind(&a.tenant_id)
                .bind(&a.customer_id)
                .bind(&a.label)
                .bind(&a.street)
                .bind(&a.number)
                .bind(&a.complement)
                .bind(&a.neighborhood)
                .bind(&a.city)
                .bind(&a.state)
                .bind(&a.postal_code)
                .bind(&a.country)
                .bind(if a.is_primary { 1i64 } else { 0i64 })
                .bind(&a.created_at)
                .bind(&a.updated_at)
                .bind(&a.deleted_at)
                .bind(&a.created_by),
            )
            .await?;
        }

        // Sincroniza categorias de fornecedor sem colidir com idx_customer_supplier_category_active
        if !supplier_categories.is_empty() {
            let placeholders = vec!["?"; supplier_categories.len()].join(", ");
            let sql_soft_del = format!(
                "UPDATE customer_supplier_category SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL AND category NOT IN ({placeholders})"
            );
            let mut q = sqlx::query(&sql_soft_del)
                .bind(&customer.updated_at)
                .bind(&customer.updated_at)
                .bind(&customer.tenant_id)
                .bind(&customer.id);
            for cat in supplier_categories {
                q = q.bind(cat.as_str());
            }
            self.execute(q).await?;
        } else {
            self.execute(
                sqlx::query(
                    "UPDATE customer_supplier_category SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
                )
                .bind(&customer.updated_at)
                .bind(&customer.updated_at)
                .bind(&customer.tenant_id)
                .bind(&customer.id),
            )
            .await?;
        }

        let active_cat_rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT category FROM customer_supplier_category WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
                )
                .bind(&customer.tenant_id)
                .bind(&customer.id),
            )
            .await?;
        let active_cats: std::collections::HashSet<String> = active_cat_rows
            .into_iter()
            .map(|r| r.get::<String, _>("category"))
            .collect();

        for cat in supplier_categories {
            if !active_cats.contains(cat.as_str()) {
                let cat_id = Uuid::now_v7().to_string();
                self.execute(
                    sqlx::query(
                        r#"
                        INSERT INTO customer_supplier_category (
                            id, tenant_id, customer_id, category, created_at, updated_at, deleted_at, created_by
                        ) VALUES (?, ?, ?, ?, ?, ?, NULL, ?)
                        "#,
                    )
                    .bind(&cat_id)
                    .bind(&customer.tenant_id)
                    .bind(&customer.id)
                    .bind(cat.as_str())
                    .bind(&customer.updated_at)
                    .bind(&customer.updated_at)
                    .bind(&customer.created_by),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn set_active(
        &self,
        tenant_id: &str,
        id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE customer SET active = ?, updated_at = ? WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(if active { 1i64 } else { 0i64 })
            .bind(updated_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn soft_delete(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE customer SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;

        self.execute(
            sqlx::query(
                "UPDATE customer_contact SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;

        self.execute(
            sqlx::query(
                "UPDATE customer_address SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;

        self.execute(
            sqlx::query(
                "UPDATE customer_supplier_category SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND customer_id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;

        Ok(())
    }

    async fn count_active(&self, tenant_id: &str) -> Result<u64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) as count FROM customer WHERE tenant_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id),
            )
            .await?;
        let count: i64 = row.get("count");
        Ok(count as u64)
    }
}

fn row_to_venue(r: &sqlx::sqlite::SqliteRow) -> Venue {
    let active_int: i64 = r.get("active");
    Venue {
        id: r.get("id"),
        tenant_id: r.get("tenant_id"),
        name: r.get("name"),
        street: r.get("street"),
        number: r.get("number"),
        complement: r.get("complement"),
        neighborhood: r.get("neighborhood"),
        city: r.get("city"),
        state: r.get("state"),
        postal_code: r.get("postal_code"),
        country: r.get("country"),
        parking_notes: r.get("parking_notes"),
        loading_dock_notes: r.get("loading_dock_notes"),
        access_hours: r.get("access_hours"),
        internal_transfer_minutes: r.get("internal_transfer_minutes"),
        notes: r.get("notes"),
        active: active_int != 0,
        search_text: r.get("search_text"),
        name_sort: r.get("name_sort"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        deleted_at: r.get("deleted_at"),
        created_by: r.get("created_by"),
    }
}

fn row_to_venue_space(r: &sqlx::sqlite::SqliteRow) -> VenueSpace {
    let active_int: i64 = r.get("active");
    VenueSpace {
        id: r.get("id"),
        tenant_id: r.get("tenant_id"),
        venue_id: r.get("venue_id"),
        name: r.get("name"),
        name_normalized: r.get("name_normalized"),
        capacity_people: r.get("capacity_people"),
        area_sqm: r.get("area_sqm"),
        ceiling_height_m: r.get("ceiling_height_m"),
        power_available: r.get("power_available"),
        loading_access: r.get("loading_access"),
        rigging_points: r.get("rigging_points"),
        acoustics_notes: r.get("acoustics_notes"),
        setup_hours_notes: r.get("setup_hours_notes"),
        noise_restrictions_notes: r.get("noise_restrictions_notes"),
        load_dimensions_notes: r.get("load_dimensions_notes"),
        notes: r.get("notes"),
        active: active_int != 0,
        sort_order: r.get("sort_order"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        deleted_at: r.get("deleted_at"),
        created_by: r.get("created_by"),
    }
}

fn row_to_venue_contact(r: &sqlx::sqlite::SqliteRow) -> VenueContact {
    let is_primary_int: i64 = r.get("is_primary");
    VenueContact {
        id: r.get("id"),
        tenant_id: r.get("tenant_id"),
        venue_id: r.get("venue_id"),
        name: r.get("name"),
        email: r.get("email"),
        phone: r.get("phone"),
        role: r.get("role"),
        is_primary: is_primary_int != 0,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        deleted_at: r.get("deleted_at"),
        created_by: r.get("created_by"),
    }
}

#[async_trait]
impl VenueStore for SqliteStore {
    async fn find_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<VenueWithDetails>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, name, street, number, complement, neighborhood,
                           city, state, postal_code, country, parking_notes, loading_dock_notes,
                           access_hours, internal_transfer_minutes, notes, active, search_text, name_sort,
                           created_at, updated_at, deleted_at, created_by
                    FROM venue
                    WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let venue = match row {
            Some(ref r) => row_to_venue(r),
            None => return Ok(None),
        };

        let space_rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, venue_id, name, name_normalized, capacity_people,
                           area_sqm, ceiling_height_m, power_available, loading_access,
                           rigging_points, acoustics_notes, setup_hours_notes,
                           noise_restrictions_notes, load_dimensions_notes, notes,
                           active, sort_order, created_at, updated_at, deleted_at, created_by
                    FROM venue_space
                    WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL
                    ORDER BY sort_order ASC, name ASC
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let spaces = space_rows.iter().map(row_to_venue_space).collect();

        let contact_rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, tenant_id, venue_id, name, email, phone, role, is_primary,
                           created_at, updated_at, deleted_at, created_by
                    FROM venue_contact
                    WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL
                    ORDER BY is_primary DESC, name ASC
                    "#,
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;

        let contacts = contact_rows.iter().map(row_to_venue_contact).collect();

        Ok(Some(VenueWithDetails {
            venue,
            spaces,
            contacts,
        }))
    }

    async fn list(&self, tenant_id: &str, filter: &VenueFilter) -> Result<VenuePage> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 100) as i64;
        let fetch_limit = limit + 1;

        let cursor = match filter.cursor {
            Some(ref c) => Some(VenueCursor::decode(c)?),
            None => None,
        };

        let search_pattern = filter.query.as_ref().map(|q| {
            let escaped = q
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            format!("%{escaped}%")
        });

        let mut sql = String::from(
            r#"
            SELECT v.id, v.tenant_id, v.name, v.street, v.number, v.complement, v.neighborhood,
                   v.city, v.state, v.postal_code, v.country, v.parking_notes, v.loading_dock_notes,
                   v.access_hours, v.internal_transfer_minutes, v.notes, v.active, v.search_text, v.name_sort,
                   v.created_at, v.updated_at, v.deleted_at, v.created_by,
                   (SELECT COUNT(*) FROM venue_space vs
                    WHERE vs.tenant_id = v.tenant_id AND vs.venue_id = v.id AND vs.deleted_at IS NULL) AS spaces_count
            FROM venue v
            WHERE v.tenant_id = ? AND v.deleted_at IS NULL
            "#,
        );

        if filter.active_only == Some(true) {
            sql.push_str(" AND v.active = 1 ");
        } else if filter.active_only == Some(false) {
            sql.push_str(" AND v.active = 0 ");
        }
        if search_pattern.is_some() {
            sql.push_str(" AND v.search_text LIKE ? ESCAPE '\\' ");
        }
        if cursor.is_some() {
            sql.push_str(" AND (v.name_sort > ? OR (v.name_sort = ? AND v.id > ?)) ");
        }
        sql.push_str(" ORDER BY v.name_sort ASC, v.id ASC LIMIT ? ");

        let mut q = sqlx::query(&sql).bind(tenant_id);
        if let Some(ref pat) = search_pattern {
            q = q.bind(pat);
        }
        if let Some(ref cur) = cursor {
            q = q.bind(&cur.name_sort).bind(&cur.name_sort).bind(&cur.id);
        }
        q = q.bind(fetch_limit);

        let rows = self.fetch_all(q).await?;
        let has_more = rows.len() as i64 > limit;
        let items: Vec<VenueListItem> = rows
            .into_iter()
            .take(limit as usize)
            .map(|r| {
                let spaces_count: i64 = r.get("spaces_count");
                VenueListItem {
                    venue: row_to_venue(&r),
                    spaces_count: spaces_count as u64,
                }
            })
            .collect();

        let next_cursor = if has_more {
            items
                .last()
                .map(|last| VenueCursor::new(&last.venue.name_sort, &last.venue.id).encode())
        } else {
            None
        };

        let mut count_sql = String::from(
            "SELECT COUNT(*) as total FROM venue WHERE tenant_id = ? AND deleted_at IS NULL",
        );
        if filter.active_only == Some(true) {
            count_sql.push_str(" AND active = 1 ");
        } else if filter.active_only == Some(false) {
            count_sql.push_str(" AND active = 0 ");
        }
        if search_pattern.is_some() {
            count_sql.push_str(" AND search_text LIKE ? ESCAPE '\\' ");
        }
        let mut cq = sqlx::query(&count_sql).bind(tenant_id);
        if let Some(ref pat) = search_pattern {
            cq = cq.bind(pat);
        }
        let count_row = self.fetch_one(cq).await?;
        let total_count: i64 = count_row.get("total");

        Ok(VenuePage {
            items,
            next_cursor,
            has_more,
            total_count: Some(total_count as u64),
        })
    }

    async fn exists_by_name_and_city(
        &self,
        tenant_id: &str,
        name_normalized: &str,
        city_normalized: Option<&str>,
        exclude_id: Option<&str>,
    ) -> Result<bool> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    r#"
                    SELECT id, name, city
                    FROM venue
                    WHERE tenant_id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(tenant_id),
            )
            .await?;

        for row in rows {
            let id: String = row.get("id");
            if exclude_id == Some(id.as_str()) {
                continue;
            }
            let row_name: String = row.get("name");
            if normalize_for_search(&row_name) != name_normalized {
                continue;
            }
            let row_city: Option<String> = row.get("city");
            let row_city_norm = row_city
                .as_deref()
                .map(normalize_for_search)
                .filter(|s| !s.is_empty());
            if row_city_norm.as_deref() == city_normalized {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn create(
        &self,
        venue: &Venue,
        spaces: &[VenueSpace],
        contacts: &[VenueContact],
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                INSERT INTO venue (
                    id, tenant_id, name, street, number, complement, neighborhood, city, state,
                    postal_code, country, parking_notes, loading_dock_notes, access_hours,
                    internal_transfer_minutes, notes, active, search_text, name_sort,
                    created_at, updated_at, deleted_at, created_by
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&venue.id)
            .bind(&venue.tenant_id)
            .bind(&venue.name)
            .bind(&venue.street)
            .bind(&venue.number)
            .bind(&venue.complement)
            .bind(&venue.neighborhood)
            .bind(&venue.city)
            .bind(&venue.state)
            .bind(&venue.postal_code)
            .bind(&venue.country)
            .bind(&venue.parking_notes)
            .bind(&venue.loading_dock_notes)
            .bind(&venue.access_hours)
            .bind(venue.internal_transfer_minutes)
            .bind(&venue.notes)
            .bind(if venue.active { 1 } else { 0 })
            .bind(&venue.search_text)
            .bind(&venue.name_sort)
            .bind(&venue.created_at)
            .bind(&venue.updated_at)
            .bind(&venue.deleted_at)
            .bind(&venue.created_by),
        )
        .await?;

        for sp in spaces {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO venue_space (
                        id, tenant_id, venue_id, name, name_normalized, capacity_people, area_sqm,
                        ceiling_height_m, power_available, loading_access, rigging_points,
                        acoustics_notes, setup_hours_notes, noise_restrictions_notes,
                        load_dimensions_notes, notes, active, sort_order,
                        created_at, updated_at, deleted_at, created_by
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&sp.id)
                .bind(&sp.tenant_id)
                .bind(&sp.venue_id)
                .bind(&sp.name)
                .bind(&sp.name_normalized)
                .bind(sp.capacity_people)
                .bind(sp.area_sqm)
                .bind(sp.ceiling_height_m)
                .bind(&sp.power_available)
                .bind(&sp.loading_access)
                .bind(&sp.rigging_points)
                .bind(&sp.acoustics_notes)
                .bind(&sp.setup_hours_notes)
                .bind(&sp.noise_restrictions_notes)
                .bind(&sp.load_dimensions_notes)
                .bind(&sp.notes)
                .bind(if sp.active { 1 } else { 0 })
                .bind(sp.sort_order)
                .bind(&sp.created_at)
                .bind(&sp.updated_at)
                .bind(&sp.deleted_at)
                .bind(&sp.created_by),
            )
            .await?;
        }

        for ct in contacts {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO venue_contact (
                        id, tenant_id, venue_id, name, email, phone, role, is_primary,
                        created_at, updated_at, deleted_at, created_by
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&ct.id)
                .bind(&ct.tenant_id)
                .bind(&ct.venue_id)
                .bind(&ct.name)
                .bind(&ct.email)
                .bind(&ct.phone)
                .bind(&ct.role)
                .bind(if ct.is_primary { 1 } else { 0 })
                .bind(&ct.created_at)
                .bind(&ct.updated_at)
                .bind(&ct.deleted_at)
                .bind(&ct.created_by),
            )
            .await?;
        }

        Ok(())
    }

    async fn update(
        &self,
        venue: &Venue,
        spaces: &[VenueSpace],
        contacts: &[VenueContact],
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE venue
                SET name = ?, street = ?, number = ?, complement = ?, neighborhood = ?,
                    city = ?, state = ?, postal_code = ?, country = ?, parking_notes = ?,
                    loading_dock_notes = ?, access_hours = ?, internal_transfer_minutes = ?,
                    notes = ?, active = ?, search_text = ?, name_sort = ?, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(&venue.name)
            .bind(&venue.street)
            .bind(&venue.number)
            .bind(&venue.complement)
            .bind(&venue.neighborhood)
            .bind(&venue.city)
            .bind(&venue.state)
            .bind(&venue.postal_code)
            .bind(&venue.country)
            .bind(&venue.parking_notes)
            .bind(&venue.loading_dock_notes)
            .bind(&venue.access_hours)
            .bind(venue.internal_transfer_minutes)
            .bind(&venue.notes)
            .bind(if venue.active { 1 } else { 0 })
            .bind(&venue.search_text)
            .bind(&venue.name_sort)
            .bind(&venue.updated_at)
            .bind(&venue.tenant_id)
            .bind(&venue.id),
        )
        .await?;

        // Soft-delete espaços removidos antes do upsert para liberar idx_venue_space_name_active
        if !spaces.is_empty() {
            let placeholders = vec!["?"; spaces.len()].join(", ");
            let sql_soft_del = format!(
                "UPDATE venue_space SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL AND id NOT IN ({placeholders})"
            );
            let mut q = sqlx::query(&sql_soft_del)
                .bind(&venue.updated_at)
                .bind(&venue.updated_at)
                .bind(&venue.tenant_id)
                .bind(&venue.id);
            for sp in spaces {
                q = q.bind(&sp.id);
            }
            self.execute(q).await?;
        } else {
            self.execute(
                sqlx::query(
                    "UPDATE venue_space SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL",
                )
                .bind(&venue.updated_at)
                .bind(&venue.updated_at)
                .bind(&venue.tenant_id)
                .bind(&venue.id),
            )
            .await?;
        }

        for sp in spaces {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO venue_space (
                        id, tenant_id, venue_id, name, name_normalized, capacity_people, area_sqm,
                        ceiling_height_m, power_available, loading_access, rigging_points,
                        acoustics_notes, setup_hours_notes, noise_restrictions_notes,
                        load_dimensions_notes, notes, active, sort_order,
                        created_at, updated_at, deleted_at, created_by
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        name_normalized = excluded.name_normalized,
                        capacity_people = excluded.capacity_people,
                        area_sqm = excluded.area_sqm,
                        ceiling_height_m = excluded.ceiling_height_m,
                        power_available = excluded.power_available,
                        loading_access = excluded.loading_access,
                        rigging_points = excluded.rigging_points,
                        acoustics_notes = excluded.acoustics_notes,
                        setup_hours_notes = excluded.setup_hours_notes,
                        noise_restrictions_notes = excluded.noise_restrictions_notes,
                        load_dimensions_notes = excluded.load_dimensions_notes,
                        notes = excluded.notes,
                        active = excluded.active,
                        sort_order = excluded.sort_order,
                        updated_at = excluded.updated_at,
                        deleted_at = NULL
                    "#,
                )
                .bind(&sp.id)
                .bind(&sp.tenant_id)
                .bind(&sp.venue_id)
                .bind(&sp.name)
                .bind(&sp.name_normalized)
                .bind(sp.capacity_people)
                .bind(sp.area_sqm)
                .bind(sp.ceiling_height_m)
                .bind(&sp.power_available)
                .bind(&sp.loading_access)
                .bind(&sp.rigging_points)
                .bind(&sp.acoustics_notes)
                .bind(&sp.setup_hours_notes)
                .bind(&sp.noise_restrictions_notes)
                .bind(&sp.load_dimensions_notes)
                .bind(&sp.notes)
                .bind(if sp.active { 1 } else { 0 })
                .bind(sp.sort_order)
                .bind(&sp.created_at)
                .bind(&sp.updated_at)
                .bind(&sp.deleted_at)
                .bind(&sp.created_by),
            )
            .await?;
        }

        // Reseta is_primary antes de sincronizar contatos para evitar colisão em idx_venue_contact_primary_active
        self.execute(
            sqlx::query(
                "UPDATE venue_contact SET is_primary = 0 WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL",
            )
            .bind(&venue.tenant_id)
            .bind(&venue.id),
        )
        .await?;

        if !contacts.is_empty() {
            let placeholders = vec!["?"; contacts.len()].join(", ");
            let sql_soft_del = format!(
                "UPDATE venue_contact SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL AND id NOT IN ({placeholders})"
            );
            let mut q = sqlx::query(&sql_soft_del)
                .bind(&venue.updated_at)
                .bind(&venue.updated_at)
                .bind(&venue.tenant_id)
                .bind(&venue.id);
            for ct in contacts {
                q = q.bind(&ct.id);
            }
            self.execute(q).await?;
        } else {
            self.execute(
                sqlx::query(
                    "UPDATE venue_contact SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL",
                )
                .bind(&venue.updated_at)
                .bind(&venue.updated_at)
                .bind(&venue.tenant_id)
                .bind(&venue.id),
            )
            .await?;
        }

        for ct in contacts {
            self.execute(
                sqlx::query(
                    r#"
                    INSERT INTO venue_contact (
                        id, tenant_id, venue_id, name, email, phone, role, is_primary,
                        created_at, updated_at, deleted_at, created_by
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        email = excluded.email,
                        phone = excluded.phone,
                        role = excluded.role,
                        is_primary = excluded.is_primary,
                        updated_at = excluded.updated_at,
                        deleted_at = NULL
                    "#,
                )
                .bind(&ct.id)
                .bind(&ct.tenant_id)
                .bind(&ct.venue_id)
                .bind(&ct.name)
                .bind(&ct.email)
                .bind(&ct.phone)
                .bind(&ct.role)
                .bind(if ct.is_primary { 1 } else { 0 })
                .bind(&ct.created_at)
                .bind(&ct.updated_at)
                .bind(&ct.deleted_at)
                .bind(&ct.created_by),
            )
            .await?;
        }

        Ok(())
    }

    async fn set_active(
        &self,
        tenant_id: &str,
        id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE venue SET active = ?, updated_at = ? WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(if active { 1 } else { 0 })
            .bind(updated_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn soft_delete(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE venue SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        self.execute(
            sqlx::query(
                "UPDATE venue_space SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        self.execute(
            sqlx::query(
                "UPDATE venue_contact SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND venue_id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn count_active(&self, tenant_id: &str) -> Result<u64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) as count FROM venue WHERE tenant_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id),
            )
            .await?;
        let count: i64 = row.get("count");
        Ok(count as u64)
    }
}

#[async_trait]
impl TenantSettingStore for SqliteStore {
    async fn get_many(&self, tenant_id: &str, keys: &[&str]) -> Result<Vec<TenantSetting>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; keys.len()].join(", ");
        let sql = format!(
            "SELECT id, tenant_id, setting_key, setting_value, updated_at, updated_by \
             FROM tenant_setting WHERE tenant_id = ? AND setting_key IN ({placeholders})"
        );
        let mut q = sqlx::query(&sql).bind(tenant_id);
        for key in keys {
            q = q.bind(*key);
        }
        let rows = self.fetch_all(q).await?;
        Ok(rows
            .into_iter()
            .map(|r| TenantSetting {
                id: r.get("id"),
                tenant_id: r.get("tenant_id"),
                setting_key: r.get("setting_key"),
                setting_value: r.get("setting_value"),
                updated_at: r.get("updated_at"),
                updated_by: r.get("updated_by"),
            })
            .collect())
    }

    async fn upsert(&self, setting: &TenantSetting) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                INSERT INTO tenant_setting (id, tenant_id, setting_key, setting_value, updated_at, updated_by)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(tenant_id, setting_key) DO UPDATE SET
                    setting_value = excluded.setting_value,
                    updated_at = excluded.updated_at,
                    updated_by = excluded.updated_by
                "#,
            )
            .bind(&setting.id)
            .bind(&setting.tenant_id)
            .bind(&setting.setting_key)
            .bind(&setting.setting_value)
            .bind(&setting.updated_at)
            .bind(&setting.updated_by),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eventrent_core::domain::{
        CreateUserInput, InitialSetupInput, LoginInput, LoginPolicy, OperationMode, PersonType,
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
                customers: store.clone(),
                venues: store.clone(),
                settings: store.clone(),
                catalog: store,
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
            customers: Arc::new(normal_store.clone()),
            venues: Arc::new(normal_store.clone()),
            settings: Arc::new(normal_store.clone()),
            catalog: Arc::new(normal_store),
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
                &LoginPolicy::default(),
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
    async fn test_recovery_failure_persists_counter_and_lockout_despite_returning_err() {
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
                    company_name: "Empresa Recovery Test".to_string(),
                    owner_name: "Dono".to_string(),
                    owner_email: "dono.recovery@empresa.com".to_string(),
                    owner_password: "SenhaCerta#2026".to_string(),
                },
                30,
            )
            .await
            .unwrap();

        let policy = LoginPolicy {
            reveal_account_locked: true,
        };

        // Tentativa 1 com chave de recuperação inválida
        let err1 = services
            .recover_password(
                &tenant_id,
                &RecoverPasswordInput {
                    email: setup.session.email.clone(),
                    recovery_key: "ER-CHAVE-INVALIDA-0001".to_string(),
                    new_password: "NovaSenha#2026".to_string(),
                },
                &policy,
            )
            .await;
        assert!(matches!(err1, Err(CoreError::InvalidCredentials)));

        // Prova que recovery_failed_attempts foi gravado (1) e auditoria de falha gerada
        let user = store
            .users
            .find_by_email(&tenant_id, &setup.session.email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.recovery_failed_attempts, 1);
        assert!(user.recovery_locked_until.is_none());
        let audits = store.audit.list_by_tenant(&tenant_id, 10).await.unwrap();
        assert!(audits.iter().any(|a| a.action == "auth.recovery_failed"));

        // Tentativas 2, 3 e 4
        for i in 2..=4 {
            let res = services
                .recover_password(
                    &tenant_id,
                    &RecoverPasswordInput {
                        email: setup.session.email.clone(),
                        recovery_key: format!("ER-CHAVE-INVALIDA-000{i}"),
                        new_password: "NovaSenha#2026".to_string(),
                    },
                    &policy,
                )
                .await;
            assert!(matches!(res, Err(CoreError::InvalidCredentials)));
            let user = store
                .users
                .find_by_email(&tenant_id, &setup.session.email)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(user.recovery_failed_attempts, i);
            assert!(user.recovery_locked_until.is_none());
        }

        // Tentativa 5: atinge o limite e bloqueia a recuperação
        let err5 = services
            .recover_password(
                &tenant_id,
                &RecoverPasswordInput {
                    email: setup.session.email.clone(),
                    recovery_key: "ER-CHAVE-INVALIDA-0005".to_string(),
                    new_password: "NovaSenha#2026".to_string(),
                },
                &policy,
            )
            .await;
        assert!(matches!(err5, Err(CoreError::AccountLocked { .. })));

        let user_locked = store
            .users
            .find_by_email(&tenant_id, &setup.session.email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user_locked.recovery_failed_attempts, 5);
        assert!(
            user_locked.recovery_locked_until.is_some(),
            "recovery_locked_until deve estar preenchido no banco"
        );

        let audits_locked = store.audit.list_by_tenant(&tenant_id, 20).await.unwrap();
        assert!(audits_locked
            .iter()
            .any(|a| a.action == "auth.recovery_locked"));

        // Prova proteção anti-enumeração: com reveal_account_locked = false, retorna InvalidCredentials
        let policy_masked = LoginPolicy {
            reveal_account_locked: false,
        };
        let err_masked = services
            .recover_password(
                &tenant_id,
                &RecoverPasswordInput {
                    email: setup.session.email.clone(),
                    recovery_key: "ER-CHAVE-QUALQUER-0000".to_string(),
                    new_password: "NovaSenha#2026".to_string(),
                },
                &policy_masked,
            )
            .await;
        assert!(matches!(err_masked, Err(CoreError::InvalidCredentials)));

        // Prova proteção anti-enumeração com e-mail inexistente: retorna InvalidCredentials
        let err_non_existent = services
            .recover_password(
                &tenant_id,
                &RecoverPasswordInput {
                    email: "naoexiste@luzesom.com.br".to_string(),
                    recovery_key: "ER-CHAVE-QUALQUER-0000".to_string(),
                    new_password: "NovaSenha#2026".to_string(),
                },
                &LoginPolicy::default(),
            )
            .await;
        assert!(matches!(
            err_non_existent,
            Err(CoreError::InvalidCredentials)
        ));
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
            customers: store.clone(),
            venues: store.clone(),
            settings: store.clone(),
            catalog: store.clone(),
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

    #[test]
    fn test_map_sqlx_err_transient_and_permanent_classification() {
        use std::borrow::Cow;

        #[derive(Debug)]
        struct MockDbError {
            code: Option<String>,
            message: String,
        }

        impl std::fmt::Display for MockDbError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message)
            }
        }

        impl std::error::Error for MockDbError {}

        impl sqlx::error::DatabaseError for MockDbError {
            fn message(&self) -> &str {
                &self.message
            }
            fn code(&self) -> Option<Cow<'_, str>> {
                self.code.as_deref().map(Cow::Borrowed)
            }
            fn kind(&self) -> sqlx::error::ErrorKind {
                sqlx::error::ErrorKind::Other
            }
            fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
                self
            }
            fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
                self
            }
            fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
                self
            }
        }

        fn make_db_err(code: Option<&str>, msg: &str) -> sqlx::Error {
            sqlx::Error::Database(Box::new(MockDbError {
                code: code.map(String::from),
                message: msg.to_string(),
            }))
        }

        // 1. Códigos SQLite de conflito transitório
        let transient_codes = [
            ("5", "database is locked / busy"),
            ("6", "database table is locked"),
            ("261", "database busy: recovery in progress"),
            ("517", "database busy: snapshot conflict"),
            ("773", "database busy: timeout expired"),
            ("262", "table locked: shared cache"),
            ("518", "virtual table locked"),
        ];

        for (code, msg) in transient_codes {
            let err = make_db_err(Some(code), msg);
            let mapped = map_sqlx_err(err);
            assert!(
                matches!(mapped, CoreError::TransientConflict(_)),
                "Código {code} ({msg}) deveria virar CoreError::TransientConflict, mas virou: {mapped:?}"
            );
        }

        // 2. Erros de unicidade e integridade referencial com códigos mapeados
        let unique_err = make_db_err(
            Some("2067"),
            "UNIQUE constraint failed: customer.document_number",
        );
        assert_eq!(
            map_sqlx_err(unique_err),
            CoreError::Conflict("Registro conflitante já existe.".to_string())
        );

        let pk_err = make_db_err(Some("1555"), "PRIMARY KEY constraint failed: customer.id");
        assert_eq!(
            map_sqlx_err(pk_err),
            CoreError::Conflict("Registro conflitante já existe.".to_string())
        );

        let fk_err = make_db_err(Some("787"), "FOREIGN KEY constraint failed");
        assert_eq!(
            map_sqlx_err(fk_err),
            CoreError::Validation("Registro relacionado não encontrado ou inválido.".to_string())
        );

        // 3. PoolTimedOut
        let pool_timeout = sqlx::Error::PoolTimedOut;
        assert!(matches!(
            map_sqlx_err(pool_timeout),
            CoreError::TransientConflict(_)
        ));

        // 4. Sem código ou fallback contains("busy") removido:
        // Mensagem de texto "busy" sem código numérico NÃO vira TransientConflict,
        // e nunca vaza texto cru SQL para o usuário.
        let msg_busy = make_db_err(None, "operational error: database is busy");
        assert_eq!(
            map_sqlx_err(msg_busy),
            CoreError::Database("Erro interno de banco de dados. Operação cancelada.".to_string())
        );

        let msg_locked = make_db_err(None, "locking protocol error: table locked");
        assert_eq!(
            map_sqlx_err(msg_locked),
            CoreError::Database("Erro interno de banco de dados. Operação cancelada.".to_string())
        );

        // 5. Códigos permanentes retornam mensagem genérica segura, nunca SQL cru
        let permanent_codes = [
            ("1", "syntax error in SQL statement"),
            ("14", "unable to open database file"),
            ("8", "attempt to write a readonly database"),
        ];

        for (code, msg) in permanent_codes {
            let err = make_db_err(Some(code), msg);
            let mapped = map_sqlx_err(err);
            assert_eq!(
                mapped,
                CoreError::Database("Erro interno de banco de dados. Operação cancelada.".to_string()),
                "Código permanente {code} ({msg}) deveria virar CoreError::Database genérico, mas virou: {mapped:?}"
            );
        }

        // 6. Outros tipos de sqlx::Error que não são de banco
        let row_not_found = sqlx::Error::RowNotFound;
        assert_eq!(
            map_sqlx_err(row_not_found),
            CoreError::Database("Erro interno de banco de dados. Operação cancelada.".to_string())
        );
    }

    #[tokio::test]
    async fn test_save_branding_audit_log_transactional_and_never_logs_image_data() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao criar banco SQLite em memória");
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
                    company_name: "Locadora Audit Test".to_string(),
                    owner_name: "Ana Dono".to_string(),
                    owner_email: "ana.audit@empresa.com".to_string(),
                    owner_password: "SenhaSegura#2026".to_string(),
                },
                30,
            )
            .await
            .unwrap();

        let png_data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=";
        let update_input = UpdateBrandingInput {
            company_name: "Locadora Visual Novo".to_string(),
            primary_color: "#1e40af".to_string(),
            secondary_color: "#3b82f6".to_string(),
            accent_color: "#f59e0b".to_string(),
            logo_url: Some(png_data.to_string()),
            logo_dark_url: Some(png_data.to_string()),
            icon_url: Some(png_data.to_string()),
        };

        let updated = services
            .save_branding(&setup.session, &update_input)
            .await
            .expect("save_branding deve ser bem-sucedido");

        assert_eq!(updated.company_name, "Locadora Visual Novo");

        // Consulta a auditoria e comprova que NUNCA há dados binários ou Data URI gravados
        let audits = store.audit.list_by_tenant(&tenant_id, 20).await.unwrap();
        let branding_audit = audits
            .iter()
            .find(|a| a.action == "branding.updated")
            .expect("Registro de auditoria branding.updated deve existir");

        assert_eq!(branding_audit.entity_type, "tenant_branding");
        assert_eq!(branding_audit.entity_id, updated.id);
        assert_eq!(
            branding_audit.actor_user_id.as_deref(),
            Some(setup.session.user_id.as_str())
        );

        // Prova estrita: o sumário é conciso, informativo e sem payload binário
        assert!(branding_audit
            .summary
            .contains("Identidade visual atualizada para 'Locadora Visual Novo'"));
        assert!(branding_audit.summary.contains("logo_claro: true"));
        assert!(branding_audit.summary.contains("logo_escuro: true"));
        assert!(branding_audit.summary.contains("icone: true"));
        assert!(!branding_audit.summary.contains("data:image"));
        assert!(!branding_audit.summary.contains("base64"));
        assert!(!branding_audit.summary.contains("iVBORw0KGgo"));
        assert!(branding_audit.summary.len() < 300);
    }

    #[tokio::test]
    async fn test_benchmark_20k_customers_search_and_pagination() {
        use std::time::Instant;

        let pool = init_memory_db()
            .await
            .expect("Falha ao criar banco SQLite em memória");
        let store = SqliteStore::new(pool.clone()).into_bundle();
        let tenant_id = ensure_default_tenant(store.tenants.as_ref(), store.branding.as_ref())
            .await
            .unwrap();

        let insert_start = Instant::now();
        let mut tx = pool.begin().await.expect("Falha ao abrir transação");
        let now = "2026-11-11T12:00:00Z";

        for i in 0..20_000usize {
            let id = format!("cust-bench-{i:05}");
            let person_type = if i % 3 == 0 { "individual" } else { "legal" };
            let name = format!("Empresa Evento Audiovisual {i:05}");
            let trade_name = format!("Evento {i:05}");
            let doc = format!("{:014}", 10_000_000_000_000u64 + i as u64);
            let search_text = format!(
                "empresa evento audiovisual {i:05} evento {i:05} {doc} contato{i}@eventos.com.br"
            );
            let name_sort = format!("empresa evento audiovisual {i:05}");

            sqlx::query(
                r#"
                INSERT INTO customer (
                    id, tenant_id, person_type, name, trade_name, document_type,
                    document_number, state_registration, municipal_registration, notes,
                    active, search_text, name_sort, created_at, updated_at, deleted_at, created_by
                ) VALUES (?, ?, ?, ?, ?, 'cnpj', ?, NULL, NULL, NULL, 1, ?, ?, ?, ?, NULL, NULL)
                "#,
            )
            .bind(&id)
            .bind(&tenant_id)
            .bind(person_type)
            .bind(&name)
            .bind(&trade_name)
            .bind(&doc)
            .bind(&search_text)
            .bind(&name_sort)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .expect("Falha ao inserir cliente de benchmark");
        }
        tx.commit().await.expect("Falha ao comitar lote de 20k");
        let insert_ms = insert_start.elapsed().as_millis();

        // 1. Primeira página (limit 50)
        let page1_start = Instant::now();
        let page1 = store
            .customers
            .list(
                &tenant_id,
                &CustomerFilter {
                    query: None,
                    person_type: None,
                    role_filter: eventrent_core::domain::CustomerRoleFilter::All,
                    cursor: None,
                    limit: Some(50),
                },
            )
            .await
            .expect("Falha ao listar primeira página");
        let page1_ms = page1_start.elapsed().as_millis();

        assert_eq!(page1.items.len(), 50);
        assert!(page1.has_more);
        assert!(page1.next_cursor.is_some());
        assert!(
            page1_ms < 500,
            "Primeira página demorou {page1_ms}ms (limite 500ms)"
        );

        // 2. Segunda página via cursor
        let page2_start = Instant::now();
        let page2 = store
            .customers
            .list(
                &tenant_id,
                &CustomerFilter {
                    query: None,
                    person_type: None,
                    role_filter: eventrent_core::domain::CustomerRoleFilter::All,
                    cursor: page1.next_cursor.clone(),
                    limit: Some(50),
                },
            )
            .await
            .expect("Falha ao listar segunda página via cursor");
        let page2_ms = page2_start.elapsed().as_millis();

        assert_eq!(page2.items.len(), 50);
        assert_ne!(page2.items[0].id, page1.items[0].id);
        assert!(
            page2_ms < 500,
            "Segunda página por cursor demorou {page2_ms}ms (limite 500ms)"
        );

        // 3. Busca textual multi-palavra em 20.000 registros
        let search_start = Instant::now();
        let search_res = store
            .customers
            .list(
                &tenant_id,
                &CustomerFilter {
                    query: Some("empresa 15432".to_string()),
                    person_type: None,
                    role_filter: eventrent_core::domain::CustomerRoleFilter::All,
                    cursor: None,
                    limit: Some(50),
                },
            )
            .await
            .expect("Falha na busca entre 20.000 clientes");
        let search_ms = search_start.elapsed().as_millis();

        assert_eq!(search_res.items.len(), 1);
        assert_eq!(search_res.items[0].id, "cust-bench-15432");
        assert!(
            search_ms < 500,
            "Busca textual em 20k clientes demorou {search_ms}ms (limite 500ms)"
        );

        println!(
            "[BENCHMARK 20K CLIENTES] insert={insert_ms}ms, page1={page1_ms}ms, page2_cursor={page2_ms}ms, search={search_ms}ms"
        );
    }

    #[tokio::test]
    async fn test_migration_0005_rebuilds_and_preserves_existing_customer_data() {
        let db_path = std::env::temp_dir().join(format!("eventrent_mig0005_{}.db", Uuid::now_v7()));
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("connect sqlite");

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await
            .expect("fk on");

        // Aplica migrações 0001..0004 manualmente para simular banco existente antes da 0005
        for mig in [
            include_str!("../migrations/0001_init_tenant_and_branding.sql"),
            include_str!("../migrations/0002_users_roles_and_audit.sql"),
            include_str!("../migrations/0003_branding_dark_logo_and_icon.sql"),
            include_str!("../migrations/0004_customers.sql"),
        ] {
            sqlx::raw_sql(mig)
                .execute(&pool)
                .await
                .expect("apply 0001..0004");
        }

        let now = "2026-11-12T10:00:00Z";
        sqlx::query("INSERT INTO tenant (id, tenant_id, name, slug, created_at, updated_at) VALUES ('t1', 't1', 'Locadora', 'locadora', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.expect("insert tenant");
        sqlx::query("INSERT INTO customer (id, tenant_id, person_type, name, document_number, search_text, name_sort, created_at, updated_at) VALUES ('c1', 't1', 'individual', 'Maria Silva', '52998224725', 'maria silva', 'maria silva', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.expect("insert customer");
        sqlx::query("INSERT INTO customer_contact (id, customer_id, name, email, is_primary, created_at, updated_at) VALUES ('cc1', 'c1', 'Maria', 'maria@ex.com', 1, ?, ?)")
            .bind(now).bind(now).execute(&pool).await.expect("insert contact");
        sqlx::query("INSERT INTO customer_address (id, customer_id, label, street, number, neighborhood, city, state, postal_code, is_primary, created_at, updated_at) VALUES ('ca1', 'c1', 'Sede', 'Rua A', '10', 'Centro', 'SP', 'SP', '01001000', 1, ?, ?)")
            .bind(now).bind(now).execute(&pool).await.expect("insert address");

        // Executa a 0005 preservando dados
        sqlx::raw_sql(include_str!("../migrations/0005_customers_rev.sql"))
            .execute(&pool)
            .await
            .expect("apply 0005");

        let row = sqlx::query(
            "SELECT document_type, document_number, active FROM customer WHERE id = 'c1'",
        )
        .fetch_one(&pool)
        .await
        .expect("fetch c1");
        let doc_type: String = row.get("document_type");
        let doc_num: Option<String> = row.get("document_number");
        let active: i64 = row.get("active");
        assert_eq!(doc_type, "cpf");
        assert_eq!(doc_num.as_deref(), Some("52998224725"));
        assert_eq!(active, 1);

        let crow = sqlx::query("SELECT tenant_id FROM customer_contact WHERE id = 'cc1'")
            .fetch_one(&pool)
            .await
            .expect("fetch cc1");
        let c_tenant: String = crow.get("tenant_id");
        assert_eq!(c_tenant, "t1");

        let arow = sqlx::query("SELECT tenant_id, country FROM customer_address WHERE id = 'ca1'")
            .fetch_one(&pool)
            .await
            .expect("fetch ca1");
        let a_tenant: String = arow.get("tenant_id");
        let a_country: String = arow.get("country");
        assert_eq!(a_tenant, "t1");
        assert_eq!(a_country, "BR");

        // EXPLAIN QUERY PLAN da busca por trecho (sem idx_customer_tenant_search)
        let plan_rows = sqlx::query(
            "EXPLAIN QUERY PLAN SELECT id FROM customer WHERE tenant_id = 't1' AND deleted_at IS NULL AND search_text LIKE '%maria%' ORDER BY name_sort ASC, id ASC LIMIT 51",
        )
        .fetch_all(&pool)
        .await
        .expect("explain query plan");
        for pr in &plan_rows {
            let detail: String = pr.get("detail");
            println!("[EXPLAIN QUERY PLAN] {detail}");
            assert!(!detail.contains("idx_customer_tenant_search"));
        }

        // Validação da FK composta (tenant_id, customer_id):
        // Tentativa de inserir contato ou endereço com tenant diferente do cliente deve FALHAR
        sqlx::query("INSERT INTO tenant (id, tenant_id, name, slug, created_at, updated_at) VALUES ('t2', 't2', 'Outra', 'outra', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.expect("insert tenant t2");

        let err_contact = sqlx::query("INSERT INTO customer_contact (id, tenant_id, customer_id, name, email, is_primary, created_at, updated_at) VALUES ('cc_inv', 't2', 'c1', 'Invasor', 'inv@ex.com', 0, ?, ?)")
            .bind(now).bind(now).execute(&pool).await;
        assert!(
            err_contact.is_err(),
            "Inserir contato com tenant diferente do cliente deve falhar por FK composta"
        );

        let err_address = sqlx::query("INSERT INTO customer_address (id, tenant_id, customer_id, label, street, number, neighborhood, city, state, postal_code, is_primary, created_at, updated_at) VALUES ('ca_inv', 't2', 'c1', 'Invasor', 'Rua B', '20', 'Bairro', 'SP', 'SP', '01001000', 0, ?, ?)")
            .bind(now).bind(now).execute(&pool).await;
        assert!(
            err_address.is_err(),
            "Inserir endereço com tenant diferente do cliente deve falhar por FK composta"
        );
    }

    #[tokio::test]
    async fn test_store_atomic_swap_of_primary_contact_and_address() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao inicializar banco em memória");
        let store = SqliteStore::new(pool.clone()).into_bundle();

        let tenant_id = "tenant-swap-001";
        let now = chrono::Utc::now().to_rfc3339();

        let tenant = Tenant {
            id: tenant_id.to_string(),
            tenant_id: tenant_id.to_string(),
            name: "Empresa Swap".to_string(),
            slug: "empresa-swap".to_string(),
            document_number: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };
        store.tenants.create(&tenant).await.expect("create tenant");

        let cust_id = "cust-swap-1";
        let customer = Customer {
            id: cust_id.to_string(),
            tenant_id: tenant_id.to_string(),
            person_type: PersonType::Individual,
            name: "Cliente Swap".to_string(),
            trade_name: None,
            document_type: DocumentType::Cpf,
            document_number: Some("52998224725".to_string()),
            state_registration: None,
            municipal_registration: None,
            notes: None,
            active: true,
            is_customer: true,
            is_supplier: false,
            search_text: "cliente swap".to_string(),
            name_sort: "cliente swap".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        let contact_a = CustomerContact {
            id: "ct-swap-a".to_string(),
            tenant_id: tenant_id.to_string(),
            customer_id: cust_id.to_string(),
            name: "Contato A".to_string(),
            email: Some("a@swap.com".to_string()),
            phone: None,
            role: None,
            is_primary: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };
        let contact_b = CustomerContact {
            id: "ct-swap-b".to_string(),
            tenant_id: tenant_id.to_string(),
            customer_id: cust_id.to_string(),
            name: "Contato B".to_string(),
            email: Some("b@swap.com".to_string()),
            phone: None,
            role: None,
            is_primary: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        let addr_a = CustomerAddress {
            id: "ad-swap-a".to_string(),
            tenant_id: tenant_id.to_string(),
            customer_id: cust_id.to_string(),
            label: "Endereço A".to_string(),
            street: "Rua A".to_string(),
            number: "10".to_string(),
            complement: None,
            neighborhood: "Centro".to_string(),
            city: "SP".to_string(),
            state: "SP".to_string(),
            postal_code: "01001000".to_string(),
            country: "BR".to_string(),
            is_primary: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };
        let addr_b = CustomerAddress {
            id: "ad-swap-b".to_string(),
            tenant_id: tenant_id.to_string(),
            customer_id: cust_id.to_string(),
            label: "Endereço B".to_string(),
            street: "Rua B".to_string(),
            number: "20".to_string(),
            complement: None,
            neighborhood: "Centro".to_string(),
            city: "SP".to_string(),
            state: "SP".to_string(),
            postal_code: "01001000".to_string(),
            country: "BR".to_string(),
            is_primary: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        store
            .customers
            .create(
                &customer,
                &[contact_a.clone(), contact_b.clone()],
                &[addr_a.clone(), addr_b.clone()],
                &[],
            )
            .await
            .expect("create customer");

        // Troca atômica de principal: Contact B vem ANTES de A no slice e é marcado como is_primary: true
        // Address B vem ANTES de A e é marcado como is_primary: true
        let mut new_contact_b = contact_b.clone();
        new_contact_b.is_primary = true;
        let mut new_contact_a = contact_a.clone();
        new_contact_a.is_primary = false;

        let mut new_addr_b = addr_b.clone();
        new_addr_b.is_primary = true;
        let mut new_addr_a = addr_a.clone();
        new_addr_a.is_primary = false;

        let swap_res = store
            .customers
            .update(
                &customer,
                &[new_contact_b, new_contact_a],
                &[new_addr_b, new_addr_a],
                &[],
            )
            .await;
        assert!(
            swap_res.is_ok(),
            "Troca atômica de contatos e endereços principais na mesma transação deve funcionar sem violar índice parcial: {:?}",
            swap_res
        );

        let details = store
            .customers
            .find_by_id(tenant_id, cust_id)
            .await
            .expect("find")
            .expect("exists");
        let found_a = details
            .contacts
            .iter()
            .find(|c| c.id == "ct-swap-a")
            .unwrap();
        let found_b = details
            .contacts
            .iter()
            .find(|c| c.id == "ct-swap-b")
            .unwrap();
        assert!(!found_a.is_primary);
        assert!(found_b.is_primary);

        let found_ad_a = details
            .addresses
            .iter()
            .find(|a| a.id == "ad-swap-a")
            .unwrap();
        let found_ad_b = details
            .addresses
            .iter()
            .find(|a| a.id == "ad-swap-b")
            .unwrap();
        assert!(!found_ad_a.is_primary);
        assert!(found_ad_b.is_primary);
    }

    #[tokio::test]
    async fn test_store_update_soft_deletes_contact_and_creates_new_primary() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao inicializar banco em memória");
        let store = SqliteStore::new(pool.clone()).into_bundle();

        let tenant_id = "tenant-soft-001";
        let now = chrono::Utc::now().to_rfc3339();

        let tenant = Tenant {
            id: tenant_id.to_string(),
            tenant_id: tenant_id.to_string(),
            name: "Empresa Soft".to_string(),
            slug: "empresa-soft".to_string(),
            document_number: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };
        store.tenants.create(&tenant).await.expect("create tenant");

        let cust_id = "cust-soft-1";
        let customer = Customer {
            id: cust_id.to_string(),
            tenant_id: tenant_id.to_string(),
            person_type: PersonType::Individual,
            name: "Cliente Soft".to_string(),
            trade_name: None,
            document_type: DocumentType::Cpf,
            document_number: Some("52998224725".to_string()),
            state_registration: None,
            municipal_registration: None,
            notes: None,
            active: true,
            is_customer: true,
            is_supplier: false,
            search_text: "cliente soft".to_string(),
            name_sort: "cliente soft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        let contact_old = CustomerContact {
            id: "ct-old".to_string(),
            tenant_id: tenant_id.to_string(),
            customer_id: cust_id.to_string(),
            name: "Antigo".to_string(),
            email: Some("antigo@ex.com".to_string()),
            phone: None,
            role: None,
            is_primary: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        store
            .customers
            .create(&customer, &[contact_old], &[], &[])
            .await
            .expect("create");

        // Atualização passando novo contato C (não inclui ct-old, provocando soft delete)
        let contact_new = CustomerContact {
            id: "ct-new".to_string(),
            tenant_id: tenant_id.to_string(),
            customer_id: cust_id.to_string(),
            name: "Novo".to_string(),
            email: Some("novo@ex.com".to_string()),
            phone: None,
            role: None,
            is_primary: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        store
            .customers
            .update(&customer, &[contact_new], &[], &[])
            .await
            .expect("update");

        let details = store
            .customers
            .find_by_id(tenant_id, cust_id)
            .await
            .expect("find")
            .expect("exists");
        assert_eq!(details.contacts.len(), 1);
        assert_eq!(details.contacts[0].id, "ct-new");
        assert!(details.contacts[0].is_primary);

        // Verifica no banco cru que ct-old teve soft delete executado
        let row_old = sqlx::query("SELECT deleted_at FROM customer_contact WHERE id = 'ct-old'")
            .fetch_one(&pool)
            .await
            .expect("fetch old");
        let del: Option<String> = row_old.get("deleted_at");
        assert!(
            del.is_some(),
            "Contato removido da lista deve ter deleted_at preenchido"
        );
    }

    #[tokio::test]
    async fn test_store_customer_tenant_isolation() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao inicializar banco em memória");
        let store = SqliteStore::new(pool.clone()).into_bundle();

        let now = chrono::Utc::now().to_rfc3339();

        for tid in ["tenant-iso-1", "tenant-iso-2"] {
            store
                .tenants
                .create(&Tenant {
                    id: tid.to_string(),
                    tenant_id: tid.to_string(),
                    name: format!("Empresa {tid}"),
                    slug: tid.to_string(),
                    document_number: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    deleted_at: None,
                    created_by: None,
                })
                .await
                .expect("create tenant");
        }

        let customer1 = Customer {
            id: "cust-iso-1".to_string(),
            tenant_id: "tenant-iso-1".to_string(),
            person_type: PersonType::Individual,
            name: "Cliente Tenant 1".to_string(),
            trade_name: None,
            document_type: DocumentType::Cpf,
            document_number: Some("52998224725".to_string()),
            state_registration: None,
            municipal_registration: None,
            notes: None,
            active: true,
            is_customer: true,
            is_supplier: false,
            search_text: "cliente tenant 1".to_string(),
            name_sort: "cliente tenant 1".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };

        store
            .customers
            .create(&customer1, &[], &[], &[])
            .await
            .expect("create cust1");

        // 1. find_by_id isolado por tenant
        assert!(store
            .customers
            .find_by_id("tenant-iso-2", "cust-iso-1")
            .await
            .unwrap()
            .is_none());
        assert!(store
            .customers
            .find_by_id("tenant-iso-1", "cust-iso-1")
            .await
            .unwrap()
            .is_some());

        // 2. find_by_document isolado por tenant (Tenant 2 não enxerga documento de Tenant 1)
        assert!(store
            .customers
            .find_by_document("tenant-iso-2", DocumentType::Cpf, "52998224725")
            .await
            .unwrap()
            .is_none());
        assert!(store
            .customers
            .find_by_document("tenant-iso-1", DocumentType::Cpf, "52998224725")
            .await
            .unwrap()
            .is_some());

        // 3. list isolado por tenant
        let list_t2 = store
            .customers
            .list(
                "tenant-iso-2",
                &CustomerFilter {
                    query: None,
                    person_type: None,
                    role_filter: eventrent_core::domain::CustomerRoleFilter::All,
                    cursor: None,
                    limit: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(list_t2.items.len(), 0);

        let list_t1 = store
            .customers
            .list(
                "tenant-iso-1",
                &CustomerFilter {
                    query: None,
                    person_type: None,
                    role_filter: eventrent_core::domain::CustomerRoleFilter::All,
                    cursor: None,
                    limit: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(list_t1.items.len(), 1);

        // 4. set_active em outro tenant não afeta tenant-iso-1
        store
            .customers
            .set_active("tenant-iso-2", "cust-iso-1", false, &now)
            .await
            .unwrap();
        let c1_still_active = store
            .customers
            .find_by_id("tenant-iso-1", "cust-iso-1")
            .await
            .unwrap()
            .unwrap();
        assert!(c1_still_active.customer.active);

        // 5. soft_delete em outro tenant não afeta tenant-iso-1
        store
            .customers
            .soft_delete("tenant-iso-2", "cust-iso-1", &now)
            .await
            .unwrap();
        let c1_still_exists = store
            .customers
            .find_by_id("tenant-iso-1", "cust-iso-1")
            .await
            .unwrap();
        assert!(c1_still_exists.is_some());
    }

    #[tokio::test]
    async fn test_store_suppliers_roles_categories_fk_and_isolation() {
        let pool = init_memory_db()
            .await
            .expect("Falha ao inicializar banco em memória");
        let store = SqliteStore::new(pool.clone()).into_bundle();
        let now = Utc::now().to_rfc3339();

        for tid in ["tenant-sup-1", "tenant-sup-2"] {
            store
                .tenants
                .create(&Tenant {
                    id: tid.to_string(),
                    tenant_id: tid.to_string(),
                    name: format!("Empresa {tid}"),
                    slug: tid.to_string(),
                    document_number: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    deleted_at: None,
                    created_by: None,
                })
                .await
                .unwrap();
        }

        // 1. Cria fornecedor puro no tenant 1 com 2 categorias
        let supplier = Customer {
            id: "sup-1".to_string(),
            tenant_id: "tenant-sup-1".to_string(),
            person_type: PersonType::Legal,
            name: "Locadora Parceira Ltda".to_string(),
            trade_name: None,
            document_type: DocumentType::None,
            document_number: None,
            state_registration: None,
            municipal_registration: None,
            notes: None,
            active: true,
            is_customer: false,
            is_supplier: true,
            search_text: "locadora parceira ltda".to_string(),
            name_sort: "locadora parceira ltda".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };
        let cats = vec![
            eventrent_core::domain::SupplierCategory::Sublocacao,
            eventrent_core::domain::SupplierCategory::Transporte,
        ];
        store
            .customers
            .create(&supplier, &[], &[], &cats)
            .await
            .unwrap();

        let loaded = store
            .customers
            .find_by_id("tenant-sup-1", "sup-1")
            .await
            .unwrap()
            .expect("Fornecedor deve existir no tenant 1");
        assert!(!loaded.customer.is_customer);
        assert!(loaded.customer.is_supplier);
        assert_eq!(loaded.supplier_categories.len(), 2);

        // 2. Atualiza categorias (remove Transporte, adiciona Manutencao e mantém Sublocacao)
        let new_cats = vec![
            eventrent_core::domain::SupplierCategory::Sublocacao,
            eventrent_core::domain::SupplierCategory::Manutencao,
        ];
        store
            .customers
            .update(&supplier, &[], &[], &new_cats)
            .await
            .unwrap();
        let reloaded = store
            .customers
            .find_by_id("tenant-sup-1", "sup-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reloaded.supplier_categories.len(), 2);
        assert!(reloaded
            .supplier_categories
            .contains(&eventrent_core::domain::SupplierCategory::Manutencao));
        assert!(!reloaded
            .supplier_categories
            .contains(&eventrent_core::domain::SupplierCategory::Transporte));

        // 3. Filtro por papel (Supplier encontra, Customer não encontra)
        let sup_list = store
            .customers
            .list(
                "tenant-sup-1",
                &CustomerFilter {
                    role_filter: eventrent_core::domain::CustomerRoleFilter::Supplier,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(sup_list.items.len(), 1);

        let cust_list = store
            .customers
            .list(
                "tenant-sup-1",
                &CustomerFilter {
                    role_filter: eventrent_core::domain::CustomerRoleFilter::Customer,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(cust_list.items.len(), 0);

        // 4. Isolamento por tenant
        let sup_list_t2 = store
            .customers
            .list(
                "tenant-sup-2",
                &CustomerFilter {
                    role_filter: eventrent_core::domain::CustomerRoleFilter::Supplier,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(sup_list_t2.items.len(), 0);

        // 5. CHECK (is_customer = 1 OR is_supplier = 1) no banco
        let check_err = sqlx::query(
            "INSERT INTO customer (id, tenant_id, person_type, name, is_customer, is_supplier, search_text, name_sort, created_at, updated_at) VALUES ('bad-role', 'tenant-sup-1', 'legal', 'Sem Papel', 0, 0, 'sem papel', 'sem papel', ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(
            check_err.is_err(),
            "CHECK deve impedir cadastro sem cliente e sem fornecedor"
        );

        // 6. FK composta (tenant_id, customer_id) em customer_supplier_category
        let fk_err = sqlx::query(
            "INSERT INTO customer_supplier_category (id, tenant_id, customer_id, category, created_at, updated_at) VALUES ('cat-cross', 'tenant-sup-2', 'sup-1', 'sublocacao', ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(
            fk_err.is_err(),
            "FK composta deve impedir categoria com tenant diferente do cliente"
        );
    }

    #[tokio::test]
    async fn test_store_venues_fk_unique_spaces_contacts_and_isolation() {
        use eventrent_core::domain::{
            CreateVenueInput, SessionInfo, UpdateVenueInput, VenueContactInput, VenueFilter,
            VenueSpaceInput,
        };

        let pool = init_memory_db().await.unwrap();
        let store = SqliteStore::new(pool.clone()).into_bundle();
        let services = CoreServices::new(store.clone());
        let now = Utc::now().to_rfc3339();

        // Cria dois tenants distintos
        for tid in ["tenant-v1", "tenant-v2"] {
            store
                .tenants
                .create(&Tenant {
                    id: tid.to_string(),
                    tenant_id: tid.to_string(),
                    name: format!("Empresa {tid}"),
                    slug: tid.to_string(),
                    document_number: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    deleted_at: None,
                    created_by: None,
                })
                .await
                .unwrap();
        }
        sqlx::query(
            "INSERT INTO role (id, tenant_id, name, slug, is_system, created_at, updated_at) VALUES ('role-owner-v1', 'tenant-v1', 'Proprietário', 'owner', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO app_user (id, tenant_id, role_id, name, email, password_hash, active, created_at, updated_at) VALUES ('user-owner-v1', 'tenant-v1', 'role-owner-v1', 'Owner V1', 'owner@v1.com', 'hash', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        let owner_session_t1 = SessionInfo {
            user_id: "user-owner-v1".to_string(),
            tenant_id: "tenant-v1".to_string(),
            name: "Owner V1".to_string(),
            email: "owner@v1.com".to_string(),
            role_id: "role-owner".to_string(),
            role_name: "Proprietário".to_string(),
            role_slug: "owner".to_string(),
            permissions: vec!["venue:read".to_string(), "venue:manage".to_string()],
            authenticated_at: now.clone(),
            last_activity_at: now.clone(),
            inactivity_timeout_minutes: 30,
        };

        let tech_session_t1 = SessionInfo {
            permissions: vec!["venue:read".to_string()],
            role_slug: "technician".to_string(),
            ..owner_session_t1.clone()
        };

        let owner_session_t2 = SessionInfo {
            tenant_id: "tenant-v2".to_string(),
            ..owner_session_t1.clone()
        };

        // 1. Técnico não pode criar local (Forbidden)
        let created_by_tech = services
            .create_venue(
                &tech_session_t1,
                &CreateVenueInput {
                    name: "Bloqueado".to_string(),
                    street: None,
                    number: None,
                    complement: None,
                    neighborhood: None,
                    city: None,
                    state: None,
                    postal_code: None,
                    country: "BR".to_string(),
                    parking_notes: None,
                    loading_dock_notes: None,
                    access_hours: None,
                    internal_transfer_minutes: None,
                    notes: None,
                    spaces: vec![],
                    contacts: vec![],
                },
            )
            .await;
        assert!(matches!(created_by_tech, Err(CoreError::Forbidden(_))));

        // 2. Cria Riocentro com 2 espaços e 1 contato
        let created = services
            .create_venue(
                &owner_session_t1,
                &CreateVenueInput {
                    name: "Riocentro".to_string(),
                    street: Some("Av. Salvador Allende".to_string()),
                    number: Some("6555".to_string()),
                    complement: None,
                    neighborhood: Some("Barra da Tijuca".to_string()),
                    city: Some("Rio de Janeiro".to_string()),
                    state: Some("RJ".to_string()),
                    postal_code: Some("22783115".to_string()),
                    country: "BR".to_string(),
                    parking_notes: None,
                    loading_dock_notes: None,
                    access_hours: None,
                    internal_transfer_minutes: None,
                    notes: None,
                    spaces: vec![
                        VenueSpaceInput {
                            id: Some("id-forjado".to_string()),
                            name: "Pavilhão 1".to_string(),
                            capacity_people: Some(3000),
                            area_sqm: Some(12000.0),
                            ceiling_height_m: Some(12.0),
                            power_available: Some("380V".to_string()),
                            loading_access: None,
                            rigging_points: None,
                            acoustics_notes: None,
                            setup_hours_notes: None,
                            noise_restrictions_notes: None,
                            load_dimensions_notes: None,
                            notes: None,
                            active: true,
                            sort_order: 0,
                        },
                        VenueSpaceInput {
                            id: None,
                            name: "Pavilhão 2".to_string(),
                            capacity_people: Some(5000),
                            area_sqm: None,
                            ceiling_height_m: None,
                            power_available: None,
                            loading_access: None,
                            rigging_points: None,
                            acoustics_notes: None,
                            setup_hours_notes: None,
                            noise_restrictions_notes: None,
                            load_dimensions_notes: None,
                            notes: None,
                            active: true,
                            sort_order: 1,
                        },
                    ],
                    contacts: vec![VenueContactInput {
                        id: None,
                        name: "Marcos Doca".to_string(),
                        email: Some("doca@riocentro.com.br".to_string()),
                        phone: Some("21999887766".to_string()),
                        role: Some("Operações".to_string()),
                        is_primary: false,
                    }],
                },
            )
            .await
            .unwrap();

        assert_eq!(created.spaces.len(), 2);
        assert_ne!(
            created.spaces[0].id, "id-forjado",
            "ID enviado na criação deve ser ignorado"
        );
        assert!(created.contacts[0].is_primary);
        assert_eq!(created.contacts[0].phone.as_deref(), Some("+5521999887766"));

        let p1_id = created.spaces[0].id.clone();

        // 3. Técnico pode ler e listar
        let read_by_tech = services
            .get_venue(&tech_session_t1, &created.venue.id)
            .await
            .unwrap();
        assert_eq!(read_by_tech.spaces.len(), 2);

        // 4. Atualização preserva ID estável de Pavilhão 1, remove Pavilhão 2 e adiciona Pavilhão 3
        let updated = services
            .update_venue(
                &owner_session_t1,
                &UpdateVenueInput {
                    id: created.venue.id.clone(),
                    name: "Riocentro".to_string(),
                    street: created.venue.street.clone(),
                    number: created.venue.number.clone(),
                    complement: None,
                    neighborhood: created.venue.neighborhood.clone(),
                    city: created.venue.city.clone(),
                    state: created.venue.state.clone(),
                    postal_code: created.venue.postal_code.clone(),
                    country: "BR".to_string(),
                    parking_notes: None,
                    loading_dock_notes: None,
                    access_hours: None,
                    internal_transfer_minutes: Some(0),
                    notes: None,
                    spaces: vec![
                        VenueSpaceInput {
                            id: Some(p1_id.clone()),
                            name: "Pavilhão 1 Atualizado".to_string(),
                            capacity_people: Some(3500),
                            area_sqm: None,
                            ceiling_height_m: None,
                            power_available: None,
                            loading_access: None,
                            rigging_points: None,
                            acoustics_notes: None,
                            setup_hours_notes: None,
                            noise_restrictions_notes: None,
                            load_dimensions_notes: None,
                            notes: None,
                            active: true,
                            sort_order: 0,
                        },
                        VenueSpaceInput {
                            id: None,
                            name: "Pavilhão 3".to_string(),
                            capacity_people: Some(2000),
                            area_sqm: None,
                            ceiling_height_m: None,
                            power_available: None,
                            loading_access: None,
                            rigging_points: None,
                            acoustics_notes: None,
                            setup_hours_notes: None,
                            noise_restrictions_notes: None,
                            load_dimensions_notes: None,
                            notes: None,
                            active: true,
                            sort_order: 1,
                        },
                    ],
                    contacts: vec![],
                    expected_updated_at: created.venue.updated_at.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.spaces[0].id, p1_id);
        assert_eq!(updated.spaces[0].name, "Pavilhão 1 Atualizado");
        assert_eq!(updated.spaces.len(), 2);

        // Concorrência otimista: usar updated_at antigo deve retornar Conflict
        let conflict = services
            .update_venue(
                &owner_session_t1,
                &UpdateVenueInput {
                    id: created.venue.id.clone(),
                    name: "Riocentro".to_string(),
                    street: None,
                    number: None,
                    complement: None,
                    neighborhood: None,
                    city: None,
                    state: None,
                    postal_code: None,
                    country: "BR".to_string(),
                    parking_notes: None,
                    loading_dock_notes: None,
                    access_hours: None,
                    internal_transfer_minutes: None,
                    notes: None,
                    spaces: vec![],
                    contacts: vec![],
                    expected_updated_at: created.venue.updated_at.clone(),
                },
            )
            .await;
        assert!(matches!(conflict, Err(CoreError::Conflict(_))));

        // 5. Isolamento multi-tenant
        assert!(matches!(
            services
                .get_venue(&owner_session_t2, &created.venue.id)
                .await,
            Err(CoreError::NotFound(_))
        ));
        let t2_list = services
            .list_venues(&owner_session_t2, &VenueFilter::default())
            .await
            .unwrap();
        assert_eq!(t2_list.items.len(), 0);

        // 5b. Limites de busca em list_venues (100 caracteres e 8 termos)
        let too_long_query = services
            .list_venues(
                &owner_session_t1,
                &VenueFilter {
                    query: Some("a".repeat(101)),
                    ..Default::default()
                },
            )
            .await;
        assert!(matches!(too_long_query, Err(CoreError::Validation(_))));

        let too_many_terms = services
            .list_venues(
                &owner_session_t1,
                &VenueFilter {
                    query: Some("t1 t2 t3 t4 t5 t6 t7 t8 t9".to_string()),
                    ..Default::default()
                },
            )
            .await;
        assert!(matches!(too_many_terms, Err(CoreError::Validation(_))));

        // 6. FK composta no banco impede venue_space / venue_contact com tenant divergente
        let fk_space_err = sqlx::query(
            "INSERT INTO venue_space (id, tenant_id, venue_id, name, name_normalized, active, sort_order, created_at, updated_at) VALUES ('sp-cross', 'tenant-v2', ?, 'Invasor', 'invasor', 1, 0, ?, ?)",
        )
        .bind(&created.venue.id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(fk_space_err.is_err());

        let fk_contact_err = sqlx::query(
            "INSERT INTO venue_contact (id, tenant_id, venue_id, name, is_primary, created_at, updated_at) VALUES ('ct-cross', 'tenant-v2', ?, 'Invasor', 0, ?, ?)",
        )
        .bind(&created.venue.id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(fk_contact_err.is_err());
    }

    #[tokio::test]
    async fn test_store_allocation_settings_idempotent_and_rbac() {
        use eventrent_core::domain::{SessionInfo, UpdateAllocationSettingsInput};

        let pool = init_memory_db().await.unwrap();
        let store = SqliteStore::new(pool.clone()).into_bundle();
        let services = CoreServices::new(store.clone());
        let now = Utc::now().to_rfc3339();

        store
            .tenants
            .create(&Tenant {
                id: "tenant-set-1".to_string(),
                tenant_id: "tenant-set-1".to_string(),
                name: "Empresa Config".to_string(),
                slug: "empresa-config".to_string(),
                document_number: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
                created_by: None,
            })
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO role (id, tenant_id, name, slug, is_system, created_at, updated_at) VALUES ('role-owner-set1', 'tenant-set-1', 'Proprietário', 'owner', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO app_user (id, tenant_id, role_id, name, email, password_hash, active, created_at, updated_at) VALUES ('user-set-1', 'tenant-set-1', 'role-owner-set1', 'Owner', 'owner@set.com', 'hash', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        let owner_session = SessionInfo {
            user_id: "user-set-1".to_string(),
            tenant_id: "tenant-set-1".to_string(),
            name: "Owner".to_string(),
            email: "owner@set.com".to_string(),
            role_id: "role-owner".to_string(),
            role_name: "Proprietário".to_string(),
            role_slug: "owner".to_string(),
            permissions: vec!["settings:read".to_string(), "settings:manage".to_string()],
            authenticated_at: now.clone(),
            last_activity_at: now.clone(),
            inactivity_timeout_minutes: 30,
        };

        let readonly_session = SessionInfo {
            permissions: vec!["settings:read".to_string()],
            ..owner_session.clone()
        };

        // 1. Primeira leitura devolve defaults em memória SEM gravar nada no banco
        let s1 = services
            .get_allocation_settings(&readonly_session)
            .await
            .unwrap();
        assert_eq!(s1.changeover_minutes, 30);
        assert_eq!(s1.internal_transfer_minutes, 20);
        assert_eq!(s1.external_transfer_minutes, 60);

        let count_after_read: i64 =
            sqlx::query("SELECT COUNT(*) as total FROM tenant_setting WHERE tenant_id = ?")
                .bind("tenant-set-1")
                .fetch_one(&pool)
                .await
                .unwrap()
                .get("total");
        assert_eq!(
            count_after_read, 0,
            "get_allocation_settings não pode gravar no banco quando chave está ausente"
        );

        // Chave antiga no banco deve ser ignorada silenciosamente
        sqlx::query(
            "INSERT INTO tenant_setting (id, tenant_id, setting_key, setting_value, updated_at) VALUES ('old-1', 'tenant-set-1', 'allocation.default_setup_minutes', '99', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        let internal_s1 = services
            .get_allocation_settings_for_tenant("tenant-set-1")
            .await
            .unwrap();
        assert_eq!(s1, internal_s1);

        // 2. Sem settings:manage não pode alterar
        let forbidden = services
            .update_allocation_settings(
                &readonly_session,
                &UpdateAllocationSettingsInput {
                    changeover_minutes: 45,
                    internal_transfer_minutes: 25,
                    external_transfer_minutes: 120,
                },
            )
            .await;
        assert!(matches!(forbidden, Err(CoreError::Forbidden(_))));

        // 3. Com settings:manage atualiza e persiste
        let updated = services
            .update_allocation_settings(
                &owner_session,
                &UpdateAllocationSettingsInput {
                    changeover_minutes: 45,
                    internal_transfer_minutes: 25,
                    external_transfer_minutes: 120,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.changeover_minutes, 45);
        assert_eq!(updated.internal_transfer_minutes, 25);
        assert_eq!(updated.external_transfer_minutes, 120);

        let reloaded = services
            .get_allocation_settings(&readonly_session)
            .await
            .unwrap();
        assert_eq!(reloaded, updated);

        // 4. Toda alteração em update_allocation_settings gera audit_log na mesma transação
        let audit_rows = sqlx::query(
            "SELECT action, entity_type, entity_id, summary FROM audit_log WHERE tenant_id = ? AND action = 'settings.allocation_updated'",
        )
        .bind("tenant-set-1")
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(audit_rows.len(), 1);
        let summary: String = audit_rows[0].get("summary");
        assert!(summary.contains("virada=45min"));
        assert!(summary.contains("deslocamento_interno=25min"));
        assert!(summary.contains("deslocamento_externo=120min"));

        // 5. Valor inválido no banco (não numérico ou acima do limite) cai para o padrão em memória com aviso no log
        sqlx::query(
            "UPDATE tenant_setting SET setting_value = 'nao-numerico' WHERE tenant_id = ? AND setting_key = 'allocation.changeover_minutes'",
        )
        .bind("tenant-set-1")
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE tenant_setting SET setting_value = '999' WHERE tenant_id = ? AND setting_key = 'allocation.external_transfer_minutes'",
        )
        .bind("tenant-set-1")
        .execute(&pool)
        .await
        .unwrap();

        let fallback = services
            .get_allocation_settings_for_tenant("tenant-set-1")
            .await
            .unwrap();
        assert_eq!(fallback.changeover_minutes, 30);
        assert_eq!(fallback.internal_transfer_minutes, 25);
        assert_eq!(fallback.external_transfer_minutes, 60);
    }

    #[tokio::test]
    async fn test_store_catalog_crud_inheritance_rbac_and_isolation() {
        use eventrent_core::{
            AttributeValueInput, CatalogListFilter, ControlMode, CreateBrandInput,
            CreateEquipmentModelInput, SessionInfo, UpdateEquipmentModelInput, VoltageOption,
        };

        let pool = init_memory_db().await.unwrap();
        let store = SqliteStore::new(pool.clone());
        let services = CoreServices::new(store.into_bundle());

        for tid in ["tenant-cat-a", "tenant-cat-b"] {
            services
                .stores
                .tenants
                .create(&Tenant {
                    id: tid.to_string(),
                    tenant_id: tid.to_string(),
                    name: format!("Empresa {tid}"),
                    slug: tid.to_string(),
                    document_number: None,
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                    deleted_at: None,
                    created_by: None,
                })
                .await
                .unwrap();
        }

        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO role (id, tenant_id, name, slug, is_system, created_at, updated_at) VALUES ('role-cat-a', 'tenant-cat-a', 'Estoque', 'warehouse', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO app_user (id, tenant_id, role_id, name, email, password_hash, active, created_at, updated_at) VALUES ('u-wh-a', 'tenant-cat-a', 'role-cat-a', 'Estoque A', 'u-wh-a@test.com', 'hash', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        let mk_session = |uid: &str, tid: &str, perms: &[&str]| {
            let now = Utc::now().to_rfc3339();
            SessionInfo {
                user_id: uid.to_string(),
                tenant_id: tid.to_string(),
                name: uid.to_string(),
                email: format!("{uid}@test.com"),
                role_id: "role-id".to_string(),
                role_name: "Role".to_string(),
                role_slug: "custom".to_string(),
                permissions: perms.iter().map(|s| s.to_string()).collect(),
                authenticated_at: now.clone(),
                last_activity_at: now,
                inactivity_timeout_minutes: 30,
            }
        };

        let warehouse_a = mk_session(
            "u-wh-a",
            "tenant-cat-a",
            &["catalog:read", "catalog:manage"],
        );
        let tech_a = mk_session("u-tech-a", "tenant-cat-a", &["catalog:read"]);
        let finance_a = mk_session("u-fin-a", "tenant-cat-a", &["customer:read"]);
        let warehouse_b = mk_session(
            "u-wh-b",
            "tenant-cat-b",
            &["catalog:read", "catalog:manage"],
        );

        // 1. RBAC: Financeiro sem catalog:read recebe Forbidden
        assert!(matches!(
            services.list_brands(&finance_a, false).await,
            Err(CoreError::Forbidden(_))
        ));
        assert!(matches!(
            services
                .list_equipment_models(&finance_a, &CatalogListFilter::default())
                .await,
            Err(CoreError::Forbidden(_))
        ));

        // 2. RBAC: Técnico (somente catalog:read) não pode carregar sugeridas nem criar marca
        assert!(matches!(
            services.load_suggested_catalog_categories(&tech_a).await,
            Err(CoreError::Forbidden(_))
        ));
        assert!(matches!(
            services
                .create_brand(
                    &tech_a,
                    &CreateBrandInput {
                        name: "Shure".to_string(),
                        active: true
                    }
                )
                .await,
            Err(CoreError::Forbidden(_))
        ));

        // 3. Carga sugerida idempotente pelo Estoque (warehouse_a)
        let first_load = services
            .load_suggested_catalog_categories(&warehouse_a)
            .await
            .unwrap();
        assert!(first_load.categories_created >= 10);
        assert!(first_load.attributes_created >= 10);

        let second_load = services
            .load_suggested_catalog_categories(&warehouse_a)
            .await
            .unwrap();
        assert_eq!(second_load.categories_created, 0);
        assert_eq!(second_load.attributes_created, 0);

        // 4. Herança: "Mesas de som" herda "impedancia_ohms" de "Áudio" + próprios "canais" e "protocolo_digital"
        let tree = services.list_categories_tree(&tech_a, false).await.unwrap();
        let audio = tree
            .iter()
            .find(|i| i.category.name == "Áudio")
            .expect("Áudio");
        let mesas = tree
            .iter()
            .find(|i| i.category.name == "Mesas de som")
            .expect("Mesas de som");
        let effective = services
            .get_category_effective_attributes(&tech_a, &mesas.category.id)
            .await
            .unwrap();
        let canais_def = effective
            .iter()
            .find(|a| a.definition.field_key == "canais")
            .expect("canais");
        let impedancia_def = effective
            .iter()
            .find(|a| a.definition.field_key == "impedancia_ohms")
            .expect("impedancia_ohms herdada");
        assert!(!canais_def.is_inherited);
        assert!(impedancia_def.is_inherited);

        // 5. Criação de marca + modelo com atributos herdados e próprios + validação EAN/NCM
        let behringer = services
            .create_brand(
                &warehouse_a,
                &CreateBrandInput {
                    name: "Behringer".to_string(),
                    active: true,
                },
            )
            .await
            .unwrap();

        let created = services
            .create_equipment_model(
                &warehouse_a,
                &CreateEquipmentModelInput {
                    category_id: mesas.category.id.clone(),
                    brand_id: Some(behringer.id.clone()),
                    name: "X32 Compact".to_string(),
                    model_code: Some("X32C".to_string()),
                    sku: Some("AUD-X32C".to_string()),
                    ean: Some("7891000315507".to_string()),
                    ncm: Some("8543.70.99".to_string()),
                    description: Some("Console digital 32 canais".to_string()),
                    control_mode: ControlMode::Serialized,
                    weight_kg: Some(15.4),
                    length_cm: Some(62.5),
                    width_cm: Some(50.6),
                    height_cm: Some(21.5),
                    power_watts: Some(120),
                    voltage: Some(VoltageOption::Bivolt),
                    amperage: Some(1.5),
                    requires_case: true,
                    units_per_case: Some(1),
                    daily_rate_cents: Some(35_000),
                    replacement_value_cents: Some(1_800_000),
                    cost_price_cents: Some(1_200_000),
                    active: true,
                    attribute_values: vec![
                        AttributeValueInput {
                            definition_id: canais_def.definition.id.clone(),
                            value_text: None,
                            value_number: Some(32.0),
                            value_bool: None,
                        },
                        AttributeValueInput {
                            definition_id: impedancia_def.definition.id.clone(),
                            value_text: None,
                            value_number: Some(600.0),
                            value_bool: None,
                        },
                    ],
                },
            )
            .await
            .unwrap();
        assert_eq!(created.attribute_values.len(), 2);
        assert_eq!(created.model.ncm.as_deref(), Some("85437099"));

        // 6. Filtro por categoria pai ("Áudio") retorna modelo em subcategoria ("Mesas de som")
        let list_by_audio = services
            .list_equipment_models(
                &tech_a,
                &CatalogListFilter {
                    category_id: Some(audio.category.id.clone()),
                    limit: 50,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(list_by_audio.total_count, 1);
        assert_eq!(list_by_audio.items[0].model.id, created.model.id);

        // 7. Unicidade parcial de SKU e proteção de exclusão de categoria/marca com modelos
        assert!(matches!(
            services.delete_brand(&warehouse_a, &behringer.id).await,
            Err(CoreError::Conflict(_))
        ));
        assert!(matches!(
            services
                .delete_category(&warehouse_a, &mesas.category.id)
                .await,
            Err(CoreError::Conflict(_))
        ));
        assert!(matches!(
            services
                .delete_category(&warehouse_a, &audio.category.id)
                .await,
            Err(CoreError::Conflict(_))
        ));

        // 8. Concorrência otimista na atualização do modelo
        let stale_update = services
            .update_equipment_model(
                &warehouse_a,
                &created.model.id,
                &UpdateEquipmentModelInput {
                    category_id: mesas.category.id.clone(),
                    brand_id: Some(behringer.id.clone()),
                    name: "X32 Compact v2".to_string(),
                    model_code: Some("X32C".to_string()),
                    sku: Some("AUD-X32C".to_string()),
                    ean: None,
                    ncm: None,
                    description: None,
                    control_mode: ControlMode::Serialized,
                    weight_kg: None,
                    length_cm: None,
                    width_cm: None,
                    height_cm: None,
                    power_watts: None,
                    voltage: None,
                    amperage: None,
                    requires_case: false,
                    units_per_case: None,
                    daily_rate_cents: Some(40_000),
                    replacement_value_cents: None,
                    cost_price_cents: None,
                    active: true,
                    attribute_values: vec![],
                    expected_updated_at: "stale-ts".to_string(),
                },
            )
            .await;
        assert!(matches!(stale_update, Err(CoreError::Conflict(_))));

        // 9. Isolamento multi-tenant + FK composta no banco
        let list_b = services
            .list_equipment_models(&warehouse_b, &CatalogListFilter::default())
            .await
            .unwrap();
        assert_eq!(list_b.total_count, 0);

        let cross_fk = sqlx::query(
            "INSERT INTO equipment_model (
                id, tenant_id, category_id, brand_id, name, name_sort,
                control_mode, requires_case, active, search_text, created_at, updated_at
             ) VALUES (?, ?, ?, NULL, 'Cross', 'cross', 'serialized', 0, 1, 'cross', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .bind("cross-model-1")
        .bind("tenant-cat-b")
        .bind(&mesas.category.id)
        .execute(&pool)
        .await;
        assert!(
            cross_fk.is_err(),
            "FK composta deve impedir vínculo cross-tenant"
        );
    }
}
