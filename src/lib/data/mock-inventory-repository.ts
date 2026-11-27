import type { InventoryRepository } from "./inventory-repository";
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

export class MockInventoryRepository implements InventoryRepository {
  public warehouses: Warehouse[] = [
    {
      id: "wh_mock_01",
      tenant_id: "tenant_default",
      name: "Depósito principal",
      code: "PRINCIPAL",
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
    const wh = this.warehouses.find((w) => w.id === id && !w.deleted_at);
    if (!wh) throw new Error(`Depósito '${id}' não encontrado.`);
    for (const w of this.warehouses) {
      w.is_default = w.id === id;
    }
  }

  async deleteWarehouse(id: string): Promise<void> {
    const idx = this.warehouses.findIndex((w) => w.id === id && !w.deleted_at);
    if (idx === -1) throw new Error(`Depósito '${id}' não encontrado.`);
    this.warehouses[idx].deleted_at = new Date().toISOString();
    this.warehouses[idx].active = false;
  }

  async listEquipmentUnits(
    filter?: EquipmentUnitFilter,
  ): Promise<EquipmentUnitListPage> {
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

    const items: EquipmentUnitDetails[] = list.map((unit) => ({
      unit: { ...unit },
      model_name: "Modelo Mock",
      warehouse_name:
        this.warehouses.find((w) => w.id === unit.warehouse_id)?.name ??
        "Depósito Mock",
      model_thumbnail_url: null,
    }));

    return {
      items,
      next_cursor: null,
      has_more: false,
    };
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
      model_thumbnail_url: null,
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
      acquisition_cost_cents: input.acquisition_cost_cents ?? null,
      supplier_id: input.supplier_id ?? null,
      invoice_number: input.invoice_number ?? null,
      origin: input.origin ?? "own",
      return_by: input.return_by ?? null,
      usage_hours: input.usage_hours ?? 0,
      location_note: input.location_note ?? null,
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
    if (input.acquisition_cost_cents !== undefined)
      unit.acquisition_cost_cents = input.acquisition_cost_cents;
    if (input.supplier_id !== undefined) unit.supplier_id = input.supplier_id;
    if (input.invoice_number !== undefined)
      unit.invoice_number = input.invoice_number;
    if (input.origin !== undefined) unit.origin = input.origin;
    if (input.return_by !== undefined) unit.return_by = input.return_by;
    if (input.usage_hours !== undefined) unit.usage_hours = input.usage_hours;
    if (input.location_note !== undefined)
      unit.location_note = input.location_note;
    if (input.active !== undefined) unit.active = input.active;
    unit.updated_at = new Date().toISOString();

    return { ...unit };
  }

  async decommissionEquipmentUnit(
    id: string,
    input: DecommissionEquipmentUnitInput,
  ): Promise<EquipmentUnit> {
    const unit = this.units.find((u) => u.id === id && !u.deleted_at);
    if (!unit) throw new Error(`Unidade '${id}' não encontrada.`);
    if (
      input.expected_updated_at &&
      unit.updated_at !== input.expected_updated_at
    ) {
      throw new Error("Conflito de versão ao baixar unidade.");
    }
    unit.status = "decommissioned";
    unit.active = false;
    unit.notes = input.reason;
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
        quantity_maintenance: 0,
        min_quantity: 0,
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
          quantity_maintenance: 0,
          min_quantity: 0,
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
          quantity_maintenance: 0,
          min_quantity: 0,
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
}
