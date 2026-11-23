import { z } from "zod";

export const BrandSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
});

export const AttributeTypeSchema = z.enum([
  "text",
  "number",
  "boolean",
  "select",
  "multi_select",
]);

export const AttributeDefinitionSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  category_id: z.string(),
  key: z.string(),
  label: z.string(),
  attribute_type: AttributeTypeSchema,
  unit: z.string().nullable().optional(),
  options: z.array(z.string()).nullable().optional(),
  required: z.boolean(),
  display_order: z.number(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
});

export const InheritedAttributeDefinitionSchema = z.object({
  definition: AttributeDefinitionSchema,
  origin_category_id: z.string(),
  origin_category_name: z.string(),
});

export const EquipmentCategorySchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  parent_id: z.string().nullable().optional(),
  level: z.number(),
  full_path: z.string(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
});

export interface EquipmentCategoryTreeItem {
  category: z.infer<typeof EquipmentCategorySchema>;
  children: EquipmentCategoryTreeItem[];
}

export const EquipmentCategoryTreeItemSchema: z.ZodType<EquipmentCategoryTreeItem> =
  z.lazy(() =>
    z.object({
      category: EquipmentCategorySchema,
      children: z.array(EquipmentCategoryTreeItemSchema),
    }),
  );

export const ModelAttributeValueSchema = z.object({
  definition_id: z.string(),
  key: z.string().optional(),
  label: z.string().optional(),
  value: z.unknown(),
});

export const EquipmentModelSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  brand_id: z.string(),
  category_id: z.string(),
  name: z.string(),
  model_code: z.string().nullable().optional(),
  barcode_ean_gtin: z.string().nullable().optional(),
  control_mode: z.string(),
  weight_kg: z.number().nullable().optional(),
  power_consumption_w: z.number().nullable().optional(),
  setup_minutes: z.number().nullable().optional(),
  strike_minutes: z.number().nullable().optional(),
  default_daily_price_cents: z.number().nullable().optional(),
  default_notes: z.string().nullable().optional(),
  search_text: z.string().optional(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
});

export const EquipmentModelWithDetailsSchema = z.object({
  model: EquipmentModelSchema,
  brand_name: z.string(),
  category_name: z.string(),
  attribute_values: z.array(z.unknown()),
  effective_attributes: z.array(InheritedAttributeDefinitionSchema).optional(),
});

export const CatalogListItemSchema = z.object({
  model: EquipmentModelSchema,
  brand_name: z.string(),
  category_name: z.string(),
  primary_thumb_blob_id: z.string().nullable().optional(),
  primary_thumb_mime_type: z.string().nullable().optional(),
});

export const CatalogListResultSchema = z.object({
  items: z.array(CatalogListItemSchema),
  next_cursor: z.string().nullable().optional(),
  total_count: z.number(),
});

export const SuggestedCategoriesResultSchema = z.object({
  categories_created: z.number(),
  attributes_created: z.number(),
});

export const EquipmentModelMediaViewSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  model_id: z.string(),
  asset_id: z.string(),
  blob_id: z.string(),
  kind: z.string(),
  doc_type: z.string().nullable().optional(),
  original_filename: z.string(),
  mime_type: z.string(),
  byte_size: z.number(),
  blob_sha256: z.string(),
  width: z.number().nullable().optional(),
  height: z.number().nullable().optional(),
  sort_order: z.number(),
  is_primary: z.boolean(),
  title: z.string().nullable().optional(),
  credit_note: z.string().nullable().optional(),
  origin: z.string(),
  version_note: z.string().nullable().optional(),
  language: z.string().nullable().optional(),
  thumb_blob_id: z.string().nullable().optional(),
  thumb_sha256: z.string().nullable().optional(),
  thumb_mime_type: z.string().nullable().optional(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const MediaGarbageCollectionResultSchema = z.object({
  deleted_files: z.number(),
  deleted_blobs: z.number(),
  deleted_assets: z.number(),
});
