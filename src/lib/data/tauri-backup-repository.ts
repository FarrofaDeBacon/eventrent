import { invoke } from "@tauri-apps/api/core";
import type { BackupRepository } from "./backup-repository";
import type {
  BackupMetadataView,
  BackupSettings,
  BackupStatus,
  RestorePendingMarker,
  SetBackupSettingsInput,
  VerifyBackupResult,
} from "./schemas/backup";

export class TauriBackupRepository implements BackupRepository {
  async getStatus(): Promise<BackupStatus> {
    return invoke<BackupStatus>("get_backup_status");
  }

  async updateSettings(input: SetBackupSettingsInput): Promise<BackupSettings> {
    return invoke<BackupSettings>("set_backup_settings", { input });
  }

  async runBackupNow(): Promise<BackupMetadataView> {
    return invoke<BackupMetadataView>("run_backup_now");
  }

  async listBackups(folder?: string): Promise<BackupMetadataView[]> {
    return invoke<BackupMetadataView[]>("list_backups", {
      input: { folder: folder ?? null },
    });
  }

  async verifyBackup(filePath: string): Promise<VerifyBackupResult> {
    return invoke<VerifyBackupResult>("verify_backup", {
      input: { file_path: filePath },
    });
  }

  async scheduleRestore(filePath: string): Promise<RestorePendingMarker> {
    return invoke<RestorePendingMarker>("schedule_restore", {
      input: { file_path: filePath },
    });
  }

  async chooseFolder(): Promise<string | null> {
    return invoke<string | null>("choose_backup_folder");
  }

  async chooseFile(): Promise<string | null> {
    return invoke<string | null>("choose_backup_file");
  }
}
