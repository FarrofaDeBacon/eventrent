import { z } from "zod";

export const BackupSettingsSchema = z.object({
  destination_folder: z.string(),
  schedule_enabled: z.boolean(),
  retention_daily: z.number().int().nonnegative(),
  retention_weekly: z.number().int().nonnegative(),
  retention_monthly: z.number().int().nonnegative(),
  last_attempt_at: z.string().nullable().optional(),
  last_success_at: z.string().nullable().optional(),
  last_backup_at: z.string().nullable(),
  last_backup_status: z.string().nullable(),
  last_backup_error: z.string().nullable(),
});
export type BackupSettings = z.infer<typeof BackupSettingsSchema>;

export const BackupStatusSchema = z.object({
  settings: BackupSettingsSchema,
  is_backup_due: z.boolean(),
  days_since_last_backup: z.number().int().nonnegative().nullable(),
  is_stale: z.boolean(),
});
export type BackupStatus = z.infer<typeof BackupStatusSchema>;

export const BackupFileEntrySchema = z.object({
  path: z.string(),
  byte_size: z.number().nonnegative(),
  sha256: z.string(),
});
export type BackupFileEntry = z.infer<typeof BackupFileEntrySchema>;

export const BackupManifestSchema = z.object({
  manifest_version: z.number().int(),
  app_version: z.string(),
  created_at: z.string(),
  last_applied_migration: z.number().int(),
  migrations_checksum: z.string(),
  database_integrity_checked: z.boolean(),
  files: z.array(BackupFileEntrySchema),
  table_counts: z.record(z.string(), z.number().int()),
  skipped_media_files: z.number().int().nonnegative().default(0),
});
export type BackupManifest = z.infer<typeof BackupManifestSchema>;

export const BackupMetadataViewSchema = z.object({
  file_name: z.string(),
  file_path: z.string(),
  byte_size: z.number().nonnegative(),
  created_at: z.string(),
  app_version: z.string(),
  last_applied_migration: z.number().int(),
  is_compatible: z.boolean(),
  file_count: z.number().int().nonnegative(),
  verified: z.boolean(),
  skipped_media_files: z.number().int().nonnegative().default(0),
});
export type BackupMetadataView = z.infer<typeof BackupMetadataViewSchema>;

export const VerifyBackupResultSchema = z.object({
  valid: z.boolean(),
  manifest: BackupManifestSchema.nullable(),
  error_message: z.string().nullable(),
  is_compatible: z.boolean(),
});
export type VerifyBackupResult = z.infer<typeof VerifyBackupResultSchema>;

export const RestorePendingMarkerSchema = z.object({
  backup_file_path: z.string(),
  backup_sha256: z.string(),
  scheduled_at: z.string(),
  scheduled_by_user_id: z.string(),
});
export type RestorePendingMarker = z.infer<typeof RestorePendingMarkerSchema>;

export const SetBackupSettingsInputSchema = z.object({
  destination_folder: z.string().min(1, "Pasta de destino é obrigatória"),
  schedule_enabled: z.boolean(),
  retention_daily: z.number().int().min(1).max(365),
  retention_weekly: z.number().int().min(1).max(52),
  retention_monthly: z.number().int().min(1).max(120),
});
export type SetBackupSettingsInput = z.infer<
  typeof SetBackupSettingsInputSchema
>;
