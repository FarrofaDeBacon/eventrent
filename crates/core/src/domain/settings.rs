use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

pub const SETTING_DEFAULT_SETUP_MINUTES: &str = "allocation.default_setup_minutes";
pub const SETTING_DEFAULT_TEARDOWN_MINUTES: &str = "allocation.default_teardown_minutes";
pub const SETTING_DEFAULT_TURNOVER_BUFFER_MINUTES: &str =
    "allocation.default_turnover_buffer_minutes";

pub const DEFAULT_SETUP_MINUTES: i64 = 30;
pub const DEFAULT_TEARDOWN_MINUTES: i64 = 20;
pub const DEFAULT_TURNOVER_BUFFER_MINUTES: i64 = 60;
pub const MAX_ALLOCATION_MINUTES: i64 = 240;

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
    pub default_setup_minutes: i64,
    pub default_teardown_minutes: i64,
    pub default_turnover_buffer_minutes: i64,
}

impl Default for AllocationSettings {
    fn default() -> Self {
        Self {
            default_setup_minutes: DEFAULT_SETUP_MINUTES,
            default_teardown_minutes: DEFAULT_TEARDOWN_MINUTES,
            default_turnover_buffer_minutes: DEFAULT_TURNOVER_BUFFER_MINUTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAllocationSettingsInput {
    pub default_setup_minutes: i64,
    pub default_teardown_minutes: i64,
    pub default_turnover_buffer_minutes: i64,
}

pub fn validate_allocation_settings(input: &UpdateAllocationSettingsInput) -> Result<()> {
    let fields = [
        ("Tempo padrão de montagem", input.default_setup_minutes),
        (
            "Tempo padrão de desmontagem",
            input.default_teardown_minutes,
        ),
        (
            "Buffer padrão de giro",
            input.default_turnover_buffer_minutes,
        ),
    ];
    for (label, value) in fields {
        if !(0..=MAX_ALLOCATION_MINUTES).contains(&value) {
            return Err(CoreError::Validation(format!(
                "{label} deve estar entre 0 e {MAX_ALLOCATION_MINUTES} minutos."
            )));
        }
    }
    Ok(())
}
