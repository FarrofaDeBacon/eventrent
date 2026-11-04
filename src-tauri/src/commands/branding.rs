use crate::AppState;
use eventrent_core::domain::branding::{TenantBranding, UpdateBrandingInput};
use eventrent_core::repository::BrandingRepository;
use tauri::State;

#[tauri::command]
pub async fn get_branding(
    state: State<'_, AppState>,
    tenant_id: String,
) -> Result<Option<TenantBranding>, String> {
    BrandingRepository::get_by_tenant_id(&state.pool, &tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_branding(
    state: State<'_, AppState>,
    tenant_id: String,
    input: UpdateBrandingInput,
) -> Result<TenantBranding, String> {
    BrandingRepository::save(&state.pool, &tenant_id, &input)
        .await
        .map_err(|e| e.to_string())
}
