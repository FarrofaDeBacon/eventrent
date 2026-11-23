import { z } from "zod";

export const OperationModeSchema = z.enum(["local", "cloud"]);

export const AppConfigSchema = z.object({
  mode: OperationModeSchema.nullable(),
  setup_completed: z.boolean(),
  inactivity_timeout_minutes: z.number(),
});

export const SessionInfoSchema = z.object({
  user_id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  email: z.string(),
  role_id: z.string(),
  role_name: z.string(),
  role_slug: z.string(),
  permissions: z.array(z.string()),
  authenticated_at: z.string(),
  last_activity_at: z.string(),
  inactivity_timeout_minutes: z.number(),
  auth_epoch: z.number().optional(),
});

export const PublicBrandingSchema = z.object({
  company_name: z.string(),
  primary_color: z.string(),
  secondary_color: z.string(),
  accent_color: z.string(),
  logo_url: z.string().nullable(),
  logo_dark_url: z.string().nullable(),
  icon_url: z.string().nullable(),
  trading_name: z.string().nullable().optional(),
  updated_at: z.string().optional(),
});

export const TenantBrandingSchema = z.object({
  company_name: z.string(),
  primary_color: z.string(),
  secondary_color: z.string(),
  accent_color: z.string(),
  logo_url: z.string().nullable(),
  logo_dark_url: z.string().nullable(),
  icon_url: z.string().nullable(),
  trading_name: z.string().nullable().optional(),
  updated_at: z.string().optional(),
  id: z.string().optional(),
  tenant_id: z.string().optional(),
  favicon_url: z.string().nullable().optional(),
  created_at: z.string().optional(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
  auth_epoch: z.number().optional(),
});

export const AppStatusSchema = z.object({
  config: AppConfigSchema,
  default_tenant_id: z.string(),
  has_owner: z.boolean(),
  session: SessionInfoSchema.nullable(),
  branding: PublicBrandingSchema.nullable(),
});

export const InitialSetupResultSchema = z.object({
  tenant_id: z.string(),
  recovery_key: z.string(),
  session: SessionInfoSchema,
  branding: TenantBrandingSchema,
});

export const RecoverPasswordResultSchema = z.object({
  new_recovery_key: z.string(),
});

export const RegenerateRecoveryKeyResultSchema = z.object({
  new_recovery_key: z.string(),
});
