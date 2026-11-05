use async_trait::async_trait;
use chrono::Utc;
use eventrent_core::{
    AuditLogEntry, AuditStore, BrandingStore, CoreError, Result, Role, RoleStore,
    RoleWithPermissions, StoreBundle, Tenant, TenantBranding, TenantStore, UpdateBrandingInput,
    UserRecord, UserStore, UserWithRole,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub type DbPool = Pool<Sqlite>;

fn map_sqlx_err(err: sqlx::Error) -> CoreError {
    CoreError::Database(err.to_string())
}

/// Inicializa o banco de dados SQLite em arquivo usando `SqliteConnectOptions::new().filename(path)`,
/// garantindo compatibilidade com caminhos contendo espaços e acentuação no Windows, macOS e Linux.
pub async fn init_db(db_path: &Path) -> Result<DbPool> {
    let connection_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

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
        .foreign_keys(true);

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
pub struct SqliteStore {
    pool: DbPool,
}

impl SqliteStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub fn into_bundle(self) -> StoreBundle {
        let arc = Arc::new(self);
        StoreBundle {
            tenants: arc.clone(),
            branding: arc.clone(),
            roles: arc.clone(),
            users: arc.clone(),
            audit: arc,
        }
    }

    async fn load_permissions_for_role(&self, role_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT permission
            FROM role_permission
            WHERE role_id = ? AND deleted_at IS NULL
            ORDER BY permission ASC
            "#,
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(|r| r.get("permission")).collect())
    }
}

#[async_trait]
impl TenantStore for SqliteStore {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, name, slug, document_number,
                   created_at, updated_at, deleted_at, created_by
            FROM tenant
            WHERE id = ? AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, name, slug, document_number,
                   created_at, updated_at, deleted_at, created_by
            FROM tenant
            WHERE slug = ? AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        .bind(&tenant.created_by)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(tenant.clone())
    }

    async fn update_name(&self, id: &str, name: &str, updated_at: &str) -> Result<Tenant> {
        sqlx::query(
            r#"
            UPDATE tenant
            SET name = ?, updated_at = ?
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(name)
        .bind(updated_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        TenantStore::find_by_id(self, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Tenant {id}")))
    }
}

#[async_trait]
impl BrandingStore for SqliteStore {
    async fn get_by_tenant_id(&self, tenant_id: &str) -> Result<Option<TenantBranding>> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, company_name, primary_color, secondary_color,
                   accent_color, logo_url, favicon_url, created_at, updated_at,
                   deleted_at, created_by
            FROM tenant_branding
            WHERE tenant_id = ? AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
            .bind(&branding.id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;

            branding.company_name = input.company_name.clone();
            branding.primary_color = input.primary_color.clone();
            branding.secondary_color = input.secondary_color.clone();
            branding.accent_color = input.accent_color.clone();
            branding.logo_url = input.logo_url.clone();
            branding.updated_at = now;

            Ok(branding)
        } else {
            let new_id = Uuid::now_v7().to_string();

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
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;

            self.get_by_tenant_id(tenant_id)
                .await?
                .ok_or_else(|| CoreError::NotFound(format!("Tenant branding for {tenant_id}")))
        }
    }
}

#[async_trait]
impl RoleStore for SqliteStore {
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<RoleWithPermissions>> {
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, name, slug, description, is_system,
                   created_at, updated_at, deleted_at, created_by
            FROM role
            WHERE tenant_id = ? AND deleted_at IS NULL
            ORDER BY is_system DESC, name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, name, slug, description, is_system,
                   created_at, updated_at, deleted_at, created_by
            FROM role
            WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, name, slug, description, is_system,
                   created_at, updated_at, deleted_at, created_by
            FROM role
            WHERE tenant_id = ? AND slug = ? AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

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
        .bind(&role.created_by)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for perm in permissions {
            let perm_id = Uuid::now_v7().to_string();
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
            .bind(&role.created_by)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

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

    async fn set_permissions(
        &self,
        tenant_id: &str,
        role_id: &str,
        permissions: &[String],
        updated_at: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

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
        .bind(role_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for perm in permissions {
            let perm_id = Uuid::now_v7().to_string();
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
            .bind(updated_at)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}

#[async_trait]
impl UserStore for SqliteStore {
    async fn find_by_id(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserRecord>> {
        let row = sqlx::query(
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
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let row = sqlx::query(
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
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let row = sqlx::query(
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
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let rows = sqlx::query(
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
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        let row = sqlx::query(
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
        .bind(role_slug)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.get("cnt"))
    }

    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) AS cnt
            FROM app_user
            WHERE tenant_id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.get("cnt"))
    }

    async fn create(&self, user: &UserRecord) -> Result<UserWithRole> {
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
        .bind(&user.created_by)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn record_login_success(
        &self,
        tenant_id: &str,
        user_id: &str,
        login_at: &str,
    ) -> Result<()> {
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
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn record_login_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        failed_attempts: i32,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE app_user
            SET failed_attempts = ?, locked_until = ?, updated_at = ?
            WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(failed_attempts)
        .bind(locked_until)
        .bind(updated_at)
        .bind(tenant_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn record_recovery_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        recovery_failed_attempts: i32,
        recovery_locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE app_user
            SET recovery_failed_attempts = ?, recovery_locked_until = ?, updated_at = ?
            WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(recovery_failed_attempts)
        .bind(recovery_locked_until)
        .bind(updated_at)
        .bind(tenant_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }
}

#[async_trait]
impl AuditStore for SqliteStore {
    async fn append(&self, entry: &AuditLogEntry) -> Result<()> {
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
        .bind(&entry.created_by)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn list_by_tenant(&self, tenant_id: &str, limit: usize) -> Result<Vec<AuditLogEntry>> {
        let rows = sqlx::query(
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
        .bind(i64::try_from(limit).unwrap_or(100))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

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
    use eventrent_core::services::ensure_default_tenant;
    use eventrent_core::testing::{
        contract_audit_log, contract_default_seeds, contract_initial_setup_and_recovery_key,
        contract_last_owner_protection, contract_password_reset_clears_lockout,
        contract_recovery_key_rotation_and_lockout, contract_system_roles,
        contract_unique_partial_indexes, contract_user_creation_and_rbac, contract_user_lockout,
        run_store_contract_suite,
    };

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
}
