import { z } from "zod";

export * from "./errors";
export * from "./auth";
export * from "./users";
export * from "./customers";
export * from "./venues";
export * from "./catalog";
export * from "./settings";
export * from "./backup";

import {
  AppStatusSchema,
  InitialSetupResultSchema,
  SessionInfoSchema,
  RecoverPasswordResultSchema,
  RegenerateRecoveryKeyResultSchema,
  PublicBrandingSchema,
  TenantBrandingSchema,
} from "./auth";
import { RoleWithPermissionsSchema, UserWithRoleSchema } from "./users";
import { CustomerWithDetailsSchema, CustomerPageSchema } from "./customers";
import { VenueWithDetailsSchema, VenuePageSchema } from "./venues";
import {
  BrandSchema,
  EquipmentCategorySchema,
  EquipmentCategoryTreeItemSchema,
  InheritedAttributeDefinitionSchema,
  AttributeDefinitionSchema,
  EquipmentModelWithDetailsSchema,
  CatalogListResultSchema,
  SuggestedCategoriesResultSchema,
  EquipmentModelMediaViewSchema,
  MediaGarbageCollectionResultSchema,
} from "./catalog";
import { AllocationSettingsSchema } from "./settings";
import {
  BackupStatusSchema,
  BackupSettingsSchema,
  BackupMetadataViewSchema,
  VerifyBackupResultSchema,
  RestorePendingMarkerSchema,
} from "./backup";

/**
 * Registro de schemas de retorno em runtime para todos os comandos IPC do Tauri.
 * Cada comando que produz dados possui seu schema rigorosamente tipado.
 * Comandos void retornam `z.void()`.
 */
export const commandReturnSchemas: Record<string, z.ZodTypeAny> = {
  // Auth & Setup
  get_app_status: AppStatusSchema,
  complete_initial_setup: InitialSetupResultSchema,
  login: SessionInfoSchema,
  logout: z.void(),
  get_current_session: SessionInfoSchema.nullable(),
  recover_password: RecoverPasswordResultSchema,
  regenerate_recovery_key: RegenerateRecoveryKeyResultSchema,

  // Users & Roles
  list_roles: z.array(RoleWithPermissionsSchema),
  list_users: z.array(UserWithRoleSchema),
  create_user: UserWithRoleSchema,
  update_user: UserWithRoleSchema,
  set_user_active: UserWithRoleSchema,
  reset_user_password: z.void(),

  // Branding
  get_public_branding: PublicBrandingSchema.nullable(),
  get_branding: TenantBrandingSchema.nullable(),
  save_branding: TenantBrandingSchema,

  // Customers
  list_customers: CustomerPageSchema,
  get_customer: CustomerWithDetailsSchema,
  create_customer: CustomerWithDetailsSchema,
  update_customer: CustomerWithDetailsSchema,
  set_customer_active: CustomerWithDetailsSchema,
  delete_customer: z.void(),
  add_customer_role: CustomerWithDetailsSchema,

  // Venues
  list_venues: VenuePageSchema,
  get_venue: VenueWithDetailsSchema,
  create_venue: VenueWithDetailsSchema,
  update_venue: VenueWithDetailsSchema,
  set_venue_active: VenueWithDetailsSchema,
  delete_venue: z.void(),
  check_venue_duplicate_name_city: z.boolean(),

  // Catalog - Brands
  list_brands: z.array(BrandSchema),
  create_brand: BrandSchema,
  update_brand: BrandSchema,
  delete_brand: z.void(),

  // Catalog - Categories & Attributes
  list_categories_tree: z.array(EquipmentCategoryTreeItemSchema),
  get_category_effective_attributes: z.array(
    InheritedAttributeDefinitionSchema,
  ),
  create_category: EquipmentCategorySchema,
  update_category: EquipmentCategorySchema,
  delete_category: z.void(),
  create_attribute_definition: AttributeDefinitionSchema,
  update_attribute_definition: AttributeDefinitionSchema,
  delete_attribute_definition: z.void(),
  load_suggested_catalog_categories: SuggestedCategoriesResultSchema,

  // Catalog - Models
  list_equipment_models: CatalogListResultSchema,
  get_equipment_model: EquipmentModelWithDetailsSchema,
  create_equipment_model: EquipmentModelWithDetailsSchema,
  update_equipment_model: EquipmentModelWithDetailsSchema,
  set_equipment_model_active: z.void(),
  delete_equipment_model: z.void(),

  // Catalog - Media
  list_model_media: z.array(EquipmentModelMediaViewSchema),
  get_media_blob_bytes: z.union([
    z.instanceof(Uint8Array),
    z.instanceof(ArrayBuffer),
    z.array(z.number()),
  ]),
  attach_model_photo: EquipmentModelMediaViewSchema,
  attach_model_document: EquipmentModelMediaViewSchema,
  update_model_media_metadata: EquipmentModelMediaViewSchema,
  reorder_model_media: z.array(EquipmentModelMediaViewSchema),
  set_primary_model_photo: EquipmentModelMediaViewSchema,
  remove_model_media: z.void(),
  collect_unreferenced_media: MediaGarbageCollectionResultSchema,
  open_model_document_external: z.string(),
  save_model_document_as: z.string().nullable(),

  // Settings
  get_allocation_settings: AllocationSettingsSchema,
  update_allocation_settings: AllocationSettingsSchema,

  // Backup & Restore (ADR-0017)
  get_backup_status: BackupStatusSchema,
  set_backup_settings: BackupSettingsSchema,
  run_backup_now: BackupMetadataViewSchema,
  list_backups: z.array(BackupMetadataViewSchema),
  verify_backup: VerifyBackupResultSchema,
  schedule_restore: RestorePendingMarkerSchema,
  choose_backup_folder: z.string().nullable(),
  choose_backup_file: z.string().nullable(),
};
