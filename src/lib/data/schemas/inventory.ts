import { z } from "zod";

// ==========================================
// ENUMS & LITERALS
// ==========================================

export const UnitStatusSchema = z.enum([
  "available",
  "maintenance",
  "damaged",
  "lost",
  "decommissioned",
]);
export type UnitStatus = z.infer<typeof UnitStatusSchema>;

export const UnitOriginSchema = z.enum(["own", "third_party"]);
export type UnitOrigin = z.infer<typeof UnitOriginSchema>;

export const PhysicalConditionSchema = z.enum([
  "new",
  "good",
  "fair",
  "poor",
  "damaged",
]);
export type PhysicalCondition = z.infer<typeof PhysicalConditionSchema>;

export const StockMovementTypeSchema = z.enum([
  "purchase_in",
  "return_in",
  "transfer_out",
  "transfer_in",
  "status_change",
  "loss_out",
  "decommission_out",
  "adjustment",
]);
export type StockMovementType = z.infer<typeof StockMovementTypeSchema>;

// ==========================================
// 1. WAREHOUSES (Depósitos)
// ==========================================

export const WarehouseSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  code: z.string(),
  description: z.string().nullable().optional(),
  address_line1: z.string().nullable().optional(),
  address_line2: z.string().nullable().optional(),
  city: z.string().nullable().optional(),
  state: z.string().nullable().optional(),
  postal_code: z.string().nullable().optional(),
  is_default: z.boolean(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
});
export type Warehouse = z.infer<typeof WarehouseSchema>;

export const CreateWarehouseInputSchema = z.object({
  name: z.string().min(1, "Nome do depósito é obrigatório"),
  code: z.string().min(1, "Código do depósito é obrigatório"),
  description: z.string().nullable().optional(),
  address_line1: z.string().nullable().optional(),
  address_line2: z.string().nullable().optional(),
  city: z.string().nullable().optional(),
  state: z.string().nullable().optional(),
  postal_code: z.string().nullable().optional(),
  is_default: z.boolean().optional(),
});
export type CreateWarehouseInput = z.infer<typeof CreateWarehouseInputSchema>;

export const UpdateWarehouseInputSchema = z.object({
  expected_updated_at: z.string(),
  name: z.string().min(1, "Nome do depósito é obrigatório"),
  code: z.string().min(1, "Código do depósito é obrigatório"),
  description: z.string().nullable().optional(),
  address_line1: z.string().nullable().optional(),
  address_line2: z.string().nullable().optional(),
  city: z.string().nullable().optional(),
  state: z.string().nullable().optional(),
  postal_code: z.string().nullable().optional(),
  active: z.boolean().optional(),
});
export type UpdateWarehouseInput = z.infer<typeof UpdateWarehouseInputSchema>;

// ==========================================
// 2. EQUIPMENT UNITS (Unidades Físicas)
// ==========================================

export const EquipmentUnitSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  equipment_model_id: z.string(),
  warehouse_id: z.string(),
  patrimony_tag: z.string(),
  serial_number: z.string().nullable().optional(),
  status: UnitStatusSchema,
  physical_condition: PhysicalConditionSchema,
  notes: z.string().nullable().optional(),
  acquisition_date: z.string().nullable().optional(),
  warranty_expires_at: z.string().nullable().optional(),
  acquisition_cost_cents: z.number().int().nonnegative().nullable().optional(),
  supplier_id: z.string().nullable().optional(),
  invoice_number: z.string().nullable().optional(),
  origin: UnitOriginSchema,
  return_by: z.string().nullable().optional(),
  usage_hours: z.number().int().nonnegative(),
  location_note: z.string().nullable().optional(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
});
export type EquipmentUnit = z.infer<typeof EquipmentUnitSchema>;

export const EquipmentUnitDetailsSchema = z.object({
  unit: EquipmentUnitSchema,
  model_name: z.string(),
  warehouse_name: z.string(),
  model_thumbnail_url: z.string().nullable().optional(),
});
export type EquipmentUnitDetails = z.infer<typeof EquipmentUnitDetailsSchema>;

export const EquipmentUnitListPageSchema = z.object({
  items: z.array(EquipmentUnitDetailsSchema),
  next_cursor: z.string().nullable().optional(),
  has_more: z.boolean(),
});
export type EquipmentUnitListPage = z.infer<typeof EquipmentUnitListPageSchema>;

