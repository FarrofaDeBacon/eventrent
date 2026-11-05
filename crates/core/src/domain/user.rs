use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub name: String,
    pub email: String,
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordInput {
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverPasswordInput {
    pub email: String,
    pub recovery_key: String,
    pub new_password: String,
}

/// Resultado de uma recuperação de senha bem-sucedida.
/// A nova chave de recuperação deve ser exibida ao usuário UMA ÚNICA VEZ e nunca armazenada em texto plano.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverPasswordResult {
    /// Nova chave de recuperação em texto plano — exibir uma vez, depois descartar.
    pub new_recovery_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
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
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == perm)
    }
}
