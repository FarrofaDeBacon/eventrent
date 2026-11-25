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

export class MockInventoryRepository implements InventoryRepository {
  public warehouses: Warehouse[] = [
    {
      id: "wh_mock_01",
      tenant_id: "tenant_default",
      name: "Galpão Principal",
      code: "GALPAO-01",
      description: "Depósito central de equipamentos",
      address_line1: "Av. Industrial, 1000",
      address_line2: null,
      city: "São Paulo",
      state: "SP",
      postal_code: "01000-000",
      is_default: true,
      active: true,
      created_at: "2026-11-20T10:00:00Z",
      updated_at: "2026-11-20T10:00:00Z",
      deleted_at: null,
      created_by: "usr_mock",
    },
  ];

  public units: EquipmentUnit[] = [];
  public balances: StockBalance[] = [];
  public movements: StockMovement[] = [];
  public unitMedia: EquipmentUnitMedia[] = [];

  private patrimonySeq = 1;

  async listWarehouses(includeInactive?: boolean): Promise<Warehouse[]> {
    if (includeInactive) {
      return [...this.warehouses.filter((w) => !w.deleted_at)];
    }
    return [...this.warehouses.filter((w) => w.active && !w.deleted_at)];
  }

  async getWarehouse(id: string): Promise<Warehouse> {
    const wh = this.warehouses.find((w) => w.id === id && !w.deleted_at);
    if (!wh) throw new Error(`Depósito '${id}' não encontrado.`);
    return { ...wh };
  }

  async createWarehouse(input: CreateWarehouseInput): Promise<Warehouse> {
    const now = new Date().toISOString();
    if (input.is_default) {
      for (const w of this.warehouses) {
        w.is_default = false;
      }
    }
    const wh: Warehouse = {
      id: `wh_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
      tenant_id: "tenant_default",
      name: input.name,
      code: input.code.toUpperCase(),
      description: input.description ?? null,
      address_line1: input.address_line1 ?? null,
      address_line2: input.address_line2 ?? null,
      city: input.city ?? null,
      state: input.state ?? null,
      postal_code: input.postal_code ?? null,
      is_default: input.is_default ?? false,
      active: true,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "usr_mock",
    };
    this.warehouses.push(wh);
    return { ...wh };
  }

  async updateWarehouse(
    id: string,
    input: UpdateWarehouseInput,
  ): Promise<Warehouse> {
    const idx = this.warehouses.findIndex((w) => w.id === id && !w.deleted_at);
    if (idx === -1) throw new Error(`Depósito '${id}' não encontrado.`);
    const wh = this.warehouses[idx];
    wh.name = input.name;
    wh.code = input.code.toUpperCase();
    wh.description = input.description ?? null;
    wh.address_line1 = input.address_line1 ?? null;
    wh.address_line2 = input.address_line2 ?? null;
    wh.city = input.city ?? null;
    wh.state = input.state ?? null;
    wh.postal_code = input.postal_code ?? null;
    if (input.active !== undefined) wh.active = input.active;
    wh.updated_at = new Date().toISOString();
    return { ...wh };
  }

  async setDefaultWarehouse(id: string): Promise<void> {
    const idx = this.warehouses.findIndex((w) => w.id === id && !w.deleted_at);
    if (idx === -1) throw new Error(`Depósito '${id}' não encontrado.`);
    for (const w of this.warehouses) {
      w.is_default = false;
    }
    this.warehouses[idx].is_default = true;
    this.warehouses[idx].updated_at = new Date().toISOString();
  }

  async deleteWarehouse(id: string): Promise<void> {
    const idx = this.warehouses.findIndex((w) => w.id === id && !w.deleted_at);
    if (idx === -1) throw new Error(`Depósito '${id}' não encontrado.`);
    this.warehouses[idx].deleted_at = new Date().toISOString();
    this.warehouses[idx].is_default = false;
  }

  async listEquipmentUnits(
    filter?: EquipmentUnitFilter,
  ): Promise<EquipmentUnitDetails[]> {
    let list = this.units.filter((u) => !u.deleted_at);
    if (filter?.equipment_model_id) {
      list = list.filter(
        (u) => u.equipment_model_id === filter.equipment_model_id,
      );
    }
    if (filter?.warehouse_id) {
      list = list.filter((u) => u.warehouse_id === filter.warehouse_id);
    }
    if (filter?.status) {
      list = list.filter((u) => u.status === filter.status);
    }
    if (filter?.active_only) {
      list = list.filter((u) => u.active);
    }
    if (filter?.search) {
      const q = filter.search.toLowerCase();
      list = list.filter(
        (u) =>
          u.patrimony_tag.toLowerCase().includes(q) ||
          (u.serial_number && u.serial_number.toLowerCase().includes(q)),
      );
    }

    return list.map((unit) => ({
      unit: { ...unit },
      model_name: "Modelo Mock",
      warehouse_name:
        this.warehouses.find((w) => w.id === unit.warehouse_id)?.name ??
        "Depósito Mock",
      media: this.unitMedia
        .filter((m) => m.equipment_unit_id === unit.id)
        .map((m) => ({ ...m })),
    }));
  }

  async getEquipmentUnit(id: string): Promise<EquipmentUnitDetails> {
    const unit = this.units.find((u) => u.id === id && !u.deleted_at);
    if (!unit) throw new Error(`Unidade '${id}' não encontrada.`);
    return {
      unit: { ...unit },
      model_name: "Modelo Mock",
      warehouse_name:
        this.warehouses.find((w) => w.id === unit.warehouse_id)?.name ??
        "Depósito Mock",
      media: this.unitMedia
        .filter((m) => m.equipment_unit_id === unit.id)
        .map((m) => ({ ...m })),
    };
  }

  async createEquipmentUnit(
    input: CreateEquipmentUnitInput,
  ): Promise<EquipmentUnit> {
    const now = new Date().toISOString();
    const tagNum = this.patrimonySeq++;
    const patrimony_tag = `EQ${String(tagNum).padStart(6, "0")}`;
    const whId = input.warehouse_id ?? this.warehouses[0].id;

    const unit: EquipmentUnit = {
      id: `unit_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
      tenant_id: "tenant_default",
      equipment_model_id: input.equipment_model_id,
      warehouse_id: whId,
      patrimony_tag,
      serial_number: input.serial_number ?? null,
      status: "available",
      physical_condition: input.physical_condition ?? "good",
      notes: input.notes ?? null,
      acquisition_date: input.acquisition_date ?? null,
      warranty_expires_at: input.warranty_expires_at ?? null,
      active: true,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "usr_mock",
    };
    this.units.push(unit);

    await this.recordStockMovement({
      equipment_model_id: unit.equipment_model_id,
      equipment_unit_id: unit.id,
      from_warehouse_id: null,
      to_warehouse_id: whId,
      movement_type: "purchase_in",
      quantity: 1,
      reason: `Tombamento patrimonial ${patrimony_tag}`,
      reference_id: null,
    });

    return { ...unit };
  }

