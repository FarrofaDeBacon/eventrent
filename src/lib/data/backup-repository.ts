import type {
  BackupMetadataView,
  BackupSettings,
  BackupStatus,
  RestorePendingMarker,
  SetBackupSettingsInput,
  VerifyBackupResult,
} from "./schemas/backup";

export interface BackupRepository {
  getStatus(): Promise<BackupStatus>;
  updateSettings(input: SetBackupSettingsInput): Promise<BackupSettings>;
  runBackupNow(): Promise<BackupMetadataView>;
  listBackups(folder?: string): Promise<BackupMetadataView[]>;
  verifyBackup(filePath: string): Promise<VerifyBackupResult>;
  scheduleRestore(filePath: string): Promise<RestorePendingMarker>;
  chooseFolder(): Promise<string | null>;
  chooseFile(): Promise<string | null>;
}
