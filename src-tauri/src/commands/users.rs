use crate::guards::require_session_and_permission;
use crate::AppState;
use eventrent_core::{
    CreateUserInput, ResetPasswordInput, RoleWithPermissions, UpdateUserInput, UserWithRole,
    PERM_ROLE_READ, PERM_USER_MANAGE, PERM_USER_READ,
};
use tauri::State;

#[tauri::command]
pub async fn list_users(state: State<'_, AppState>) -> Result<Vec<UserWithRole>, String> {
    let session = require_session_and_permission(&state, Some(PERM_USER_READ))?;
    state
        .services
        .list_users(&session)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_user(
    state: State<'_, AppState>,
    input: CreateUserInput,
) -> Result<UserWithRole, String> {
    let session = require_session_and_permission(&state, Some(PERM_USER_MANAGE))?;
    state
        .services
        .create_user(&session, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_user(
    state: State<'_, AppState>,
    user_id: String,
    input: UpdateUserInput,
) -> Result<UserWithRole, String> {
    let session = require_session_and_permission(&state, Some(PERM_USER_MANAGE))?;
    state
        .services
        .update_user(&session, &user_id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_user_active(
    state: State<'_, AppState>,
    user_id: String,
    active: bool,
) -> Result<UserWithRole, String> {
    let session = require_session_and_permission(&state, Some(PERM_USER_MANAGE))?;
    state
        .services
        .set_user_active(&session, &user_id, active)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_user_password(
    state: State<'_, AppState>,
    user_id: String,
    input: ResetPasswordInput,
) -> Result<(), String> {
    let session = require_session_and_permission(&state, Some(PERM_USER_MANAGE))?;
    state
        .services
        .reset_user_password(&session, &user_id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_roles(state: State<'_, AppState>) -> Result<Vec<RoleWithPermissions>, String> {
    let session = require_session_and_permission(&state, Some(PERM_ROLE_READ))?;
    state
        .services
        .list_roles(&session)
        .await
        .map_err(|e| e.to_string())
}
