use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginPolicy {
    /// Se verdadeiro, revela se a conta está temporariamente bloqueada devolvendo `CoreError::AccountLocked`.
    /// Se falso (padrão no core / recomendado para nuvem), não revela o bloqueio para evitar enumeração de contas
    /// e ataques de enumeração/timing, executando a carga criptográfica do hash fictício e retornando `CoreError::InvalidCredentials`.
    pub reveal_account_locked: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub recovery_key_hash: Option<String>,
    pub role_id: String,
    pub active: bool,
    pub last_login_at: Option<String>,
    pub failed_attempts: i32,
    pub locked_until: Option<String>,
    /// Tentativas falhas de uso da chave de recuperação (bloqueio separado do login).
    pub recovery_failed_attempts: i32,
    /// Bloqueio temporário da chave de recuperação (backoff exponencial).
    pub recovery_locked_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

impl fmt::Debug for UserRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserRecord")
            .field("id", &self.id)
            .field("tenant_id", &self.tenant_id)
            .field("name", &self.name)
            .field("email", &self.email)
            .field("password_hash", &"[REDACTED]")
            .field(
                "recovery_key_hash",
                &self.recovery_key_hash.as_ref().map(|_| "[REDACTED]"),
            )
            .field("role_id", &self.role_id)
            .field("active", &self.active)
            .field("last_login_at", &self.last_login_at)
            .field("failed_attempts", &self.failed_attempts)
            .field("locked_until", &self.locked_until)
            .field("recovery_failed_attempts", &self.recovery_failed_attempts)
            .field("recovery_locked_until", &self.recovery_locked_until)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("deleted_at", &self.deleted_at)
            .field("created_by", &self.created_by)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserWithRole {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    pub role_id: String,
    pub role_name: String,
    pub role_slug: String,
    pub permissions: Vec<String>,
    pub active: bool,
    pub last_login_at: Option<String>,
    pub failed_attempts: i32,
    pub locked_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role_id: String,
}

impl fmt::Debug for CreateUserInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateUserInput")
            .field("name", &self.name)
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .field("role_id", &self.role_id)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub name: String,
    pub email: String,
    pub role_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResetPasswordInput {
    pub new_password: String,
}

impl fmt::Debug for ResetPasswordInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResetPasswordInput")
            .field("new_password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RecoverPasswordInput {
    pub email: String,
    pub recovery_key: String,
    pub new_password: String,
}

impl fmt::Debug for RecoverPasswordInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoverPasswordInput")
            .field("email", &self.email)
            .field("recovery_key", &"[REDACTED]")
            .field("new_password", &"[REDACTED]")
            .finish()
    }
}

/// Resultado de uma recuperação de senha bem-sucedida.
/// A nova chave de recuperação deve ser exibida ao usuário UMA ÚNICA VEZ e nunca armazenada em texto plano.
#[derive(Clone, Serialize, Deserialize)]
pub struct RecoverPasswordResult {
    /// Nova chave de recuperação em texto plano — exibir uma vez, depois descartar.
    pub new_recovery_key: String,
}

impl fmt::Debug for RecoverPasswordResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoverPasswordResult")
            .field("new_recovery_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

impl fmt::Debug for LoginInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginInput")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfo {
    pub user_id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    pub role_id: String,
    pub role_name: String,
    pub role_slug: String,
    pub permissions: Vec<String>,
    pub authenticated_at: String,
    pub last_activity_at: String,
    pub inactivity_timeout_minutes: u32,
}

impl SessionInfo {
    /// Verifica se a sessão possui a permissão requerida.
    /// Regra de implicação estrutural: a permissão `<recurso>:manage` implica automaticamente `<recurso>:read`.
    pub fn has_permission(&self, perm: &str) -> bool {
        if self.permissions.iter().any(|p| p == perm) {
            return true;
        }

        if let Some(resource) = perm.strip_suffix(":read") {
            let implied_manage = format!("{resource}:manage");
            if self.permissions.iter().any(|p| p == &implied_manage) {
                return true;
            }
        }

        false
    }
}
