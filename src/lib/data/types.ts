export type OperationMode = "local" | "cloud";

export interface AppConfig {
  mode: OperationMode | null;
  setup_completed: boolean;
  inactivity_timeout_minutes: number;
}

export interface SessionInfo {
  user_id: string;
  tenant_id: string;
  name: string;
  email: string;
  role_id: string;
  role_name: string;
  role_slug: string;
  permissions: string[];
  authenticated_at: string;
  last_activity_at: string;
  inactivity_timeout_minutes: number;
}

export interface AppStatus {
  config: AppConfig;
  default_tenant_id: string;
  has_owner: boolean;
  session: SessionInfo | null;
  branding: TenantBranding | null;
}

export interface InitialSetupInput {
  mode: OperationMode;
  company_name: string;
  owner_name: string;
  owner_email: string;
  owner_password: string;
}

export interface InitialSetupResult {
  tenant_id: string;
  recovery_key: string;
  session: SessionInfo;
  branding: TenantBranding;
}

export interface LoginInput {
  email: string;
  password: string;
}

export interface RecoverPasswordInput {
  email: string;
  recovery_key: string;
  new_password: string;
}

export interface UserWithRole {
  id: string;
  tenant_id: string;
  name: string;
  email: string;
  role_id: string;
  role_name: string;
  role_slug: string;
  permissions: string[];
  active: boolean;
  last_login_at: string | null;
  failed_attempts: number;
  locked_until: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface CreateUserInput {
  name: string;
  email: string;
  password: string;
  role_id: string;
}

export interface UpdateUserInput {
  name: string;
  email: string;
  role_id: string;
}

export interface ResetPasswordInput {
  new_password: string;
}

export interface RoleWithPermissions {
  id: string;
  tenant_id: string;
  name: string;
  slug: string;
  description: string | null;
  is_system: boolean;
  permissions: string[];
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface TenantBranding {
  id: string;
  tenant_id: string;
  company_name: string;
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  logo_url: string | null;
  favicon_url: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface UpdateBrandingInput {
  company_name: string;
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  logo_url?: string | null;
}

export const PERMISSIONS = {
  DASHBOARD_VIEW: "dashboard:view",
  USERS_VIEW: "users:view",
  USERS_CREATE: "users:create",
  USERS_EDIT: "users:edit",
  USERS_DEACTIVATE: "users:deactivate",
  USERS_RESET_PASSWORD: "users:reset_password",
  ROLES_VIEW: "roles:view",
  ROLES_MANAGE: "roles:manage",
  BRANDING_VIEW: "branding:view",
  BRANDING_EDIT: "branding:edit",
  SETTINGS_VIEW: "settings:view",
  SETTINGS_EDIT: "settings:edit",
  AUDIT_VIEW: "audit:view",
  INVENTORY_VIEW: "inventory:view",
  INVENTORY_EDIT: "inventory:edit",
  QUOTES_VIEW: "quotes:view",
  QUOTES_EDIT: "quotes:edit",
  OPERATIONS_VIEW: "operations:view",
  OPERATIONS_EXECUTE: "operations:execute",
  FINANCE_VIEW: "finance:view",
  FINANCE_EDIT: "finance:edit",
} as const;

export const DEFAULT_COMPANY_NAME = "Minha empresa";
