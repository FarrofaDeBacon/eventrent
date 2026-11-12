use crate::domain::{
    AuditLogEntry, Customer, CustomerAddress, CustomerContact, CustomerFilter, CustomerPage,
    CustomerWithDetails, DocumentType, Role, RoleWithPermissions, Tenant, TenantBranding,
    UpdateBrandingInput, UserRecord, UserWithRole,
};
use crate::error::{CoreError, Result};
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
    /// Incrementa atomicamente o contador de falhas de login em um único UPDATE e devolve o novo total.
    async fn increment_login_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        updated_at: &str,
    ) -> Result<i32>;
    /// Define o bloqueio temporário de login do usuário.
    async fn set_login_lockout(
        &self,
        tenant_id: &str,
        user_id: &str,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()>;
    /// Incrementa atomicamente o contador de falhas da chave de recuperação em um único UPDATE e devolve o novo total.
    async fn increment_recovery_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        updated_at: &str,
    ) -> Result<i32>;
    /// Define o bloqueio temporário da chave de recuperação do usuário.
    async fn set_recovery_lockout(
        &self,
        tenant_id: &str,
        user_id: &str,
        locked_until: Option<&str>,
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

#[async_trait]
pub trait CustomerStore: Send + Sync {
    async fn find_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<CustomerWithDetails>>;
    async fn find_by_document(
        &self,
        tenant_id: &str,
        document_type: DocumentType,
        document_number: &str,
    ) -> Result<Option<Customer>>;
    async fn list(&self, tenant_id: &str, filter: &CustomerFilter) -> Result<CustomerPage>;
    async fn create(
        &self,
        customer: &Customer,
        contacts: &[CustomerContact],
        addresses: &[CustomerAddress],
    ) -> Result<()>;
    async fn update(
        &self,
        customer: &Customer,
        contacts: &[CustomerContact],
        addresses: &[CustomerAddress],
    ) -> Result<()>;
    async fn soft_delete(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()>;
    async fn count_active(&self, tenant_id: &str) -> Result<u64>;
}

#[derive(Clone)]
pub struct StoreBundle {
    pub tenants: Arc<dyn TenantStore>,
    pub branding: Arc<dyn BrandingStore>,
    pub roles: Arc<dyn RoleStore>,
    pub users: Arc<dyn UserStore>,
    pub audit: Arc<dyn AuditStore>,
    pub customers: Arc<dyn CustomerStore>,
    pub tx_manager: Option<Arc<dyn TransactionManager>>,
}

#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>>;
}

#[async_trait]
pub trait TxStoreBundle: Send + Sync {
    fn tenants(&self) -> &dyn TenantStore;
    fn branding(&self) -> &dyn BrandingStore;
    fn roles(&self) -> &dyn RoleStore;
    fn users(&self) -> &dyn UserStore;
    fn audit(&self) -> &dyn AuditStore;
    fn customers(&self) -> &dyn CustomerStore;

    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TransactionPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
}

impl Default for TransactionPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 25,
        }
    }
}

use std::future::Future;
use std::pin::Pin;

pub type TxBoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Executa um closure assíncrono dentro de uma transação atômica gerenciada por `TransactionManager`.
///
/// Garantias:
/// 1. Commit automático quando o closure retornar `Ok(T)`.
/// 2. Rollback automático quando o closure retornar `Err(E)` ou ocorrer pânico (RAII/Drop).
/// 3. Nova tentativa automática com backoff exponencial quando o erro for `CoreError::TransientConflict`
///    (ex.: SQLITE_BUSY / lock contention sob concorrência).
pub async fn run_in_tx<F, T>(
    manager: &dyn TransactionManager,
    policy: &TransactionPolicy,
    f: F,
) -> Result<T>
where
    F: for<'a> Fn(&'a dyn TxStoreBundle) -> TxBoxFuture<'a, T>,
{
    let mut attempt = 0;
    loop {
        let tx = match manager.begin_transaction().await {
            Ok(tx) => tx,
            Err(err) => {
                let is_transient = matches!(err, CoreError::TransientConflict(_));
                if is_transient && attempt < policy.max_retries {
                    attempt += 1;
                    let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                    continue;
                }
                return Err(err);
            }
        };

        match f(tx.as_ref()).await {
            Ok(res) => match tx.commit().await {
                Ok(()) => return Ok(res),
                Err(commit_err) => {
                    let is_transient = matches!(commit_err, CoreError::TransientConflict(_));
                    if is_transient && attempt < policy.max_retries {
                        attempt += 1;
                        let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                        continue;
                    }
                    return Err(commit_err);
                }
            },
            Err(err) => {
                let is_transient = matches!(err, CoreError::TransientConflict(_));
                let _ = tx.rollback().await;

                if is_transient && attempt < policy.max_retries {
                    attempt += 1;
                    let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                    continue;
                }

                return Err(err);
            }
        }
    }
}
