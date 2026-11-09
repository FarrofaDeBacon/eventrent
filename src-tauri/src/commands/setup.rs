use crate::AppState;
use eventrent_core::{
    AppConfig, InitialSetupInput, InitialSetupResult, OperationMode, SessionInfo, TenantBranding,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub config: AppConfig,
    pub default_tenant_id: String,
    pub has_owner: bool,
    pub session: Option<SessionInfo>,
    pub branding: Option<TenantBranding>,
}

#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let config = state.get_config()?;
    let user_count = state
        .services
        .stores
        .users
        .count_by_tenant(&state.default_tenant_id)
        .await
        .map_err(crate::map_core_err)?;
    let branding = state
        .services
        .stores
        .branding
        .get_by_tenant_id(&state.default_tenant_id)
        .await
        .map_err(crate::map_core_err)?;
    let session = state.get_session_without_touch();

    Ok(AppStatus {
        config,
        default_tenant_id: state.default_tenant_id.clone(),
        has_owner: user_count > 0,
        session,
        branding,
    })
}

#[tauri::command]
pub async fn complete_initial_setup(
    state: State<'_, AppState>,
    input: InitialSetupInput,
) -> Result<InitialSetupResult, String> {
    let current_cfg = state.get_config()?;
    let result = state
        .services
        .complete_initial_setup(
            &state.default_tenant_id,
            &input,
            current_cfg.inactivity_timeout_minutes,
        )
        .await
        .map_err(crate::map_core_err)?;

    let updated_cfg = AppConfig {
        mode: Some(OperationMode::Local),
        setup_completed: true,
        inactivity_timeout_minutes: current_cfg.inactivity_timeout_minutes,
    };
    state.save_config(&updated_cfg)?;
    state.set_session(Some(result.session.clone()));

    Ok(result)
}
