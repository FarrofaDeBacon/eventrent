use crate::AppState;
use eventrent_core::{TenantBranding, UpdateBrandingInput};
use tauri::State;

fn resolve_tenant_id<'a>(state: &'a AppState, tenant_id: &'a str) -> &'a str {
    if tenant_id.is_empty()
        || tenant_id == "default"
        || tenant_id == "018f0000-0000-7000-8000-000000000001"
    {
        &state.default_tenant_id
    } else {
        tenant_id
    }
}

#[tauri::command]
pub async fn get_branding(
    state: State<'_, AppState>,
    tenant_id: String,
) -> Result<Option<TenantBranding>, String> {
    let resolved_id = resolve_tenant_id(&state, &tenant_id);
    state
        .services
        .stores
        .branding
        .get_by_tenant_id(resolved_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_branding(
    state: State<'_, AppState>,
    tenant_id: String,
    input: UpdateBrandingInput,
) -> Result<TenantBranding, String> {
    let _ = resolve_tenant_id(&state, &tenant_id);
    let session = state.require_session()?;
    state
        .services
        .save_branding(&session, &input)
        .await
        .map_err(|e| e.to_string())
}
