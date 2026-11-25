import type { BackupRepository } from "./backup-repository";
import type {
  BackupMetadataView,
  BackupSettings,
  BackupStatus,
  RestorePendingMarker,
  SetBackupSettingsInput,
  VerifyBackupResult,
} from "./schemas/backup";

const STORAGE_KEY = "eventrent:mock:backup_settings:v1";

const DEFAULT_SETTINGS: BackupSettings = {
  destination_folder: "C:\\Backups\\EventRent",
  schedule_enabled: true,
  retention_daily: 7,
  retention_weekly: 4,
  retention_monthly: 6,
  last_backup_at: null,
  last_backup_status: null,
  last_backup_error: null,
};

export class MockBackupRepository implements BackupRepository {
  private mockBackups: BackupMetadataView[] = [];

  constructor() {
    if (!import.meta.env.DEV && import.meta.env.VITE_E2E !== "1") {
      throw new Error(
        "Mocks não podem ser carregados no ambiente de produção. Execute o aplicativo desktop EventRent.",
      );
    }
  }

  private loadSettings(): BackupSettings {
    if (typeof window === "undefined") {
      return { ...DEFAULT_SETTINGS };
    }
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return { ...DEFAULT_SETTINGS };
    }
    try {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
    } catch {
      return { ...DEFAULT_SETTINGS };
    }
  }

  private saveSettings(settings: BackupSettings): void {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    }
  }

  async getStatus(): Promise<BackupStatus> {
    const settings = this.loadSettings();
    return {
      settings,
      is_backup_due: !settings.last_backup_at,
      days_since_last_backup: settings.last_backup_at ? 0 : null,
      is_stale: !settings.last_backup_at,
    };
  }

  async updateSettings(input: SetBackupSettingsInput): Promise<BackupSettings> {
    const current = this.loadSettings();
    const updated: BackupSettings = {
      ...current,
      destination_folder: input.destination_folder,
      schedule_enabled: input.schedule_enabled,
      retention_daily: input.retention_daily,
      retention_weekly: input.retention_weekly,
      retention_monthly: input.retention_monthly,
    };
    this.saveSettings(updated);
    return updated;
  }

  async runBackupNow(): Promise<BackupMetadataView> {
    const settings = this.loadSettings();
    const nowIso = new Date().toISOString();
    const ts = nowIso.replace(/[:.]/g, "-");
    const fileName = `eventrent-backup-${ts}.eventrent-backup`;
    const filePath = `${settings.destination_folder}\\${fileName}`;

    const newBackup: BackupMetadataView = {
      file_name: fileName,
      file_path: filePath,
      byte_size: 2048576,
      created_at: nowIso,
      app_version: "0.10.0-fase2n",
      last_applied_migration: 10,
      is_compatible: true,
      file_count: 5,
      verified: true,
      skipped_media_files: 0,
    };

    this.mockBackups.unshift(newBackup);

    const updatedSettings: BackupSettings = {
      ...settings,
      last_backup_at: nowIso,
      last_backup_status: "success",
      last_backup_error: null,
    };
    this.saveSettings(updatedSettings);

    return newBackup;
  }

  async listBackups(folder?: string): Promise<BackupMetadataView[]> {
    void folder;
    return [...this.mockBackups];
  }

  async verifyBackup(filePath: string): Promise<VerifyBackupResult> {
    void filePath;
    return {
      valid: true,
      manifest: {
        manifest_version: 1,
        app_version: "0.10.0-fase2n",
        created_at: new Date().toISOString(),
        last_applied_migration: 10,
        migrations_checksum: "mock_checksum",
        database_integrity_checked: true,
        files: [
          {
            path: "database/eventrent.db",
            byte_size: 1048576,
            sha256: "mock_sha256",
          },
        ],
        table_counts: { users: 1 },
        skipped_media_files: 0,
      },
      error_message: null,
      is_compatible: true,
    };
  }

  async scheduleRestore(
    filePath: string,
    password: string,
  ): Promise<RestorePendingMarker> {
    if (!password || password.trim().length === 0) {
      throw new Error("Senha é obrigatória para agendar restauração.");
    }
    return {
      backup_file_path: filePath,
      backup_sha256: "mock_sha256",
      scheduled_at: new Date().toISOString(),
      scheduled_by_user_id: "usr_mock_admin",
    };
  }

  async chooseFolder(): Promise<string | null> {
    return "C:\\EventRent\\Backups";
  }

  async chooseFile(): Promise<string | null> {
    return "C:\\EventRent\\Backups\\eventrent-backup-sample.eventrent-backup";
  }

  onStatusChanged(callback: () => void): () => void {
    void callback;
    return () => {};
  }
}
