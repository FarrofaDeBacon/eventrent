use crate::guards::require_session;
use crate::AppState;
use eventrent_core::{
    AttributeDefinition, Brand, CatalogListFilter, CatalogListResult,
    CreateAttributeDefinitionInput, CreateBrandInput, CreateCategoryInput,
    CreateEquipmentModelInput, EquipmentCategory, EquipmentCategoryTreeItem,
    EquipmentModelWithDetails, InheritedAttributeDefinition, SuggestedCategoriesResult,
    UpdateAttributeDefinitionInput, UpdateBrandInput, UpdateCategoryInput,
    UpdateEquipmentModelInput,
};
use tauri::State;

#[tauri::command]
pub async fn list_brands(
    state: State<'_, AppState>,
    include_inactive: Option<bool>,
) -> Result<Vec<Brand>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .list_brands(&session, include_inactive.unwrap_or(false))
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_brand(
    state: State<'_, AppState>,
    input: CreateBrandInput,
) -> Result<Brand, String> {
    let session = require_session(&state).await?;
    state
        .services
        .create_brand(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_brand(
    state: State<'_, AppState>,
    id: String,
    input: UpdateBrandInput,
) -> Result<Brand, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_brand(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_brand(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .delete_brand(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn list_categories_tree(
    state: State<'_, AppState>,
    include_inactive: Option<bool>,
) -> Result<Vec<EquipmentCategoryTreeItem>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .list_categories_tree(&session, include_inactive.unwrap_or(false))
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_category_effective_attributes(
    state: State<'_, AppState>,
    category_id: String,
) -> Result<Vec<InheritedAttributeDefinition>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_category_effective_attributes(&session, &category_id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_category(
    state: State<'_, AppState>,
    input: CreateCategoryInput,
) -> Result<EquipmentCategory, String> {
    let session = require_session(&state).await?;
    state
        .services
        .create_category(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_category(
    state: State<'_, AppState>,
    id: String,
    input: UpdateCategoryInput,
) -> Result<EquipmentCategory, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_category(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_category(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .delete_category(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_attribute_definition(
    state: State<'_, AppState>,
    input: CreateAttributeDefinitionInput,
) -> Result<AttributeDefinition, String> {
    let session = require_session(&state).await?;
    state
        .services
        .create_attribute_definition(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_attribute_definition(
    state: State<'_, AppState>,
    id: String,
    input: UpdateAttributeDefinitionInput,
) -> Result<AttributeDefinition, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_attribute_definition(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_attribute_definition(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .delete_attribute_definition(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn list_equipment_models(
    state: State<'_, AppState>,
    filter: CatalogListFilter,
) -> Result<CatalogListResult, String> {
    let session = require_session(&state).await?;
    state
        .services
        .list_equipment_models(&session, &filter)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_equipment_model(
    state: State<'_, AppState>,
    id: String,
) -> Result<EquipmentModelWithDetails, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_equipment_model(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn create_equipment_model(
    state: State<'_, AppState>,
    input: CreateEquipmentModelInput,
) -> Result<EquipmentModelWithDetails, String> {
    let session = require_session(&state).await?;
    state
        .services
        .create_equipment_model(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_equipment_model(
    state: State<'_, AppState>,
    id: String,
    input: UpdateEquipmentModelInput,
) -> Result<EquipmentModelWithDetails, String> {
    let session = require_session(&state).await?;
    state
        .services
        .update_equipment_model(&session, &id, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn set_equipment_model_active(
    state: State<'_, AppState>,
    id: String,
    active: bool,
) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .set_equipment_model_active(&session, &id, active)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn delete_equipment_model(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let session = require_session(&state).await?;
    state
        .services
        .delete_equipment_model(&session, &id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn load_suggested_catalog_categories(
    state: State<'_, AppState>,
) -> Result<SuggestedCategoriesResult, String> {
    let session = require_session(&state).await?;
    state
        .services
        .load_suggested_catalog_categories(&session)
        .await
        .map_err(crate::map_core_err)
}
