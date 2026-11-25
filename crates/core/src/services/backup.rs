use crate::domain::{
    days_since_last_backup, is_backup_due_v2, is_backup_stale, BackupSettings, BackupStatus,
    TenantSetting, PERM_BACKUP_MANAGE, SETTING_BACKUP_SETTINGS,
};
use crate::error::{CoreError, Result};
use crate::services::{build_audit_entry, CoreServices};
use crate::SessionInfo;
use chrono::Utc;

impl CoreServices {
    /// Autoriza uma operação de backup no nível do núcleo de negócio.
    /// Valida que a sessão está ativa e possui a permissão canônica `backup:manage`.
    pub fn authorize_backup_operation(&self, session: &SessionInfo) -> Result<()> {
        self.check_session_and_permission(session, PERM_BACKUP_MANAGE)
    }

    /// Autoriza uma operação de agendamento de restauração:
    /// 1. Exige sessão ativa e permissão `backup:manage`.
    /// 2. Valida a senha atual do usuário ativo na sessão com proteção de lockout progressivo.
    pub async fn authorize_restore_operation(
        &self,
        session: &SessionInfo,
        current_password: &str,
        policy: &crate::domain::LoginPolicy,
    ) -> Result<()> {
        self.authorize_backup_operation(session)?;
        self.verify_user_password_with_lockout(session, current_password, policy, "restore")
            .await?;
        Ok(())
    }

    /// Consulta o status consolidado de backup do tenant atual.
    /// Exige sessão válida e permissão `backup:manage`.
    pub async fn get_backup_status(&self, session: &SessionInfo) -> Result<BackupStatus> {
        self.authorize_backup_operation(session)?;
        self.get_backup_status_for_tenant(&session.tenant_id).await
    }

    /// Consulta o status de backup para um tenant (usado internamente e por comandos autenticados).
    pub async fn get_backup_status_for_tenant(&self, tenant_id: &str) -> Result<BackupStatus> {
        let settings = self.get_backup_settings_for_tenant(tenant_id).await?;
        let now = Utc::now();
        let last_success = settings.last_success_at.as_deref().or(
            if settings.last_backup_status.as_deref() == Some("success") {
                settings.last_backup_at.as_deref()
            } else {
                None
            },
        );
        let last_attempt = settings
            .last_attempt_at
            .as_deref()
            .or(settings.last_backup_at.as_deref());
        let last_status = settings.last_backup_status.as_deref();
        let due = is_backup_due_v2(
            last_success,
            last_attempt,
            last_status,
            now,
            settings.schedule_enabled,
        );
        let stale = is_backup_stale(last_success, now);
        let days = days_since_last_backup(last_success, now);

        Ok(BackupStatus {
            settings,
            is_backup_due: due,
            days_since_last_backup: days,
            is_stale: stale,
        })
    }

    /// Lê a configuração de backup persistida em `tenant_setting` sob a chave `backup_settings`.
    pub async fn get_backup_settings_for_tenant(&self, tenant_id: &str) -> Result<BackupSettings> {
        let rows = self
            .stores
            .settings
            .get_many(tenant_id, &[SETTING_BACKUP_SETTINGS])
            .await?;

        if let Some(row) = rows.into_iter().next() {
            if let Ok(settings) = serde_json::from_str::<BackupSettings>(&row.setting_value) {
                return Ok(settings);
            }
        }

        Ok(BackupSettings::default())
    }

