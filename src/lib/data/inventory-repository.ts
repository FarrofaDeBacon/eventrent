import type {
  Warehouse,
  CreateWarehouseInput,
  UpdateWarehouseInput,
  EquipmentUnit,
  EquipmentUnitDetails,
  EquipmentUnitListPage,
  CreateEquipmentUnitInput,
  UpdateEquipmentUnitInput,
  DecommissionEquipmentUnitInput,
  EquipmentUnitFilter,
  StockBalance,
  StockMovement,
  RecordStockMovementInput,
  StockMovementFilter,
} from "./schemas/inventory";

export interface InventoryRepository {
  // Warehouses (Depósitos)
  listWarehouses(includeInactive?: boolean): Promise<Warehouse[]>;
  getWarehouse(id: string): Promise<Warehouse>;
  createWarehouse(input: CreateWarehouseInput): Promise<Warehouse>;
  updateWarehouse(id: string, input: UpdateWarehouseInput): Promise<Warehouse>;
  setDefaultWarehouse(id: string): Promise<void>;
  deleteWarehouse(id: string): Promise<void>;

  // Equipment Units (Unidades Físicas)
  listEquipmentUnits(
    filter?: EquipmentUnitFilter,
  ): Promise<EquipmentUnitListPage>;
  getEquipmentUnit(id: string): Promise<EquipmentUnitDetails>;
  createEquipmentUnit(input: CreateEquipmentUnitInput): Promise<EquipmentUnit>;
  updateEquipmentUnit(
    id: string,
    input: UpdateEquipmentUnitInput,
  ): Promise<EquipmentUnit>;
  decommissionEquipmentUnit(
    id: string,
    input: DecommissionEquipmentUnitInput,
  ): Promise<EquipmentUnit>;
  deleteEquipmentUnit(id: string): Promise<void>;

  // Stock & Balances (Estoque e Saldos)
  getStockBalance(
    warehouseId: string,
    equipmentModelId: string,
  ): Promise<StockBalance>;
  listStockBalances(
    equipmentModelId?: string,
    warehouseId?: string,
  ): Promise<StockBalance[]>;
  recordStockMovement(input: RecordStockMovementInput): Promise<StockMovement>;
  listStockMovements(filter?: StockMovementFilter): Promise<StockMovement[]>;
}
