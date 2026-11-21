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
  auth_epoch?: number;
}

export interface AppStatus {
  config: AppConfig;
  default_tenant_id: string;
  has_owner: boolean;
  session: SessionInfo | null;
  branding: PublicBranding | null;
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

export interface RecoverPasswordResult {
  new_recovery_key: string;
}

export interface RegenerateRecoveryKeyInput {
  current_password: string;
}

export interface RegenerateRecoveryKeyResult {
  new_recovery_key: string;
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
  auth_epoch?: number;
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
  auth_epoch?: number;
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
  auth_epoch?: number;
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
  SUPPLIER_READ: "supplier:read",
  SUPPLIER_MANAGE: "supplier:manage",
  VENUE_READ: "venue:read",
  VENUE_MANAGE: "venue:manage",
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
export type DocumentType = "cpf" | "cnpj" | "foreign" | "none";
export type SupplierCategory =
  | "sublocacao"
  | "manutencao"
  | "transporte"
  | "compras"
  | "servicos"
  | "outros";
export type CustomerRoleFilter = "customer" | "supplier" | "both" | "all";
export type CustomerRole = "customer" | "supplier";

export interface Customer {
  id: string;
  tenant_id: string;
  person_type: PersonType;
  name: string;
  trade_name?: string | null;
  document_type: DocumentType;
  document_number?: string | null;
  state_registration?: string | null;
  municipal_registration?: string | null;
  notes?: string | null;
  active: boolean;
  is_customer: boolean;
  is_supplier: boolean;
  search_text: string;
  name_sort: string;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface CustomerContact {
  id: string;
  tenant_id: string;
  customer_id: string;
  name: string;
  email?: string | null;
  phone?: string | null;
  role?: string | null;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface CustomerAddress {
  id: string;
  tenant_id: string;
  customer_id: string;
  label: string;
  street: string;
  number: string;
  complement?: string | null;
  neighborhood: string;
  city: string;
  state: string;
  postal_code: string;
  country: string;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface CustomerWithDetails {
  customer: Customer;
  contacts: CustomerContact[];
  addresses: CustomerAddress[];
  supplier_categories: SupplierCategory[];
}

export interface CreateContactInput {
  id?: string | null;
  name: string;
  email?: string | null;
  phone?: string | null;
  role?: string | null;
  is_primary: boolean;
}

export interface CreateAddressInput {
  id?: string | null;
  label: string;
  street: string;
  number: string;
  complement?: string | null;
  neighborhood: string;
  city: string;
  state: string;
  postal_code: string;
  country?: string | null;
  is_primary: boolean;
}

export interface CreateCustomerInput {
  person_type: PersonType;
  name: string;
  trade_name?: string | null;
  document_type?: DocumentType;
  document_number?: string | null;
  state_registration?: string | null;
  municipal_registration?: string | null;
  notes?: string | null;
  is_customer?: boolean;
  is_supplier?: boolean;
  supplier_categories?: SupplierCategory[];
  contacts: CreateContactInput[];
  addresses: CreateAddressInput[];
}

export interface UpdateCustomerInput extends CreateCustomerInput {
  expected_updated_at: string;
}

export interface CustomerFilter {
  query?: string;
  person_type?: PersonType;
  role_filter?: CustomerRoleFilter;
  cursor?: string;
  limit?: number;
}

export interface CustomerPage {
  items: Customer[];
  next_cursor?: string | null;
  has_more: boolean;
  total_count?: number | null;
}

export interface AllocationSettings {
  changeover_minutes: number;
  internal_transfer_minutes: number;
  external_transfer_minutes: number;
}

export interface UpdateAllocationSettingsInput {
  changeover_minutes: number;
  internal_transfer_minutes: number;
  external_transfer_minutes: number;
}

export interface Venue {
  id: string;
  tenant_id: string;
  name: string;
  street?: string | null;
  number?: string | null;
  complement?: string | null;
  neighborhood?: string | null;
  city?: string | null;
  state?: string | null;
  postal_code?: string | null;
  country: string;
  parking_notes?: string | null;
  loading_dock_notes?: string | null;
  access_hours?: string | null;
  internal_transfer_minutes?: number | null;
  notes?: string | null;
  active: boolean;
  search_text: string;
  name_sort: string;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface VenueSpace {
  id: string;
  tenant_id: string;
  venue_id: string;
  name: string;
  name_normalized: string;
  capacity_people?: number | null;
  area_sqm?: number | null;
  ceiling_height_m?: number | null;
  power_available?: string | null;
  loading_access?: string | null;
  rigging_points?: string | null;
  acoustics_notes?: string | null;
  setup_hours_notes?: string | null;
  noise_restrictions_notes?: string | null;
  load_dimensions_notes?: string | null;
  notes?: string | null;
  active: boolean;
  sort_order: number;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface VenueContact {
  id: string;
  tenant_id: string;
  venue_id: string;
  name: string;
  email?: string | null;
  phone?: string | null;
  role?: string | null;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
  created_by?: string | null;
}

export interface VenueWithDetails extends Venue {
  spaces: VenueSpace[];
  contacts: VenueContact[];
}

export interface VenueListItem extends Venue {
  spaces_count: number;
}

export interface VenuePage {
  items: VenueListItem[];
  next_cursor?: string | null;
  has_more: boolean;
  total_count?: number | null;
}

export interface VenueFilter {
  query?: string;
  active_only?: boolean;
  cursor?: string;
  limit?: number;
}

export interface VenueSpaceInput {
  id?: string | null;
  name: string;
  capacity_people?: number | null;
  area_sqm?: number | null;
  ceiling_height_m?: number | null;
  power_available?: string | null;
  loading_access?: string | null;
  rigging_points?: string | null;
  acoustics_notes?: string | null;
  setup_hours_notes?: string | null;
  noise_restrictions_notes?: string | null;
  load_dimensions_notes?: string | null;
  notes?: string | null;
  active?: boolean;
  sort_order?: number;
}

export interface VenueContactInput {
  id?: string | null;
  name: string;
  email?: string | null;
  phone?: string | null;
  role?: string | null;
  is_primary: boolean;
}

export interface CreateVenueInput {
  name: string;
  street?: string | null;
  number?: string | null;
  complement?: string | null;
  neighborhood?: string | null;
  city?: string | null;
  state?: string | null;
  postal_code?: string | null;
  country: string;
  parking_notes?: string | null;
  loading_dock_notes?: string | null;
  access_hours?: string | null;
  internal_transfer_minutes?: number | null;
  notes?: string | null;
  spaces: VenueSpaceInput[];
  contacts: VenueContactInput[];
}

export interface UpdateVenueInput extends CreateVenueInput {
  id: string;
  expected_updated_at: string;
}
