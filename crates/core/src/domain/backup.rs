use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

pub const DEFAULT_RETENTION_DAILY: u32 = 7;
pub const DEFAULT_RETENTION_WEEKLY: u32 = 4;
pub const DEFAULT_RETENTION_MONTHLY: u32 = 6;
pub const BACKUP_FILE_EXTENSION: &str = "eventrent-backup";
pub const BACKUP_TEMP_EXTENSION: &str = "tmp.eventrent-backup";
pub const RESTORE_PENDING_FILE: &str = "restore_pending.json";
pub const SETTING_BACKUP_SETTINGS: &str = "backup_settings";

static BACKUP_OR_RESTORE_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Retorna true se houver uma operação de backup ou restauração em andamento no processo.
pub fn is_backup_or_restore_active() -> bool {
    BACKUP_OR_RESTORE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst)
}

/// Guarda RAII para garantir exclusão mútua em operações de backup e restauração.
pub struct BackupOperationGuard;

impl BackupOperationGuard {
    pub fn try_acquire() -> Result<Self> {
        if BACKUP_OR_RESTORE_ACTIVE
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            Err(CoreError::TransientConflict(
                "Operação de backup ou restauração já está em andamento.".to_string(),
            ))
        } else {
            Ok(Self)
        }
    }
}