  async updateEquipmentUnit(
    id: string,
    input: UpdateEquipmentUnitInput,
  ): Promise<EquipmentUnit> {
    const unit = this.units.find((u) => u.id === id && !u.deleted_at);
    if (!unit) throw new Error(`Unidade '${id}' não encontrada.`);

    if (input.warehouse_id !== undefined && input.warehouse_id !== null) {
      unit.warehouse_id = input.warehouse_id;
    }
    if (input.serial_number !== undefined)
      unit.serial_number = input.serial_number;
    if (input.status !== undefined) unit.status = input.status;
    if (input.physical_condition !== undefined)
      unit.physical_condition = input.physical_condition;
    if (input.notes !== undefined) unit.notes = input.notes;
    if (input.acquisition_date !== undefined)
      unit.acquisition_date = input.acquisition_date;
    if (input.warranty_expires_at !== undefined)
      unit.warranty_expires_at = input.warranty_expires_at;
    if (input.active !== undefined) unit.active = input.active;
    unit.updated_at = new Date().toISOString();

    return { ...unit };
  }

  async deleteEquipmentUnit(id: string): Promise<void> {
    const unit = this.units.find((u) => u.id === id && !u.deleted_at);
    if (!unit) throw new Error(`Unidade '${id}' não encontrada.`);
    unit.deleted_at = new Date().toISOString();
    unit.active = false;
  }

  async getStockBalance(
    warehouseId: string,
    equipmentModelId: string,
  ): Promise<StockBalance> {
    let bal = this.balances.find(
      (b) =>
        b.warehouse_id === warehouseId &&
        b.equipment_model_id === equipmentModelId,
    );
    if (!bal) {
      bal = {
        id: `sb_${Date.now()}`,
        tenant_id: "tenant_default",
        warehouse_id: warehouseId,
        equipment_model_id: equipmentModelId,
        quantity_on_hand: 0,
        quantity_allocated: 0,
        quantity_maintenance: 0,
        updated_at: new Date().toISOString(),
      };
      this.balances.push(bal);
    }
    return { ...bal };
  }

