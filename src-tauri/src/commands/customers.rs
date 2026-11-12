use crate::guards::require_session;
use crate::AppState;
use eventrent_core::{
    CreateCustomerInput, CustomerFilter, CustomerPage, CustomerWithDetails, UpdateCustomerInput,
};
use tauri::State;

#[tauri::command]
pub async fn list_customers(
    state: State<'_, AppState>,
    filter: CustomerFilter,
) -> Result<CustomerPage, String> {
    let session = require_session(&state)?;
    state
        .services
        .list_customers(&session, &filter)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_customer(
    state: State<'_, AppState>,
    id: String,
) -> Result<CustomerWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .get_customer(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_customer(
    state: State<'_, AppState>,
    input: CreateCustomerInput,
) -> Result<CustomerWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .create_customer(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_customer(
    state: State<'_, AppState>,
    id: String,
    input: UpdateCustomerInput,
) -> Result<CustomerWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .update_customer(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn set_customer_active(
    state: State<'_, AppState>,
    id: String,
    active: bool,
) -> Result<CustomerWithDetails, String> {
    let session = require_session(&state)?;
    state
        .services
        .set_customer_active(&session, &id, active)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_customer(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state)?;
    state
        .services
        .delete_customer(&session, &id)
        .await
        .map_err(crate::map_core_err)
}
