import { describe, it, expect, beforeEach } from "vitest";
import { MockInventoryRepository } from "./mock-inventory-repository";

describe("InventoryRepository (Mock Data Layer)", () => {
  let repo: MockInventoryRepository;

  beforeEach(() => {
    repo = new MockInventoryRepository();
  });

  describe("Depósitos (Warehouses)", () => {
    it("deve listar depósitos contendo o Galpão Principal como padrão", async () => {
      const warehouses = await repo.listWarehouses();
      expect(warehouses.length).toBe(1);
      expect(warehouses[0].name).toBe("Galpão Principal");
      expect(warehouses[0].is_default).toBe(true);
    });

    it("deve criar um segundo depósito e permitir torná-lo o padrão", async () => {
      const created = await repo.createWarehouse({
        name: "Depósito Secundário",
        code: "DEP-02",
        city: "Campinas",
        state: "SP",
      });

      expect(created.id).toBeDefined();
      expect(created.code).toBe("DEP-02");
      expect(created.is_default).toBe(false);

      // Define como padrão
      await repo.setDefaultWarehouse(created.id);

      const updatedCreated = await repo.getWarehouse(created.id);
      expect(updatedCreated.is_default).toBe(true);

      const originalWh = await repo.getWarehouse("wh_mock_01");
      expect(originalWh.is_default).toBe(false);
    });

    it("deve atualizar e remover logicamente um depósito", async () => {
      const created = await repo.createWarehouse({
        name: "Depósito Temporário",
        code: "DEP-TMP",
      });

      await repo.updateWarehouse(created.id, {
        name: "Depósito Atualizado",
        code: "DEP-ATU",
      });

      const updated = await repo.getWarehouse(created.id);
      expect(updated.name).toBe("Depósito Atualizado");

      await repo.deleteWarehouse(created.id);

      const activeList = await repo.listWarehouses(false);
      expect(activeList.some((w) => w.id === created.id)).toBe(false);
    });
  });

  describe("Unidades Físicas (Equipment Units)", () => {
    it("deve cadastrar unidade física serializada gerando tag patrimonial sequencial EQ000001", async () => {
      const unit = await repo.createEquipmentUnit({
        equipment_model_id: "mod_cdj_3000",
        serial_number: "CDJ-SN-9988",
        notes: "Com case rígido",
      });

      expect(unit.patrimony_tag).toBe("EQ000001");
      expect(unit.status).toBe("available");
      expect(unit.physical_condition).toBe("good");

      // Uma movimentação de compra deve ter sido registrada automaticamente
      const movements = await repo.listStockMovements({
        equipment_unit_id: unit.id,
      });
      expect(movements.length).toBe(1);
      expect(movements[0].movement_type).toBe("purchase_in");

      // Saldo do modelo no depósito padrão deve ser 1
      const balance = await repo.getStockBalance(
        unit.warehouse_id,
        "mod_cdj_3000",
      );
      expect(balance.quantity_on_hand).toBe(1);
    });

    it("deve atualizar status e notas de uma unidade física", async () => {
      const unit = await repo.createEquipmentUnit({
        equipment_model_id: "mod_cdj_3000",
        serial_number: "CDJ-SN-1122",
      });

      const updated = await repo.updateEquipmentUnit(unit.id, {
        status: "maintenance",
        physical_condition: "fair",
        notes: "Botão Cue com mau contato",
      });

      expect(updated.status).toBe("maintenance");
      expect(updated.physical_condition).toBe("fair");

      const fetched = await repo.getEquipmentUnit(unit.id);
      expect(fetched.unit.status).toBe("maintenance");
      expect(fetched.unit.notes).toBe("Botão Cue com mau contato");
    });

    it("deve filtrar unidades por busca de patrimônio ou série", async () => {
      await repo.createEquipmentUnit({
        equipment_model_id: "mod_cdj_3000",
        serial_number: "PIONEER-ALPHA",
      });
      await repo.createEquipmentUnit({
        equipment_model_id: "mod_cdj_3000",
        serial_number: "PIONEER-BETA",
      });

      const searchAlpha = await repo.listEquipmentUnits({
        search: "alpha",
      });
      expect(searchAlpha.length).toBe(1);
      expect(searchAlpha[0].unit.serial_number).toBe("PIONEER-ALPHA");

      const searchBeta = await repo.listEquipmentUnits({
        search: "EQ000002",
      });
      expect(searchBeta.length).toBe(1);
    });
  });

  describe("Mídias de Unidade Física", () => {
    it("deve anexar e desanexar foto de vistoria/avaria em uma unidade", async () => {
      const unit = await repo.createEquipmentUnit({
        equipment_model_id: "mod_speaker",
      });

      const media = await repo.attachUnitMedia({
        equipment_unit_id: unit.id,
        blob_id: "blob_foto_avaria_123",
        media_type: "damage_photo",
        caption: "Arranhão na grade frontal",
      });

      expect(media.id).toBeDefined();
      expect(media.media_type).toBe("damage_photo");

      const unitDetails = await repo.getEquipmentUnit(unit.id);
      expect(unitDetails.media.length).toBe(1);
      expect(unitDetails.media[0].caption).toBe("Arranhão na grade frontal");

      await repo.detachUnitMedia(media.id);

      const unitDetailsAfter = await repo.getEquipmentUnit(unit.id);
      expect(unitDetailsAfter.media.length).toBe(0);
    });
  });

  describe("Saldos e Livro-Razão (Stock & Balances)", () => {
    it("deve atualizar saldos corretamente com movimentações de compra e transferência", async () => {
      const wh2 = await repo.createWarehouse({
        name: "Filial Sul",
        code: "FIL-SUL",
      });

      // 1. Compra de 10 unidades para o galpão principal
      await repo.recordStockMovement({
        equipment_model_id: "mod_cabo_xlr",
        to_warehouse_id: "wh_mock_01",
        movement_type: "purchase_in",
        quantity: 10,
        reason: "Compra em lote",
      });

      const balPrincipal = await repo.getStockBalance(
        "wh_mock_01",
        "mod_cabo_xlr",
      );
      expect(balPrincipal.quantity_on_hand).toBe(10);

      // 2. Transferência de 4 unidades para a Filial Sul
      await repo.recordStockMovement({
        equipment_model_id: "mod_cabo_xlr",
        from_warehouse_id: "wh_mock_01",
        to_warehouse_id: wh2.id,
        movement_type: "transfer_out",
        quantity: 4,
        reason: "Remanejamento entre unidades",
      });

      const balPrincipalAfter = await repo.getStockBalance(
        "wh_mock_01",
        "mod_cabo_xlr",
      );
      const balSul = await repo.getStockBalance(wh2.id, "mod_cabo_xlr");

      expect(balPrincipalAfter.quantity_on_hand).toBe(6);
      expect(balSul.quantity_on_hand).toBe(4);
    });
  });
});
