export type AttributeDataType = "text" | "number" | "boolean" | "select";
export type ControlMode = "serialized" | "quantity";
export type VoltageOption = "110" | "220" | "bivolt";
export type MediaMimeType =
  "image/webp" | "image/jpeg" | "image/png" | "application/pdf";
export type MediaKind = "image" | "document";
export type MediaDocType = "manual" | "datasheet" | "certificate" | "other";
export type MediaOrigin = "manufacturer" | "own" | "website" | "other";

export interface Brand {
  id: string;
  tenant_id: string;
  name: string;
  name_normalized: string;
  active: boolean;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface EquipmentCategory {
  id: string;
  tenant_id: string;
  parent_id: string | null;
  name: string;
  name_normalized: string;
  description: string | null;
  sort_order: number;
  active: boolean;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface AttributeDefinition {
  id: string;
  tenant_id: string;
  category_id: string;
  field_key: string;
  label: string;
  data_type: AttributeDataType;
  unit: string | null;
  choices: string[];
  is_required: boolean;
  sort_order: number;
  active: boolean;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface InheritedAttributeDefinition {
  definition: AttributeDefinition;
  source_category_id: string;
  source_category_name: string;
  is_inherited: boolean;
}

export interface EquipmentCategoryTreeItem {
  category: EquipmentCategory;
  depth: number;
  own_attributes: AttributeDefinition[];
  model_count: number;
}

export interface EquipmentModel {
  id: string;
  tenant_id: string;
  category_id: string;
  brand_id: string | null;
  name: string;
  name_sort: string;
  model_code: string | null;
  sku: string | null;
  ean: string | null;
  ncm: string | null;
  description: string | null;
  control_mode: ControlMode;
  weight_kg: number | null;
  length_cm: number | null;
  width_cm: number | null;
  height_cm: number | null;
  power_watts: number | null;
  voltage: VoltageOption | null;
  amperage: number | null;
  requires_case: boolean;
  units_per_case: number | null;
  daily_rate_cents: number | null;
  replacement_value_cents: number | null;
  cost_price_cents: number | null;
  active: boolean;
  search_text: string;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface EquipmentModelAttributeValue {
  id: string;
  tenant_id: string;
  model_id: string;
  definition_id: string;
  value_text: string | null;
  value_number: number | null;
  value_bool: boolean | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  created_by: string | null;
}

export interface EquipmentModelListItem {
  model: EquipmentModel;
  category_name: string;
  brand_name: string | null;
  primary_thumb_blob_id?: string | null;
  primary_thumb_mime_type?: string | null;
}

export interface EquipmentModelMediaView {
  id: string;
  tenant_id: string;
  model_id: string;
  asset_id: string;
  kind: MediaKind;
  blob_id: string;
  blob_sha256: string;
  mime_type: MediaMimeType;
  thumb_blob_id: string | null;
  thumb_sha256: string | null;
  thumb_mime_type: MediaMimeType | null;
  title: string;
  doc_type: MediaDocType | null;
  origin: MediaOrigin;
  credit_note: string | null;
  language: string | null;
  version_note: string | null;
  original_filename: string;
  width: number | null;
  height: number | null;
  byte_size: number;
  is_primary: boolean;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface AttachModelPhotoInput {
  model_id: string;
  title: string;
  origin: MediaOrigin;
  credit_note: string | null;
  original_filename: string;
  display_mime_type: MediaMimeType;
  display_bytes: Uint8Array;
  thumb_mime_type: MediaMimeType;
  thumb_bytes: Uint8Array;
}

export interface AttachModelDocumentInput {
  model_id: string;
  title: string;
  doc_type: MediaDocType;
  origin: MediaOrigin;
  credit_note: string | null;
  language: string | null;
  version_note: string | null;
  original_filename: string;
  mime_type: MediaMimeType;
  bytes: Uint8Array;
}

export interface UpdateModelMediaInput {
  media_link_id: string;
  title: string;
  doc_type: MediaDocType | null;
  origin: MediaOrigin;
  credit_note: string | null;
  language: string | null;
  version_note: string | null;
  expected_updated_at: string;
}

export interface ReorderModelMediaInput {
  model_id: string;
  kind: MediaKind;
  ordered_link_ids: string[];
}

export interface MediaGarbageCollectionResult {
  deleted_assets: number;
  deleted_blobs: number;
  deleted_files: number;
}

export interface EquipmentModelWithDetails {
  model: EquipmentModel;
  category_name: string;
  brand_name: string | null;
  effective_attributes: InheritedAttributeDefinition[];
  attribute_values: EquipmentModelAttributeValue[];
}

export interface CatalogListResult {
  items: EquipmentModelListItem[];
  next_cursor: string | null;
  total_count: number;
}

export interface SuggestedCategoriesResult {
  categories_created: number;
  attributes_created: number;
}

export interface CreateBrandInput {
  name: string;
  active: boolean;
}

export interface UpdateBrandInput {
  name: string;
  active: boolean;
  expected_updated_at: string;
}

export interface CreateCategoryInput {
  parent_id: string | null;
  name: string;
  description: string | null;
  sort_order: number;
  active: boolean;
}

export interface UpdateCategoryInput {
  parent_id: string | null;
  name: string;
  description: string | null;
  sort_order: number;
  active: boolean;
  expected_updated_at: string;
}

export interface CreateAttributeDefinitionInput {
  category_id: string;
  field_key: string;
  label: string;
  data_type: AttributeDataType;
  unit: string | null;
  choices: string[];
  is_required: boolean;
  sort_order: number;
  active: boolean;
}

export interface UpdateAttributeDefinitionInput {
  field_key: string;
  label: string;
  data_type: AttributeDataType;
  unit: string | null;
  choices: string[];
  is_required: boolean;
  sort_order: number;
  active: boolean;
  expected_updated_at: string;
}

export interface AttributeValueInput {
  definition_id: string;
  value_text: string | null;
  value_number: number | null;
  value_bool: boolean | null;
}

export interface CreateEquipmentModelInput {
  category_id: string;
  brand_id: string | null;
  name: string;
  model_code: string | null;
  sku: string | null;
  ean: string | null;
  ncm: string | null;
  description: string | null;
  control_mode: ControlMode;
  weight_kg: number | null;
  length_cm: number | null;
  width_cm: number | null;
  height_cm: number | null;
  power_watts: number | null;
  voltage: VoltageOption | null;
  amperage: number | null;
  requires_case: boolean;
  units_per_case: number | null;
  daily_rate_cents: number | null;
  replacement_value_cents: number | null;
  cost_price_cents: number | null;
  active: boolean;
  attribute_values: AttributeValueInput[];
}

export interface UpdateEquipmentModelInput extends CreateEquipmentModelInput {
  expected_updated_at: string;
}

export interface CatalogListFilter {
  query?: string | null;
  category_id?: string | null;
  brand_id?: string | null;
  control_mode?: ControlMode | null;
  active?: boolean | null;
  cursor?: string | null;
  limit: number;
}

export interface CatalogRepository {
  listBrands(includeInactive?: boolean): Promise<Brand[]>;
  createBrand(input: CreateBrandInput): Promise<Brand>;
  updateBrand(id: string, input: UpdateBrandInput): Promise<Brand>;
  deleteBrand(id: string): Promise<void>;

  listCategoriesTree(
    includeInactive?: boolean,
  ): Promise<EquipmentCategoryTreeItem[]>;
  getCategoryEffectiveAttributes(
    categoryId: string,
  ): Promise<InheritedAttributeDefinition[]>;
  createCategory(input: CreateCategoryInput): Promise<EquipmentCategory>;
  updateCategory(
    id: string,
    input: UpdateCategoryInput,
  ): Promise<EquipmentCategory>;
  deleteCategory(id: string): Promise<void>;

  createAttributeDefinition(
    input: CreateAttributeDefinitionInput,
  ): Promise<AttributeDefinition>;
  updateAttributeDefinition(
    id: string,
    input: UpdateAttributeDefinitionInput,
  ): Promise<AttributeDefinition>;
  deleteAttributeDefinition(id: string): Promise<void>;

  listEquipmentModels(filter: CatalogListFilter): Promise<CatalogListResult>;
  getEquipmentModel(id: string): Promise<EquipmentModelWithDetails>;
  createEquipmentModel(
    input: CreateEquipmentModelInput,
  ): Promise<EquipmentModelWithDetails>;
  updateEquipmentModel(
    id: string,
    input: UpdateEquipmentModelInput,
  ): Promise<EquipmentModelWithDetails>;
  setEquipmentModelActive(id: string, active: boolean): Promise<void>;
  deleteEquipmentModel(id: string): Promise<void>;

  loadSuggestedCategories(): Promise<SuggestedCategoriesResult>;

  listModelMedia(modelId: string): Promise<EquipmentModelMediaView[]>;
  getMediaBlobBytes(blobId: string): Promise<Uint8Array>;
  attachModelPhoto(
    input: AttachModelPhotoInput,
  ): Promise<EquipmentModelMediaView>;
  attachModelDocument(
    input: AttachModelDocumentInput,
  ): Promise<EquipmentModelMediaView>;
  updateModelMediaMetadata(
    input: UpdateModelMediaInput,
  ): Promise<EquipmentModelMediaView>;
  reorderModelMedia(
    input: ReorderModelMediaInput,
  ): Promise<EquipmentModelMediaView[]>;
  setPrimaryModelPhoto(mediaLinkId: string): Promise<EquipmentModelMediaView>;
  removeModelMedia(mediaLinkId: string): Promise<void>;
  collectUnreferencedMedia(): Promise<MediaGarbageCollectionResult>;
  openModelDocumentExternal(mediaLinkId: string): Promise<string>;
  saveModelDocumentAs(mediaLinkId: string): Promise<string | null>;
}
