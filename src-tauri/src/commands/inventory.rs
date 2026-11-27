use crate::guards::require_session;
use crate::AppState;
use eventrent_core::{
    CreateEquipmentUnitInput, CreateWarehouseInput, DecommissionEquipmentUnitInput, EquipmentUnit,
    EquipmentUnitDetails, EquipmentUnitFilter, EquipmentUnitListPage, RecordStockMovementInput,
    StockBalance, StockMovement, StockMovementFilter, UpdateEquipmentUnitInput,
    UpdateWarehouseInput, Warehouse,
};
use tauri::State;

// ==========================================
// 1. DEPÓSITOS (Warehouses)
// ==========================================

#[tauri::command]
pub async fn list_warehouses(
    state: State<'_, AppState>,
    include_inactive: Option<bool>,
) -> Result<Vec<Warehouse>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .list_warehouses(&session, include_inactive.unwrap_or(false))
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_warehouse(state: State<'_, AppState>, id: String) -> Result<Warehouse, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_warehouse(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_warehouse(
    state: State<'_, AppState>,
    input: CreateWarehouseInput,
) -> Result<Warehouse, String> {
    let session = require_session(&state).await?;
    state
        .services
        .create_warehouse(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_warehouse(
    state: State<'_, AppState>,
    id: String,
    input: UpdateWarehouseInput,
) -> Result<Warehouse, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_warehouse(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn set_default_warehouse(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .set_default_warehouse(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_warehouse(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .delete_warehouse(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

// ==========================================
// 2. UNIDADES FÍSICAS (Equipment Units)
// ==========================================

#[tauri::command]
pub async fn list_equipment_units(
    state: State<'_, AppState>,
    filter: Option<EquipmentUnitFilter>,
) -> Result<EquipmentUnitListPage, String> {
    let session = require_session(&state).await?;
    let f = filter.unwrap_or_default();
    state
        .services
        .list_equipment_units(&session, &f)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_equipment_unit(
    state: State<'_, AppState>,
    id: String,
) -> Result<EquipmentUnitDetails, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_equipment_unit(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_equipment_unit(
    state: State<'_, AppState>,
    input: CreateEquipmentUnitInput,
) -> Result<EquipmentUnit, String> {
    let session = require_session(&state).await?;
    state
        .services
        .create_equipment_unit(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_equipment_unit(
    state: State<'_, AppState>,
    id: String,
    input: UpdateEquipmentUnitInput,
) -> Result<EquipmentUnit, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_equipment_unit(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn decommission_equipment_unit(
    state: State<'_, AppState>,
    id: String,
    input: DecommissionEquipmentUnitInput,
) -> Result<EquipmentUnit, String> {
    let session = require_session(&state).await?;
    state
        .services
        .decommission_equipment_unit(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_equipment_unit(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .delete_equipment_unit(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

// ==========================================
// 3. ESTOQUE E SALDOS (Stock & Balances)
// ==========================================

#[tauri::command]
pub async fn get_stock_balance(
    state: State<'_, AppState>,
    warehouse_id: String,
    equipment_model_id: String,
) -> Result<StockBalance, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_stock_balance(&session, &warehouse_id, &equipment_model_id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn list_stock_balances(
    state: State<'_, AppState>,
    equipment_model_id: Option<String>,
    warehouse_id: Option<String>,
) -> Result<Vec<StockBalance>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .list_stock_balances(
            &session,
            equipment_model_id.as_deref(),
            warehouse_id.as_deref(),
        )
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn record_stock_movement(
    state: State<'_, AppState>,
    input: RecordStockMovementInput,
) -> Result<StockMovement, String> {
    let session = require_session(&state).await?;
    state
        .services
        .record_stock_movement(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn list_stock_movements(
    state: State<'_, AppState>,
    filter: Option<StockMovementFilter>,
) -> Result<Vec<StockMovement>, String> {
    let session = require_session(&state).await?;
    let f = filter.unwrap_or_default();
    state
        .services
        .list_stock_movements(&session, &f)
        .await
        .map_err(crate::map_core_err)
}