    /// Atualiza as configurações de backup do tenant na tabela `tenant_setting`.
    /// Exige a permissão `backup:manage` e grava trilha na auditoria operacional.
    pub async fn set_backup_settings(
        &self,
        session: &SessionInfo,
        settings: &BackupSettings,
    ) -> Result<BackupStatus> {
        self.authorize_backup_operation(session)?;

        if settings.retention_daily < 1 || settings.retention_daily > 60 {
            return Err(CoreError::Validation(format!(
                "Retenção diária inválida ({}). O valor deve estar entre 1 e 60 dias.",
                settings.retention_daily
            )));
        }
        if settings.retention_weekly > 52 {
            return Err(CoreError::Validation(format!(
                "Retenção semanal inválida ({}). O valor deve estar entre 0 e 52 semanas.",
                settings.retention_weekly
            )));
        }
        if settings.retention_monthly > 60 {
            return Err(CoreError::Validation(format!(
                "Retenção mensal inválida ({}). O valor deve estar entre 0 e 60 meses.",
                settings.retention_monthly
            )));
        }

        if !settings.destination_folder.trim().is_empty() {
            let dest_path = std::path::Path::new(&settings.destination_folder);
            if !dest_path.exists() {
                return Err(CoreError::Validation(format!(
                    "A pasta de destino informada não existe: {}",
                    settings.destination_folder
                )));
            }
            if !dest_path.is_dir() {
                return Err(CoreError::Validation(format!(
                    "O caminho informado não é um diretório: {}",
                    settings.destination_folder
                )));
            }
        }

        let serialized = serde_json::to_string(settings).map_err(|e| {
            CoreError::Validation(format!("Falha ao serializar configurações de backup: {e}"))
        })?;

        let session_clone = session.clone();
        let serialized_clone = serialized.clone();
        let dest_clone = settings.destination_folder.clone();
        let settings_clone = settings.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let serialized = serialized_clone.clone();
            let dest = dest_clone.clone();
            let settings = settings_clone.clone();

            Box::pin(async move {
                let now = Utc::now().to_rfc3339();
                stores
                    .settings()
                    .upsert(&TenantSetting {
                        id: uuid::Uuid::now_v7().to_string(),
                        tenant_id: session.tenant_id.clone(),
                        setting_key: SETTING_BACKUP_SETTINGS.to_string(),
                        setting_value: serialized,
                        updated_at: now,
                        updated_by: Some(session.user_id.clone()),
                    })
                    .await?;

                let detail = format!(
                    "Configurações de backup atualizadas: pasta='{}', agendamento={}, retenção diária={}, semanal={}, mensal={}",
                    dest,
                    settings.schedule_enabled,
                    settings.retention_daily,
                    settings.retention_weekly,
                    settings.retention_monthly
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "backup.settings_updated",
                        "tenant_setting",
                        SETTING_BACKUP_SETTINGS,
                        &detail,
                    ))
                    .await?;

                Ok(())
            })
        })
        .await?;

        self.get_backup_status_for_tenant(&session.tenant_id).await
    }

    /// Registra o resultado da execução de um backup (sucesso ou falha) nas configurações e na auditoria.
    pub async fn record_backup_outcome(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        status: &str,
        error: Option<&str>,
        backup_path: Option<&str>,
    ) -> Result<()> {
        let mut settings = self.get_backup_settings_for_tenant(tenant_id).await?;
        let now = Utc::now().to_rfc3339();
        settings.last_attempt_at = Some(now.clone());
        if status == "success" {
            settings.last_success_at = Some(now.clone());
            settings.last_backup_at = Some(now.clone());
        }
        settings.last_backup_status = Some(status.to_string());
        settings.last_backup_error = error.map(|s| s.to_string());

        let serialized = serde_json::to_string(&settings).map_err(|e| {
            CoreError::Validation(format!("Falha ao serializar configurações de backup: {e}"))
        })?;

        let tenant_id_str = tenant_id.to_string();
        let user_id_opt = user_id.map(|s| s.to_string());
        let status_str = status.to_string();
        let detail = match (status, backup_path, error) {
            ("success", Some(p), _) => format!("Backup concluído com sucesso em '{p}'."),
            ("success", None, _) => "Backup concluído com sucesso.".to_string(),
            ("failed", _, Some(err)) => format!("Falha na execução de backup: {err}"),
            _ => format!("Execução de backup finalizada com status '{status}'."),
        };

        self.run_transaction(|stores| {
            let tenant_id = tenant_id_str.clone();
            let serialized = serialized.clone();
            let user_id = user_id_opt.clone();
            let status = status_str.clone();
            let detail = detail.clone();

            Box::pin(async move {
                let now_str = Utc::now().to_rfc3339();
                stores
                    .settings()
                    .upsert(&TenantSetting {
                        id: uuid::Uuid::now_v7().to_string(),
                        tenant_id: tenant_id.clone(),
                        setting_key: SETTING_BACKUP_SETTINGS.to_string(),
                        setting_value: serialized,
                        updated_at: now_str,
                        updated_by: user_id.clone(),
                    })
                    .await?;

                let action = if status == "success" {
                    "backup.completed"
                } else {
                    "backup.failed"
                };

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        user_id.as_deref(),
                        action,
                        "backup",
                        &tenant_id,
                        &detail,
                    ))
                    .await?;

                Ok(())
            })
        })
        .await
    }
}
