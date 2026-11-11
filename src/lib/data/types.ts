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
  logo_dark_url: string | null;
  icon_url: string | null;
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
  logo_dark_url?: string | null;
  icon_url?: string | null;
}

export interface PublicBranding {
  company_name: string;
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  logo_url: string | null;
  logo_dark_url: string | null;
  icon_url: string | null;
}

/**
 * Catálogo de permissões `resource:action`.
 * Fonte única de verdade: `crates/core/src/domain/permission.rs`.
 * A paridade é verificada pelo teste Rust `permission_catalog_matches_frontend` em `src-tauri`.
 */
export const PERMISSIONS = {
  USER_READ: "user:read",
  USER_MANAGE: "user:manage",
  ROLE_READ: "role:read",
  ROLE_MANAGE: "role:manage",
  BRANDING_READ: "branding:read",
  BRANDING_MANAGE: "branding:manage",
  CUSTOMER_READ: "customer:read",
  CUSTOMER_MANAGE: "customer:manage",
  CATALOG_READ: "catalog:read",
  CATALOG_MANAGE: "catalog:manage",
  RESERVATION_READ: "reservation:read",
  RESERVATION_MANAGE: "reservation:manage",
  QUOTE_READ: "quote:read",
  QUOTE_MANAGE: "quote:manage",
  WAREHOUSE_READ: "warehouse:read",
  WAREHOUSE_OPERATE: "warehouse:operate",
  FINANCE_READ: "finance:read",
  FINANCE_MANAGE: "finance:manage",
  AUDIT_READ: "audit:read",
  SETTINGS_READ: "settings:read",
  SETTINGS_MANAGE: "settings:manage",
} as const;

/** Slugs dos perfis de sistema (fonte: `crates/core/src/domain/permission.rs`). */
export const ROLE_SLUGS = {
  OWNER: "owner",
  COMMERCIAL: "commercial",
  WAREHOUSE: "warehouse",
  TECHNICIAN: "technician",
  FINANCE: "finance",
} as const;

export const DEFAULT_COMPANY_NAME = "Minha empresa";

// =========================================================================
// TIPOS DO DOMÍNIO DE CLIENTES (FASE 1B.2)
// =========================================================================

export type PersonType = "individual" | "legal";

export interface Customer {
  id: string;
  tenant_id: string;
  person_type: PersonType;
  name: string;
  trade_name?: string | null;
  document_number: string;
  state_registration?: string | null;
  municipal_registration?: string | null;
  notes?: string | null;
  search_text: string;
  name_sort: string;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface CustomerContact {
  id: string;
  customer_id: string;
  name: string;
  email?: string | null;
  phone?: string | null;
  role?: string | null;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
}

export interface CustomerAddress {
  id: string;
  customer_id: string;
  label: string;
  street: string;
  number: string;
  complement?: string | null;
  neighborhood: string;
  city: string;
  state: string;
  postal_code: string;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
}

export interface CustomerWithDetails {
  customer: Customer;
  contacts: CustomerContact[];
  addresses: CustomerAddress[];
}

export interface CreateContactInput {
  name: string;
  email?: string | null;
  phone?: string | null;
  role?: string | null;
  is_primary: boolean;
}

export interface CreateAddressInput {
  label: string;
  street: string;
  number: string;
  complement?: string | null;
  neighborhood: string;
  city: string;
  state: string;
  postal_code: string;
  is_primary: boolean;
}

export interface CreateCustomerInput {
  person_type: PersonType;
  name: string;
  trade_name?: string | null;
  document_number: string;
  state_registration?: string | null;
  municipal_registration?: string | null;
  notes?: string | null;
  contacts: CreateContactInput[];
  addresses: CreateAddressInput[];
}

export type UpdateCustomerInput = CreateCustomerInput;

export interface CustomerFilter {
  query?: string;
  person_type?: PersonType;
  cursor?: string;
  limit?: number;
}

export interface CustomerPage {
  items: Customer[];
  next_cursor?: string | null;
  has_more: boolean;
  total_count?: number | null;
}