impl Drop for BackupOperationGuard {
    fn drop(&mut self) {
        BACKUP_OR_RESTORE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSettings {
    pub destination_folder: String,
    pub schedule_enabled: bool,
    pub retention_daily: u32,
    pub retention_weekly: u32,
    pub retention_monthly: u32,
    pub last_backup_at: Option<String>,
    pub last_backup_status: Option<String>,
    pub last_backup_error: Option<String>,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            destination_folder: String::new(),
            schedule_enabled: true,
            retention_daily: DEFAULT_RETENTION_DAILY,
            retention_weekly: DEFAULT_RETENTION_WEEKLY,
            retention_monthly: DEFAULT_RETENTION_MONTHLY,
            last_backup_at: None,
            last_backup_status: None,
            last_backup_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupStatus {
    pub settings: BackupSettings,
    pub is_backup_due: bool,
    pub days_since_last_backup: Option<u32>,
    pub is_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupFileEntry {
    pub path: String,
    pub byte_size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub manifest_version: u32,
    pub app_version: String,
    pub created_at: String,
    pub last_applied_migration: i64,
    pub migrations_checksum: String,
    pub database_integrity_checked: bool,
    pub files: Vec<BackupFileEntry>,
    pub table_counts: HashMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupMetadataView {
    pub file_name: String,
    pub file_path: String,
    pub byte_size: u64,
    pub created_at: String,
    pub app_version: String,
    pub last_applied_migration: i64,
    pub is_compatible: bool,
    pub file_count: usize,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyBackupResult {
    pub valid: bool,
    pub manifest: Option<BackupManifest>,
    pub error_message: Option<String>,
    pub is_compatible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestorePendingMarker {
    pub backup_file_path: String,
    pub backup_sha256: String,
    pub scheduled_at: String,
    pub scheduled_by_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionCandidate {
    pub file_name: String,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetentionPlan {
    pub files_to_keep: Vec<String>,
    pub files_to_delete: Vec<String>,
}

/// Determina se um backup agendado está devido (mais de 24h desde o último sucesso ou nunca executado).
pub fn is_backup_due(
    last_backup_at: Option<&str>,
    now: DateTime<Utc>,
    schedule_enabled: bool,
) -> bool {
    if !schedule_enabled {
        return false;
    }
    match last_backup_at {
        None => true,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => (now - dt.with_timezone(&Utc)).num_hours() >= 24,
            Err(_) => true,
        },
    }
}

/// Determina se o status de backup está "estéril" (> 7 dias sem backup ou nenhum backup já realizado).
/// Aciona alerta fixo e destacado na interface de usuário.
pub fn is_backup_stale(last_backup_at: Option<&str>, now: DateTime<Utc>) -> bool {
    match last_backup_at {
        None => true,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => (now - dt.with_timezone(&Utc)).num_days() >= 7,
            Err(_) => true,
        },
    }
}

/// Calcula quantos dias se passaram desde o último backup.
pub fn days_since_last_backup(last_backup_at: Option<&str>, now: DateTime<Utc>) -> Option<u32> {
    let s = last_backup_at?;
    let dt = DateTime::parse_from_rfc3339(s).ok()?;
    let days = (now - dt.with_timezone(&Utc)).num_days();
    if days < 0 {
        Some(0)
    } else {
        Some(days as u32)
    }
}

/// Calcula o plano de retenção conforme a regra canônica:
/// - Retém os N backups diários mais recentes.
/// - Retém os N backups semanais (o mais recente de cada semana ISO).
/// - Retém os N backups mensais (o mais recente de cada mês).
///
/// Todos os demais arquivos de backup são marcados para descarte.
pub fn calculate_retention_plan(
    candidates: &[RetentionCandidate],
    daily_limit: u32,
    weekly_limit: u32,
    monthly_limit: u32,
) -> RetentionPlan {
    if candidates.is_empty() {
        return RetentionPlan::default();
    }

    let mut sorted = candidates.to_vec();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.created_at)); // decrescente (mais recentes primeiro)

    let mut keep_set = HashSet::new();

    // 1. Diários: os primeiros `daily_limit` backups mais recentes
    for c in sorted.iter().take(daily_limit as usize) {
        keep_set.insert(c.file_path.clone());
    }

    // 2. Semanais: o mais recente de cada semana (ano-semana)
    let mut weeks_seen = HashSet::new();
    let mut weekly_count = 0;
    for c in &sorted {
        if weekly_count >= weekly_limit {
            break;
        }
        let year_week = (
            c.created_at.iso_week().year(),
            c.created_at.iso_week().week(),
        );
        if weeks_seen.insert(year_week) {
            keep_set.insert(c.file_path.clone());
            weekly_count += 1;
        }
    }

    // 3. Mensais: o mais recente de cada mês (ano-mês)
    let mut months_seen = HashSet::new();
    let mut monthly_count = 0;
    for c in &sorted {
        if monthly_count >= monthly_limit {
            break;
        }
        let year_month = (c.created_at.year(), c.created_at.month());
        if months_seen.insert(year_month) {
            keep_set.insert(c.file_path.clone());
            monthly_count += 1;
        }
    }

    let mut files_to_keep = Vec::new();
    let mut files_to_delete = Vec::new();

    for c in sorted {
        if keep_set.contains(&c.file_path) {
            files_to_keep.push(c.file_path);
        } else {
            files_to_delete.push(c.file_path);
        }
    }

    RetentionPlan {
        files_to_keep,
        files_to_delete,
    }
}

/// Valida o diretório de destino do backup contra Path Traversal e colisão com a pasta de dados do app.
pub fn validate_destination_folder(dest: &Path, app_data_dir: &Path) -> Result<()> {
    if dest.as_os_str().is_empty() {
        return Err(CoreError::Validation(
            "Pasta de destino de backup não pode ser vazia.".to_string(),
        ));
    }

    let canonical_dest = match dest.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Err(CoreError::Validation(format!(
                "A pasta de destino de backup não existe ou é inacessível: {}",
                dest.display()
            )));
        }
    };

    if !canonical_dest.is_dir() {
        return Err(CoreError::Validation(format!(
            "O caminho informado não é um diretório: {}",
            dest.display()
        )));
    }

    if let Ok(canonical_app) = app_data_dir.canonicalize() {
        if canonical_dest == canonical_app || canonical_dest.starts_with(&canonical_app) {
            return Err(CoreError::Validation(
                "A pasta de destino do backup não pode estar dentro do diretório de dados do aplicativo para evitar recursão e corrupção.".to_string(),
            ));
        }
    }

    Ok(())
}

/// Valida se um arquivo de backup possui nome e extensão canônicos.
pub fn is_valid_backup_filename(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".eventrent-backup") && !lower.starts_with(".tmp.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_is_backup_due_and_stale() {
        let now = DateTime::parse_from_rfc3339("2026-11-23T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // Nunca executado
        assert!(is_backup_due(None, now, true));
        assert!(!is_backup_due(None, now, false));
        assert!(is_backup_stale(None, now));
        assert_eq!(days_since_last_backup(None, now), None);

        // 12 horas atrás -> não devido, não estéril
        let recent = (now - Duration::hours(12)).to_rfc3339();
        assert!(!is_backup_due(Some(&recent), now, true));
        assert!(!is_backup_stale(Some(&recent), now));
        assert_eq!(days_since_last_backup(Some(&recent), now), Some(0));

        // 25 horas atrás -> devido, mas não estéril
        let due_time = (now - Duration::hours(25)).to_rfc3339();
        assert!(is_backup_due(Some(&due_time), now, true));
        assert!(!is_backup_stale(Some(&due_time), now));
        assert_eq!(days_since_last_backup(Some(&due_time), now), Some(1));

        // 8 dias atrás -> devido e estéril
        let stale_time = (now - Duration::days(8)).to_rfc3339();
        assert!(is_backup_due(Some(&stale_time), now, true));
        assert!(is_backup_stale(Some(&stale_time), now));
        assert_eq!(days_since_last_backup(Some(&stale_time), now), Some(8));
    }

    #[test]
    fn test_retention_plan_calculation() {
        let base = DateTime::parse_from_rfc3339("2026-11-23T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let mut candidates = Vec::new();
        for day in 0..30 {
            let dt = base - Duration::days(day);
            candidates.push(RetentionCandidate {
                file_name: format!("backup-{day}.eventrent-backup"),
                file_path: format!("/backups/backup-{day}.eventrent-backup"),
                created_at: dt,
            });
        }

        let plan = calculate_retention_plan(&candidates, 7, 4, 3);
        assert!(!plan.files_to_keep.is_empty());
        assert!(!plan.files_to_delete.is_empty());
        assert_eq!(
            plan.files_to_keep.len() + plan.files_to_delete.len(),
            candidates.len()
        );

        // Os primeiros 7 mais recentes devem estar preservados
        for day in 0..7 {
            assert!(
                plan.files_to_keep
                    .contains(&format!("/backups/backup-{day}.eventrent-backup")),
                "Backup do dia {day} deve ser retido pelo limite diário"
            );
        }
    }
}
