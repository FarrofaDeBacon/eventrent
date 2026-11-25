import { invoke } from "@tauri-apps/api/core";
import type { InventoryRepository } from "./inventory-repository";
import type {
  Warehouse,
  CreateWarehouseInput,
  UpdateWarehouseInput,
  EquipmentUnit,
  EquipmentUnitDetails,
  CreateEquipmentUnitInput,
  UpdateEquipmentUnitInput,
  EquipmentUnitFilter,
  StockBalance,
  StockMovement,
  RecordStockMovementInput,
  StockMovementFilter,
  EquipmentUnitMedia,
  AttachUnitMediaInput,
} from "./schemas/inventory";

export class TauriInventoryRepository implements InventoryRepository {
  async listWarehouses(includeInactive?: boolean): Promise<Warehouse[]> {
    return invoke<Warehouse[]>("list_warehouses", {
      includeInactive: includeInactive ?? false,
    });
  }

  async getWarehouse(id: string): Promise<Warehouse> {
    return invoke<Warehouse>("get_warehouse", { id });
  }

  async createWarehouse(input: CreateWarehouseInput): Promise<Warehouse> {
    return invoke<Warehouse>("create_warehouse", { input });
  }

  async updateWarehouse(
    id: string,
    input: UpdateWarehouseInput,
  ): Promise<Warehouse> {
    return invoke<Warehouse>("update_warehouse", { id, input });
  }

  async setDefaultWarehouse(id: string): Promise<void> {
    return invoke<void>("set_default_warehouse", { id });
  }

  async deleteWarehouse(id: string): Promise<void> {
    return invoke<void>("delete_warehouse", { id });
  }

  async listEquipmentUnits(
    filter?: EquipmentUnitFilter,
  ): Promise<EquipmentUnitDetails[]> {
    return invoke<EquipmentUnitDetails[]>("list_equipment_units", {
      filter: filter ?? null,
    });
  }

  async getEquipmentUnit(id: string): Promise<EquipmentUnitDetails> {
    return invoke<EquipmentUnitDetails>("get_equipment_unit", { id });
  }

  async createEquipmentUnit(
    input: CreateEquipmentUnitInput,
  ): Promise<EquipmentUnit> {
    return invoke<EquipmentUnit>("create_equipment_unit", { input });
  }

  async updateEquipmentUnit(
    id: string,
    input: UpdateEquipmentUnitInput,
  ): Promise<EquipmentUnit> {
    return invoke<EquipmentUnit>("update_equipment_unit", { id, input });
  }

  async deleteEquipmentUnit(id: string): Promise<void> {
    return invoke<void>("delete_equipment_unit", { id });
  }

  async getStockBalance(
    warehouseId: string,
    equipmentModelId: string,
  ): Promise<StockBalance> {
    return invoke<StockBalance>("get_stock_balance", {
      warehouseId,
      equipmentModelId,
    });
  }

  async listStockBalances(
    equipmentModelId?: string,
    warehouseId?: string,
  ): Promise<StockBalance[]> {
    return invoke<StockBalance[]>("list_stock_balances", {
      equipmentModelId: equipmentModelId ?? null,
      warehouseId: warehouseId ?? null,
    });
  }

  async recordStockMovement(
    input: RecordStockMovementInput,
  ): Promise<StockMovement> {
    return invoke<StockMovement>("record_stock_movement", { input });
  }

  async listStockMovements(
    filter?: StockMovementFilter,
  ): Promise<StockMovement[]> {
    return invoke<StockMovement[]>("list_stock_movements", {
      filter: filter ?? null,
    });
  }

  async attachUnitMedia(
    input: AttachUnitMediaInput,
  ): Promise<EquipmentUnitMedia> {
    return invoke<EquipmentUnitMedia>("attach_unit_media", { input });
  }

  async detachUnitMedia(id: string): Promise<void> {
    return invoke<void>("detach_unit_media", { id });
  }
}
