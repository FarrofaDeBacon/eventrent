use crate::guards::require_session;
use crate::AppState;
use eventrent_core::{
    LoginInput, RecoverPasswordInput, RecoverPasswordResult, RegenerateRecoveryKeyInput,
    RegenerateRecoveryKeyResult, SessionInfo,
};
use tauri::State;

#[tauri::command]
pub async fn login(state: State<'_, AppState>, input: LoginInput) -> Result<SessionInfo, String> {
    let cfg = state.get_config()?;
    // No app desktop local, a política permite revelar conta bloqueada para feedback direto ao operador
    let policy = eventrent_core::LoginPolicy {
        reveal_account_locked: true,
    };
    let session = state
        .services
        .login(
            &state.default_tenant_id,
            &input,
            &policy,
            cfg.inactivity_timeout_minutes,
        )
        .await
        .map_err(crate::map_core_err)?;

    state.set_session(Some(session.clone()));
    Ok(session)
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    state.set_session(None);
    Ok(())
}

#[tauri::command]
pub async fn get_current_session(
    state: State<'_, AppState>,
) -> Result<Option<SessionInfo>, String> {
    Ok(state.get_session_without_touch())
}

#[tauri::command]
pub async fn recover_password(
    state: State<'_, AppState>,
    input: RecoverPasswordInput,
) -> Result<RecoverPasswordResult, String> {
    let policy = eventrent_core::LoginPolicy {
        reveal_account_locked: true,
    };
    state
        .services
        .recover_password(&state.default_tenant_id, &input, &policy)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn regenerate_recovery_key(
    state: State<'_, AppState>,
    input: RegenerateRecoveryKeyInput,
) -> Result<RegenerateRecoveryKeyResult, String> {
    let session = require_session(&state).await?;
    let policy = eventrent_core::LoginPolicy {
        reveal_account_locked: true,
    };
    state
        .services
        .regenerate_recovery_key(&session, &input.current_password, &policy)
        .await
        .map_err(crate::map_core_err)
}