export const CreateEquipmentUnitInputSchema = z.object({
  equipment_model_id: z.string().min(1, "Modelo é obrigatório"),
  warehouse_id: z.string().nullable().optional(),
  serial_number: z.string().nullable().optional(),
  physical_condition: PhysicalConditionSchema.optional(),
  notes: z.string().nullable().optional(),
  acquisition_date: z.string().nullable().optional(),
  warranty_expires_at: z.string().nullable().optional(),
  acquisition_cost_cents: z.number().int().nonnegative().nullable().optional(),
  supplier_id: z.string().nullable().optional(),
  invoice_number: z.string().nullable().optional(),
  origin: UnitOriginSchema.optional(),
  return_by: z.string().nullable().optional(),
  usage_hours: z.number().int().nonnegative().optional(),
  location_note: z.string().nullable().optional(),
});
export type CreateEquipmentUnitInput = z.infer<
  typeof CreateEquipmentUnitInputSchema
>;

export const UpdateEquipmentUnitInputSchema = z.object({
  expected_updated_at: z.string(),
  warehouse_id: z.string().nullable().optional(),
  serial_number: z.string().nullable().optional(),
  status: UnitStatusSchema.optional(),
  physical_condition: PhysicalConditionSchema.optional(),
  notes: z.string().nullable().optional(),
  acquisition_date: z.string().nullable().optional(),
  warranty_expires_at: z.string().nullable().optional(),
  acquisition_cost_cents: z.number().int().nonnegative().nullable().optional(),
  supplier_id: z.string().nullable().optional(),
  invoice_number: z.string().nullable().optional(),
  origin: UnitOriginSchema.optional(),
  return_by: z.string().nullable().optional(),
  usage_hours: z.number().int().nonnegative().optional(),
  location_note: z.string().nullable().optional(),
  active: z.boolean().optional(),
  status_change_reason: z.string().nullable().optional(),
});
export type UpdateEquipmentUnitInput = z.infer<
  typeof UpdateEquipmentUnitInputSchema
>;

export const DecommissionEquipmentUnitInputSchema = z.object({
  expected_updated_at: z.string(),
  reason: z.string().min(1, "Motivo da baixa é obrigatório"),
});
export type DecommissionEquipmentUnitInput = z.infer<
  typeof DecommissionEquipmentUnitInputSchema
>;

export const EquipmentUnitFilterSchema = z.object({
  equipment_model_id: z.string().optional(),
  warehouse_id: z.string().optional(),
  status: UnitStatusSchema.optional(),
  active_only: z.boolean().optional(),
  search: z.string().optional(),
  cursor: z.string().optional(),
  limit: z.number().int().positive().optional(),
});
export type EquipmentUnitFilter = z.infer<typeof EquipmentUnitFilterSchema>;

// ==========================================
// 3. STOCK & MOVEMENTS (Estoque e Saldos)
// ==========================================

export const StockBalanceSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  warehouse_id: z.string(),
  equipment_model_id: z.string(),
  quantity_on_hand: z.number().int().nonnegative(),
  quantity_maintenance: z.number().int().nonnegative(),
  min_quantity: z.number().int().nonnegative(),
  updated_at: z.string(),
});
export type StockBalance = z.infer<typeof StockBalanceSchema>;

export const StockMovementSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  equipment_model_id: z.string(),
  equipment_unit_id: z.string().nullable().optional(),
  from_warehouse_id: z.string().nullable().optional(),
  to_warehouse_id: z.string().nullable().optional(),
  movement_type: StockMovementTypeSchema,
  quantity: z.number().int().positive(),
  reason: z.string(),
  reference_id: z.string().nullable().optional(),
  performed_by_user_id: z.string(),
  created_at: z.string(),
});
export type StockMovement = z.infer<typeof StockMovementSchema>;

export const RecordStockMovementInputSchema = z.object({
  equipment_model_id: z.string().min(1, "Modelo é obrigatório"),
  equipment_unit_id: z.string().nullable().optional(),
  from_warehouse_id: z.string().nullable().optional(),
  to_warehouse_id: z.string().nullable().optional(),
  movement_type: StockMovementTypeSchema,
  quantity: z.number().int().positive("Quantidade deve ser maior que zero"),
  reason: z.string().min(1, "Motivo é obrigatório"),
  reference_id: z.string().nullable().optional(),
});
export type RecordStockMovementInput = z.infer<
  typeof RecordStockMovementInputSchema
>;

export const StockMovementFilterSchema = z.object({
  equipment_model_id: z.string().optional(),
  equipment_unit_id: z.string().optional(),
  warehouse_id: z.string().optional(),
  movement_type: StockMovementTypeSchema.optional(),
  limit: z.number().int().positive().optional(),
});
export type StockMovementFilter = z.infer<typeof StockMovementFilterSchema>;
