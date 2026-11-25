use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use eventrent_core::error::CoreError;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

/// Intervalo regular entre checagens de backup automático enquanto o app está aberto (15 minutos).
pub const BACKUP_CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Janela de espera mínima após uma falha antes de tentar novamente (1 hora).
pub const BACKUP_RETRY_INTERVAL_AFTER_FAILURE: chrono::Duration = chrono::Duration::hours(1);

/// Executa a rotina de verificação e execução de backup agendado se devido.
/// Retorna `Ok(Some(PathBuf))` se um backup foi gerado, `Ok(None)` se não era devido, ou `Err` se falhou.
pub async fn run_scheduled_backup_if_due(
    state: &AppState,
    app_handle: Option<&AppHandle>,
) -> Result<Option<PathBuf>, String> {
    // 1. Obtém as configurações e status do tenant padrão
    let status = match state
        .services
        .get_backup_status_for_tenant(&state.default_tenant_id)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Agendador de backup: não foi possível ler status do tenant: {e}");
            return Ok(None);
        }
    };

    // 2. Verifica se o agendamento está ativo
    if !status.settings.schedule_enabled {
        tracing::debug!("Agendador de backup: agendamento automático está desativado.");
        return Ok(None);
    }

    // 3. Verifica pasta de destino
    let dest_str = status.settings.destination_folder.trim();
    if dest_str.is_empty() {
        tracing::debug!("Agendador de backup: nenhuma pasta de destino configurada.");
        return Ok(None);
    }
    let dest_path = PathBuf::from(dest_str);

    // 4. Se a última execução falhou, respeita a janela de 1 hora antes de tentar novamente
    if status.settings.last_backup_status.as_deref() == Some("failed") {
        if let Some(ref last_at_str) = status.settings.last_backup_at {
            if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(last_at_str) {
                let diff = Utc::now().signed_duration_since(last_dt.with_timezone(&Utc));
                if diff < BACKUP_RETRY_INTERVAL_AFTER_FAILURE {
                    tracing::debug!(
                        diff_mins = diff.num_minutes(),
                        "Agendador de backup: última tentativa falhou; aguardando janela de retry de 1 hora."
                    );
                    return Ok(None);
                }
            }
        }
    }

    // 5. Verifica se o backup está realmente devido
    if !status.is_backup_due {
        tracing::debug!("Agendador de backup: backup ainda não está devido.");
        return Ok(None);
    }

    tracing::info!(
        tenant = %state.default_tenant_id,
        dest = %dest_path.display(),
        "Agendador de backup: backup automático está devido. Iniciando execução..."
    );

    // 6. Executa o backup (o serviço de backup adquire a trava RAII BackupOperationGuard internamente)
    let outcome = state
        .backup_service
        .run_backup(&dest_path, &state.db_pool, &state.app_version)
        .await;

    match outcome {
        Ok((_manifest, backup_file)) => {
            tracing::info!(
                backup_file = %backup_file.display(),
                "Agendador de backup: backup automático concluído com sucesso."
            );

            // Registra sucesso na auditoria e configurações
            if let Err(e) = state
                .services
                .record_backup_outcome(
                    &state.default_tenant_id,
                    None, // disparado pelo sistema/agendador
                    "success",
                    None,
                    Some(&backup_file.to_string_lossy()),
                )
                .await
            {
                tracing::error!(
                    error = %e,
                    "Falha ao registrar desfecho de sucesso de backup automático na auditoria/configurações"
                );
            }

            // Executa rotação e retenção na pasta de destino
            let _ = state.backup_service.execute_retention(
                &dest_path,
                status.settings.retention_daily,
                status.settings.retention_weekly,
                status.settings.retention_monthly,
            );

            // Emite evento Tauri para a interface atualizar o status em tempo real
            if let Some(app) = app_handle {
                let _ = app.emit("backup:status-changed", ());
            }

            Ok(Some(backup_file))
        }
        Err(CoreError::TransientConflict(msg)) => {
            tracing::warn!(
                conflict = %msg,
                "Agendador de backup: operação concorrente de backup ou restauração em andamento. Tentará no próximo ciclo."
            );
            Ok(None)
        }
        Err(err) => {
            let err_str = err.to_string();
            tracing::error!(
                error = %err_str,
                "Agendador de backup: falha na execução do backup automático."
            );

            // Registra falha na auditoria e configurações
            if let Err(e) = state
                .services
                .record_backup_outcome(
                    &state.default_tenant_id,
                    None,
                    "failed",
                    Some(&err_str),
                    None,
                )
                .await
            {
                tracing::error!(
                    error = %e,
                    "Falha ao registrar desfecho de falha de backup automático na auditoria/configurações"
                );
            }

            // Emite evento Tauri para a interface exibir aviso de falha
            if let Some(app) = app_handle {
                let _ = app.emit("backup:status-changed", ());
            }

            Err(err_str)
        }
    }
}

