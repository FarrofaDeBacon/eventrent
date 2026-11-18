use crate::guards::require_session;
use crate::AppState;
use eventrent_core::{
    CreateVenueInput, UpdateVenueInput, VenueFilter, VenuePage, VenueWithDetails,
};
use tauri::State;

#[tauri::command]
pub async fn list_venues(
    state: State<'_, AppState>,
    filter: VenueFilter,
) -> Result<VenuePage, String> {
    let session = require_session(&state)?;
    state
        .services
        .list_venues(&session, &filter)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_venue(state: State<'_, AppState>, id: String) -> Result<VenueWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .get_venue(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_venue(
    state: State<'_, AppState>,
    input: CreateVenueInput,
) -> Result<VenueWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .create_venue(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_venue(
    state: State<'_, AppState>,
    input: UpdateVenueInput,
) -> Result<VenueWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .update_venue(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn set_venue_active(
    state: State<'_, AppState>,
    id: String,
    active: bool,
) -> Result<VenueWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .set_venue_active(&session, &id, active)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_venue(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state)?;
    state
        .services
        .delete_venue(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn check_venue_duplicate_name_city(
    state: State<'_, AppState>,
    name: String,
    city: Option<String>,
    exclude_id: Option<String>,
) -> Result<bool, String> {
    let session = require_session(&state)?;
    state
        .services
        .check_venue_duplicate_name_city(&session, &name, city.as_deref(), exclude_id.as_deref())
        .await
        .map_err(crate::map_core_err)
}
