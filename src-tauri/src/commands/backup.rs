use crate::guards::require_session;
use crate::AppState;
use eventrent_core::domain::backup::{
    validate_backup_file_for_restore, validate_destination_folder,
};
use eventrent_core::{
    BackupMetadataView, BackupSettings, BackupStatus, RestorePendingMarker, VerifyBackupResult,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBackupSettingsInput {
    pub destination_folder: String,
    pub schedule_enabled: bool,
    pub retention_daily: u32,
    pub retention_weekly: u32,
    pub retention_monthly: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBackupsInput {
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyBackupInput {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRestoreInput {
    pub file_path: String,
    pub password: String,
}

#[tauri::command]
pub async fn get_backup_status(state: State<'_, AppState>) -> Result<BackupStatus, String> {
    let session = require_session(&state).await?;
    state
        .services
        .get_backup_status(&session)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn set_backup_settings(
    state: State<'_, AppState>,
    input: SetBackupSettingsInput,
) -> Result<BackupSettings, String> {
    let session = require_session(&state).await?;

    let current_status = state
        .services
        .get_backup_status(&session)
        .await
        .map_err(crate::map_core_err)?;

    let mut new_settings = current_status.settings;
    new_settings.destination_folder = input.destination_folder.trim().to_string();
    new_settings.schedule_enabled = input.schedule_enabled;
    new_settings.retention_daily = input.retention_daily;
    new_settings.retention_weekly = input.retention_weekly;
    new_settings.retention_monthly = input.retention_monthly;

    let status = state
        .services
        .set_backup_settings(&session, &new_settings)
        .await
        .map_err(crate::map_core_err)?;

    Ok(status.settings)
}

#[tauri::command]
pub async fn run_backup_now(state: State<'_, AppState>) -> Result<BackupMetadataView, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let status = state
        .services
        .get_backup_status(&session)
        .await
        .map_err(crate::map_core_err)?;

    let dest = status.settings.destination_folder.trim().to_string();
    if dest.is_empty() {
        return Err("Nenhuma pasta de destino de backup configurada. Selecione uma pasta antes de prosseguir.".to_string());
    }

    let dest_path = PathBuf::from(&dest);

    let (manifest, backup_file) = match state
        .backup_service
        .run_backup(&dest_path, &state.db_pool, &state.app_version)
        .await
    {
        Ok(res) => res,
        Err(e) => {
            let err_str = e.to_string();
            let _ = state
                .services
                .record_backup_outcome(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "failed",
                    Some(&err_str),
                    None,
                )
                .await;
            return Err(crate::map_core_err(e));
        }
    };

    let _ = state
        .services
        .record_backup_outcome(
            &session.tenant_id,
            Some(&session.user_id),
            "success",
            None,
            Some(&backup_file.to_string_lossy()),
        )
        .await;

    // Executa rotação e retenção na pasta de destino
    if let Err(retention_err) = state.backup_service.execute_retention(
        &dest_path,
        status.settings.retention_daily,
        status.settings.retention_weekly,
        status.settings.retention_monthly,
    ) {
        tracing::warn!(
            error = %retention_err,
            "Aviso: backup criado com sucesso, mas a política de retenção falhou ao rotacionar arquivos antigos"
        );
    }

    let meta = std::fs::metadata(&backup_file).ok();
    let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let file_name = backup_file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    Ok(BackupMetadataView {
        file_name,
        file_path: backup_file.to_string_lossy().to_string(),
        byte_size: file_size,
        created_at: manifest.created_at,
        app_version: manifest.app_version,
        last_applied_migration: manifest.last_applied_migration,
        is_compatible: true,
        file_count: manifest.files.len(),
        verified: true,
        skipped_media_files: manifest.skipped_media_files,
    })
}

#[tauri::command]
pub async fn list_backups(
    state: State<'_, AppState>,
    input: ListBackupsInput,
) -> Result<Vec<BackupMetadataView>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let folder_str = if let Some(f) = input.folder.filter(|f| !f.trim().is_empty()) {
        f
    } else {
        let status = state
            .services
            .get_backup_status(&session)
            .await
            .map_err(crate::map_core_err)?;
        status.settings.destination_folder
    };

    if folder_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let folder_path = PathBuf::from(folder_str.trim());
    state
        .backup_service
        .list_backups_in_folder(&folder_path, eventrent_store_sqlite::CURRENT_MAX_MIGRATION)
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn verify_backup(
    state: State<'_, AppState>,
    input: VerifyBackupInput,
) -> Result<VerifyBackupResult, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let path = PathBuf::from(input.file_path.trim());
    state
        .backup_service
        .verify_backup_file(&path, eventrent_store_sqlite::CURRENT_MAX_MIGRATION)
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn schedule_restore(
    state: State<'_, AppState>,
    input: ScheduleRestoreInput,
) -> Result<RestorePendingMarker, String> {
    let session = require_session(&state).await?;
    let policy = eventrent_core::LoginPolicy {
        reveal_account_locked: true,
    };
    state
        .services
        .authorize_restore_operation(&session, &input.password, &policy)
        .await
        .map_err(crate::map_core_err)?;

    let path = PathBuf::from(input.file_path.trim());
    state
        .backup_service
        .schedule_restore(
            &path,
            &session.user_id,
            eventrent_store_sqlite::CURRENT_MAX_MIGRATION,
        )
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn choose_backup_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let app_clone = app.clone();

    let picked =
        tokio::task::spawn_blocking(move || app_clone.dialog().file().blocking_pick_folder())
            .await
            .map_err(|e| format!("Falha ao aguardar diálogo de seleção de pasta: {e}"))?;

    let Some(folder) = picked else {
        return Ok(None);
    };

    let folder_path = folder
        .into_path()
        .map_err(|e| format!("Caminho inválido: {e}"))?;

    validate_destination_folder(&folder_path, &state.backup_service.app_data_dir)
        .map_err(crate::map_core_err)?;

    Ok(Some(folder_path.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn choose_backup_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let app_clone = app.clone();

    let picked = tokio::task::spawn_blocking(move || {
        app_clone
            .dialog()
            .file()
            .add_filter(
                "Arquivo de Backup EventRent (*.eventrent-backup)",
                &["eventrent-backup"],
            )
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("Falha ao aguardar diálogo de seleção de arquivo: {e}"))?;

    let Some(file) = picked else {
        return Ok(None);
    };

    let file_path = file
        .into_path()
        .map_err(|e| format!("Caminho inválido: {e}"))?;

    validate_backup_file_for_restore(&file_path).map_err(crate::map_core_err)?;

    Ok(Some(file_path.to_string_lossy().to_string()))
}