/// Inicia a tarefa em segundo plano do agendador de backup no runtime do Tauri.
/// Executa imediatamente na inicialização (se devido) e periodicamente a cada 15 minutos.
pub fn start_backup_scheduler(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Pausa breve no início (2 segundos) para permitir que a janela principal renderize primeiro
        tokio::time::sleep(Duration::from_secs(2)).await;

        loop {
            let state = app_handle.state::<AppState>();
            let _ = run_scheduled_backup_if_due(&state, Some(&app_handle)).await;

            tokio::time::sleep(BACKUP_CHECK_INTERVAL).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_lifecycle_tests::setup_real_app_state;
    use eventrent_core::domain::backup::BackupSettings;

    static SCHEDULER_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn test_scheduled_backup_skipped_when_disabled() {
        let _test_lock = SCHEDULER_TEST_LOCK.lock().await;
        let (app_state, _guard, _tenant_id, owner_session, _db_path) = setup_real_app_state().await;

        // Configura com schedule desabilitado
        let settings = BackupSettings {
            schedule_enabled: false,
            ..Default::default()
        };
        app_state
            .services
            .set_backup_settings(&owner_session, &settings)
            .await
            .expect("Configurações salvas");

        let res = run_scheduled_backup_if_due(&app_state, None)
            .await
            .expect("Execução da verificação");

        assert!(
            res.is_none(),
            "Backup automático não deve rodar quando agendamento estiver desativado"
        );
    }

    #[tokio::test]
    async fn test_scheduled_backup_executed_when_due() {
        let _test_lock = SCHEDULER_TEST_LOCK.lock().await;
        let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

        let backup_dest =
            std::env::temp_dir().join(format!("eventrent_auto_backups_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&backup_dest).unwrap();
        let _dest_guard = crate::session_lifecycle_tests::TempDirGuard(backup_dest.clone());

        // Configura com schedule ativo, pasta válida e último backup há 2 dias
        let two_days_ago = (Utc::now() - chrono::Duration::days(2)).to_rfc3339();
        let settings = BackupSettings {
            schedule_enabled: true,
            destination_folder: backup_dest.to_string_lossy().to_string(),
            last_backup_at: Some(two_days_ago),
            ..Default::default()
        };

        app_state
            .services
            .set_backup_settings(&owner_session, &settings)
            .await
            .expect("Configurações salvas");

        let res = run_scheduled_backup_if_due(&app_state, None)
            .await
            .expect("Execução do backup automático");

        assert!(
            res.is_some(),
            "Backup automático deve ser executado quando estiver devido"
        );
        let backup_file = res.unwrap();
        assert!(backup_file.exists());

        // Confere que status foi atualizado com sucesso
        let status = app_state
            .services
            .get_backup_status_for_tenant(&tenant_id)
            .await
            .unwrap();
        assert_eq!(
            status.settings.last_backup_status.as_deref(),
            Some("success")
        );
        assert!(!status.is_backup_due);
    }

    #[tokio::test]
    async fn test_scheduled_backup_respects_one_hour_retry_on_previous_failure() {
        let _test_lock = SCHEDULER_TEST_LOCK.lock().await;
        let (app_state, _guard, _tenant_id, owner_session, _db_path) = setup_real_app_state().await;

        let backup_dest = std::env::temp_dir().join(format!(
            "eventrent_auto_backups_retry_{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&backup_dest).unwrap();
        let _dest_guard = crate::session_lifecycle_tests::TempDirGuard(backup_dest.clone());

        // 1. Falhou há 30 minutos (< 1h) -> deve pular e não tentar ainda
        let thirty_mins_ago = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        let mut settings = BackupSettings {
            schedule_enabled: true,
            destination_folder: backup_dest.to_string_lossy().to_string(),
            last_backup_at: Some(thirty_mins_ago),
            last_backup_status: Some("failed".to_string()),
            last_backup_error: Some("Simulação de erro prévio".to_string()),
            ..Default::default()
        };

        app_state
            .services
            .set_backup_settings(&owner_session, &settings)
            .await
            .expect("Configurações salvas");

        let res = run_scheduled_backup_if_due(&app_state, None)
            .await
            .expect("Execução da verificação");

        assert!(
            res.is_none(),
            "Backup não deve tentar novamente antes de transcorrer a janela de retry de 1 hora"
        );

        // 2. Falhou há 90 minutos (> 1h) -> agora deve tentar e executar com sucesso
        let ninety_mins_ago = (Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
        settings.last_backup_at = Some(ninety_mins_ago);

        app_state
            .services
            .set_backup_settings(&owner_session, &settings)
            .await
            .expect("Configurações salvas");

        let res2 = run_scheduled_backup_if_due(&app_state, None)
            .await
            .expect("Execução da tentativa após 1h");

        assert!(
            res2.is_some(),
            "Backup deve ser tentado novamente após a janela de 1 hora de retry"
        );
        let backup_file = res2.unwrap();
        assert!(backup_file.exists());
    }
}
