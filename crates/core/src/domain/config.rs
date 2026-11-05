use crate::domain::branding::TenantBranding;
use crate::domain::user::SessionInfo;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationMode {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub mode: Option<OperationMode>,
    pub setup_completed: bool,
    pub inactivity_timeout_minutes: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: None,
            setup_completed: false,
            inactivity_timeout_minutes: 30,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InitialSetupInput {
    pub mode: OperationMode,
    pub company_name: String,
    pub owner_name: String,
    pub owner_email: String,
    pub owner_password: String,
}

impl fmt::Debug for InitialSetupInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitialSetupInput")
            .field("mode", &self.mode)
            .field("company_name", &self.company_name)
            .field("owner_name", &self.owner_name)
            .field("owner_email", &self.owner_email)
            .field("owner_password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InitialSetupResult {
    pub tenant_id: String,
    pub recovery_key: String,
    pub session: SessionInfo,
    pub branding: TenantBranding,
}

impl fmt::Debug for InitialSetupResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitialSetupResult")
            .field("tenant_id", &self.tenant_id)
            .field("recovery_key", &"[REDACTED]")
            .field("session", &self.session)
            .field("branding", &self.branding)
            .finish()
    }
}