  async listStockBalances(
    equipmentModelId?: string,
    warehouseId?: string,
  ): Promise<StockBalance[]> {
    let list = this.balances;
    if (equipmentModelId) {
      list = list.filter((b) => b.equipment_model_id === equipmentModelId);
    }
    if (warehouseId) {
      list = list.filter((b) => b.warehouse_id === warehouseId);
    }
    return list.map((b) => ({ ...b }));
  }

  async recordStockMovement(
    input: RecordStockMovementInput,
  ): Promise<StockMovement> {
    const now = new Date().toISOString();
    const mov: StockMovement = {
      id: `mov_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
      tenant_id: "tenant_default",
      equipment_model_id: input.equipment_model_id,
      equipment_unit_id: input.equipment_unit_id ?? null,
      from_warehouse_id: input.from_warehouse_id ?? null,
      to_warehouse_id: input.to_warehouse_id ?? null,
      movement_type: input.movement_type,
      quantity: input.quantity,
      reason: input.reason,
      reference_id: input.reference_id ?? null,
      performed_by_user_id: "usr_mock",
      created_at: now,
    };
    this.movements.push(mov);

    if (input.from_warehouse_id) {
      let fromBal = this.balances.find(
        (b) =>
          b.warehouse_id === input.from_warehouse_id &&
          b.equipment_model_id === input.equipment_model_id,
      );
      if (!fromBal) {
        fromBal = {
          id: `sb_${Date.now()}_${Math.random()}`,
          tenant_id: "tenant_default",
          warehouse_id: input.from_warehouse_id,
          equipment_model_id: input.equipment_model_id,
          quantity_on_hand: 0,
          quantity_allocated: 0,
          quantity_maintenance: 0,
          updated_at: now,
        };
        this.balances.push(fromBal);
      }
      fromBal.quantity_on_hand = Math.max(
        0,
        fromBal.quantity_on_hand - input.quantity,
      );
      fromBal.updated_at = now;
    }
    if (input.to_warehouse_id) {
      let toBal = this.balances.find(
        (b) =>
          b.warehouse_id === input.to_warehouse_id &&
          b.equipment_model_id === input.equipment_model_id,
      );
      if (!toBal) {
        toBal = {
          id: `sb_${Date.now()}_${Math.random()}`,
          tenant_id: "tenant_default",
          warehouse_id: input.to_warehouse_id,
          equipment_model_id: input.equipment_model_id,
          quantity_on_hand: 0,
          quantity_allocated: 0,
          quantity_maintenance: 0,
          updated_at: now,
        };
        this.balances.push(toBal);
      }
      toBal.quantity_on_hand += input.quantity;
      toBal.updated_at = now;
    }

    return { ...mov };
  }

  async listStockMovements(
    filter?: StockMovementFilter,
  ): Promise<StockMovement[]> {
    let list = this.movements;
    if (filter?.equipment_model_id) {
      list = list.filter(
        (m) => m.equipment_model_id === filter.equipment_model_id,
      );
    }
    if (filter?.equipment_unit_id) {
      list = list.filter(
        (m) => m.equipment_unit_id === filter.equipment_unit_id,
      );
    }
    if (filter?.warehouse_id) {
      list = list.filter(
        (m) =>
          m.from_warehouse_id === filter.warehouse_id ||
          m.to_warehouse_id === filter.warehouse_id,
      );
    }
    if (filter?.movement_type) {
      list = list.filter((m) => m.movement_type === filter.movement_type);
    }
    return list.map((m) => ({ ...m }));
  }

  async attachUnitMedia(
    input: AttachUnitMediaInput,
  ): Promise<EquipmentUnitMedia> {
    const now = new Date().toISOString();
    const media: EquipmentUnitMedia = {
      id: `eum_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
      tenant_id: "tenant_default",
      equipment_unit_id: input.equipment_unit_id,
      blob_id: input.blob_id,
      media_type: input.media_type,
      caption: input.caption ?? null,
      created_at: now,
      created_by: "usr_mock",
    };
    this.unitMedia.push(media);
    return { ...media };
  }

  async detachUnitMedia(id: string): Promise<void> {
    const idx = this.unitMedia.findIndex((m) => m.id === id);
    if (idx !== -1) {
      this.unitMedia.splice(idx, 1);
    }
  }
}
