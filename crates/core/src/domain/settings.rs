use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

pub const SETTING_CHANGEOVER_MINUTES: &str = "allocation.changeover_minutes";
pub const SETTING_INTERNAL_TRANSFER_MINUTES: &str = "allocation.internal_transfer_minutes";
pub const SETTING_EXTERNAL_TRANSFER_MINUTES: &str = "allocation.external_transfer_minutes";

pub const DEFAULT_CHANGEOVER_MINUTES: i64 = 30;
pub const DEFAULT_INTERNAL_TRANSFER_MINUTES: i64 = 20;
pub const DEFAULT_EXTERNAL_TRANSFER_MINUTES: i64 = 60;

pub const MAX_CHANGEOVER_MINUTES: i64 = 240;
pub const MAX_INTERNAL_TRANSFER_SETTING_MINUTES: i64 = 240;
pub const MAX_EXTERNAL_TRANSFER_MINUTES: i64 = 720;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantSetting {
    pub id: String,
    pub tenant_id: String,
    pub setting_key: String,
    pub setting_value: String,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AllocationSettings {
    pub changeover_minutes: i64,
    pub internal_transfer_minutes: i64,
    pub external_transfer_minutes: i64,
}

impl Default for AllocationSettings {
    fn default() -> Self {
        Self {
            changeover_minutes: DEFAULT_CHANGEOVER_MINUTES,
            internal_transfer_minutes: DEFAULT_INTERNAL_TRANSFER_MINUTES,
            external_transfer_minutes: DEFAULT_EXTERNAL_TRANSFER_MINUTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAllocationSettingsInput {
    pub changeover_minutes: i64,
    pub internal_transfer_minutes: i64,
    pub external_transfer_minutes: i64,
}

pub fn validate_allocation_settings(input: &UpdateAllocationSettingsInput) -> Result<()> {
    let fields = [
        (
            "Virada no mesmo espaço",
            input.changeover_minutes,
            MAX_CHANGEOVER_MINUTES,
        ),
        (
            "Deslocamento interno",
            input.internal_transfer_minutes,
            MAX_INTERNAL_TRANSFER_SETTING_MINUTES,
        ),
        (
            "Deslocamento externo",
            input.external_transfer_minutes,
            MAX_EXTERNAL_TRANSFER_MINUTES,
        ),
    ];
    for (label, value, max) in fields {
        if !(0..=max).contains(&value) {
            return Err(CoreError::Validation(format!(
                "{label} deve estar entre 0 e {max} minutos."
            )));
        }
    }
    Ok(())
}
