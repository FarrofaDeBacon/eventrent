use crate::domain::branding::TenantBranding;
use crate::domain::user::SessionInfo;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialSetupInput {
    pub mode: OperationMode,
    pub company_name: String,
    pub owner_name: String,
    pub owner_email: String,
    pub owner_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialSetupResult {
    pub tenant_id: String,
    pub recovery_key: String,
    pub session: SessionInfo,
    pub branding: TenantBranding,
}
