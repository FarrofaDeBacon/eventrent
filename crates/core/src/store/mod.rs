use crate::domain::{
    AuditLogEntry, Role, RoleWithPermissions, Tenant, TenantBranding, UpdateBrandingInput,
    UserRecord, UserWithRole,
};
use crate::error::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait TenantStore: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn create(&self, tenant: &Tenant) -> Result<Tenant>;
    async fn update_name(&self, id: &str, name: &str, updated_at: &str) -> Result<Tenant>;
}

#[async_trait]
pub trait BrandingStore: Send + Sync {
    async fn get_by_tenant_id(&self, tenant_id: &str) -> Result<Option<TenantBranding>>;
    async fn save(&self, tenant_id: &str, input: &UpdateBrandingInput) -> Result<TenantBranding>;
}

#[async_trait]
pub trait RoleStore: Send + Sync {
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<RoleWithPermissions>>;
    async fn find_by_id(
        &self,
        tenant_id: &str,
        role_id: &str,
    ) -> Result<Option<RoleWithPermissions>>;
    async fn find_by_slug(
        &self,
        tenant_id: &str,
        slug: &str,
    ) -> Result<Option<RoleWithPermissions>>;
    async fn create(&self, role: &Role, permissions: &[String]) -> Result<RoleWithPermissions>;
    async fn set_permissions(
        &self,
        tenant_id: &str,
        role_id: &str,
        permissions: &[String],
        updated_at: &str,
    ) -> Result<()>;
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn find_by_id(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserRecord>>;
    async fn find_by_email(&self, tenant_id: &str, email: &str) -> Result<Option<UserRecord>>;
    async fn find_with_role(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserWithRole>>;
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<UserWithRole>>;
    async fn count_active_by_role_slug(&self, tenant_id: &str, role_slug: &str) -> Result<i64>;
    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64>;
    async fn create(&self, user: &UserRecord) -> Result<UserWithRole>;
    async fn update(
        &self,
        tenant_id: &str,
        user_id: &str,
        name: &str,
        email: &str,
        role_id: &str,
        updated_at: &str,
    ) -> Result<UserWithRole>;
    async fn set_active(
        &self,
        tenant_id: &str,
        user_id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<UserWithRole>;
    async fn update_password(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        updated_at: &str,
    ) -> Result<()>;
    async fn record_login_success(
        &self,
        tenant_id: &str,
        user_id: &str,
        login_at: &str,
    ) -> Result<()>;
    async fn record_login_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        failed_attempts: i32,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()>;
    /// Registra falha de uso da chave de recuperação com backoff exponencial separado do login.
    async fn record_recovery_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        recovery_failed_attempts: i32,
        recovery_locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()>;
    /// Atualiza senha, zera contadores de bloqueio (login e recuperação) e grava novo hash da chave de recuperação.
    async fn update_password_and_recovery_key(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        recovery_key_hash: &str,
        updated_at: &str,
    ) -> Result<()>;
}

#[async_trait]
pub trait AuditStore: Send + Sync {
    async fn append(&self, entry: &AuditLogEntry) -> Result<()>;
    async fn list_by_tenant(&self, tenant_id: &str, limit: usize) -> Result<Vec<AuditLogEntry>>;
}

#[derive(Clone)]
pub struct StoreBundle {
    pub tenants: Arc<dyn TenantStore>,
    pub branding: Arc<dyn BrandingStore>,
    pub roles: Arc<dyn RoleStore>,
    pub users: Arc<dyn UserStore>,
    pub audit: Arc<dyn AuditStore>,
}
