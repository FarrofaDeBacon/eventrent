use crate::guards::require_session;
use crate::AppState;
use eventrent_core::{AllocationSettings, UpdateAllocationSettingsInput};
use tauri::State;

#[tauri::command]
pub async fn get_allocation_settings(
    state: State<'_, AppState>,
) -> Result<AllocationSettings, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_allocation_settings(&session)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_allocation_settings(
    state: State<'_, AppState>,
    input: UpdateAllocationSettingsInput,
) -> Result<AllocationSettings, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_allocation_settings(&session, &input)
        .await
        .map_err(crate::map_core_err)
}
