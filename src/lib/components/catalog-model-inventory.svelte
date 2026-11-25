<script lang="ts">
  import {
    Boxes,
    Camera,
    Plus,
    RefreshCw,
    Trash2,
    Wrench,
    ArrowRightLeft,
  } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    getInventoryRepository,
    type EquipmentUnitDetails,
    type PhysicalCondition,
    type StockBalance,
    type StockMovementType,
    type UnitMediaType,
    type UnitStatus,
    type Warehouse,
  } from "$lib/data";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import { Dialog } from "$lib/ui/dialog";
  import { Input } from "$lib/ui/input";

  interface Props {
    modelId: string;
    modelName: string;
    controlMode: "serialized" | "quantity";
    readonly?: boolean;
  }

  let { modelId, modelName, controlMode, readonly = false }: Props = $props();

  const repo = getInventoryRepository();

  let loading = $state(true);
  let error = $state<string | null>(null);

  let units = $state<EquipmentUnitDetails[]>([]);
  let warehouses = $state<Warehouse[]>([]);
  let balances = $state<StockBalance[]>([]);

  // Modais
  let createUnitModalOpen = $state(false);
  let editUnitModalOpen = $state(false);
  let selectedUnit = $state<EquipmentUnitDetails | null>(null);

  let movementModalOpen = $state(false);
  let mediaModalOpen = $state(false);

  // Form Nova Unidade
  let newUnitWarehouseId = $state("");
  let newUnitSerialNumber = $state("");
  let newUnitCondition = $state<PhysicalCondition>("good");
  let newUnitNotes = $state("");
  let newUnitAcquisitionDate = $state("");
  let submittingNewUnit = $state(false);

  // Form Edição de Unidade
  let editUnitWarehouseId = $state("");
  let editUnitStatus = $state<UnitStatus>("available");
  let editUnitCondition = $state<PhysicalCondition>("good");
  let editUnitNotes = $state("");
  let submittingEditUnit = $state(false);

  // Form Movimentação Quantitativa
  let movType = $state<StockMovementType>("purchase_in");
  let movWarehouseId = $state("");
  let movQuantity = $state(1);
  let movReason = $state("");
  let movRefId = $state("");
  let submittingMovement = $state(false);

  // Form Mídia / Foto
  let newMediaBlobId = $state("");
  let newMediaType = $state<UnitMediaType>("damage_photo");
  let newMediaCaption = $state("");
  let submittingMedia = $state(false);

  async function loadData() {
    loading = true;
    error = null;
    try {
      warehouses = await repo.listWarehouses(false);
      if (controlMode === "serialized") {
        units = await repo.listEquipmentUnits({ equipment_model_id: modelId });
      } else {
        balances = await repo.listStockBalances(modelId);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void modelId;
    void controlMode;
    void loadData();
  });

  // Ações de Unidade Física
  async function handleCreateUnit() {
    if (!modelId) return;
    submittingNewUnit = true;
    try {
      await repo.createEquipmentUnit({
        equipment_model_id: modelId,
        warehouse_id: newUnitWarehouseId || undefined,
        serial_number: newUnitSerialNumber || undefined,
        physical_condition: newUnitCondition,
        notes: newUnitNotes || undefined,
        acquisition_date: newUnitAcquisitionDate || undefined,
      });
      toast.success("Unidade física cadastrada e tombada com sucesso!");
      createUnitModalOpen = false;
      newUnitSerialNumber = "";
      newUnitNotes = "";
      newUnitAcquisitionDate = "";
      await loadData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingNewUnit = false;
    }
  }

  function openEditModal(item: EquipmentUnitDetails) {
    selectedUnit = item;
    editUnitWarehouseId = item.unit.warehouse_id;
    editUnitStatus = item.unit.status;
    editUnitCondition = item.unit.physical_condition;
    editUnitNotes = item.unit.notes ?? "";
    editUnitModalOpen = true;
  }

  async function handleSaveEditUnit() {
    if (!selectedUnit) return;
    submittingEditUnit = true;
    try {
      await repo.updateEquipmentUnit(selectedUnit.unit.id, {
        warehouse_id: editUnitWarehouseId,
        status: editUnitStatus,
        physical_condition: editUnitCondition,
        notes: editUnitNotes || undefined,
      });
      toast.success("Unidade física atualizada!");
      editUnitModalOpen = false;
      await loadData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingEditUnit = false;
    }
  }

  async function handleDeleteUnit(unitId: string, tag: string) {
    if (!confirm(`Deseja realmente dar baixa definitiva na unidade ${tag}?`)) {
      return;
    }
    try {
      await repo.deleteEquipmentUnit(unitId);
      toast.success(`Unidade ${tag} baixada do estoque.`);
      await loadData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  // Ações de Movimentação Quantitativa
  async function handleRecordMovement() {
    if (!modelId || !movReason.trim()) {
      toast.error("Informe o motivo da movimentação.");
      return;
    }
    submittingMovement = true;
    try {
      await repo.recordStockMovement({
        equipment_model_id: modelId,
        from_warehouse_id:
          movType === "transfer_out" || movType === "rental_out"
            ? movWarehouseId
            : undefined,
        to_warehouse_id:
          movType === "purchase_in" ||
          movType === "return_in" ||
          movType === "adjustment"
            ? movWarehouseId
            : undefined,
        movement_type: movType,
        quantity: Math.max(1, Number(movQuantity)),
        reason: movReason.trim(),
        reference_id: movRefId.trim() || undefined,
      });
      toast.success("Movimentação registrada com sucesso!");
      movementModalOpen = false;
      movReason = "";
      movRefId = "";
      await loadData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingMovement = false;
    }
  }

  // Mídias
  function openMediaModal(item: EquipmentUnitDetails) {
    selectedUnit = item;
    newMediaBlobId = "";
    newMediaCaption = "";
    mediaModalOpen = true;
  }

  async function handleAttachMedia() {
    if (!selectedUnit || !newMediaBlobId.trim()) {
      toast.error("Informe o identificador do arquivo de mídia.");
      return;
    }
    submittingMedia = true;
    try {
      await repo.attachUnitMedia({
        equipment_unit_id: selectedUnit.unit.id,
        blob_id: newMediaBlobId.trim(),
        media_type: newMediaType,
        caption: newMediaCaption.trim() || undefined,
      });
      toast.success("Foto/documento anexado à unidade!");
      mediaModalOpen = false;
      await loadData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingMedia = false;
    }
  }

  async function handleDetachMedia(mediaId: string) {
    try {
      await repo.detachUnitMedia(mediaId);
      toast.success("Mídia desanexada.");
      await loadData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  function getStatusBadge(status: UnitStatus) {
    switch (status) {
      case "available":
        return { label: "Disponível", variant: "default" as const };
      case "rented":
        return { label: "Alugado", variant: "secondary" as const };
      case "maintenance":
        return { label: "Manutenção", variant: "outline" as const };
      case "damaged":
        return { label: "Avariado", variant: "destructive" as const };
      case "lost":
        return { label: "Extraviado", variant: "destructive" as const };
    }
  }

  function getConditionLabel(cond: PhysicalCondition) {
    switch (cond) {
      case "new":
        return "Novo";
      case "good":
        return "Bom";
      case "fair":
        return "Regular";
      case "poor":
        return "Ruim";
    }
  }
</script>

<div class="space-y-6">
  <!-- Cabeçalho da Aba -->
  <div
    class="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-border bg-card p-4 shadow-xs"
  >
    <div class="flex items-center gap-3">
      <div
        class="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary"
      >
        <Boxes class="h-5 w-5" />
      </div>
      <div>
        <h2 class="text-base font-semibold text-foreground">
          Gestão de Estoque — {modelName}
        </h2>
        <p class="text-xs text-muted-foreground">
          Controle de {controlMode === "serialized"
            ? "unidades físicas tombadas (patrimônio / nº de série)"
            : "saldo quantitativo agregado por depósito"}
        </p>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <Button
        variant="outline"
        size="sm"
        onclick={loadData}
        aria-label="Atualizar dados de estoque"
      >
        <RefreshCw class="mr-1 h-3.5 w-3.5" />
        Atualizar
      </Button>

      {#if !readonly}
        {#if controlMode === "serialized"}
          <Button
            size="sm"
            onclick={() => {
              createUnitModalOpen = true;
            }}
            data-testid="btn-add-unit"
          >
            <Plus class="mr-1 h-3.5 w-3.5" />
            Nova Unidade Física
          </Button>
        {:else}
          <Button
            size="sm"
            onclick={() => {
              movementModalOpen = true;
            }}
            data-testid="btn-record-movement"
          >
            <ArrowRightLeft class="mr-1 h-3.5 w-3.5" />
            Lançar Movimentação
          </Button>
        {/if}
      {/if}
    </div>
  </div>

  {#if loading}
    <div
      class="rounded-xl border border-border bg-card p-10 text-center text-sm text-muted-foreground"
    >
      Carregando informações de estoque...
    </div>
  {:else if error}
    <div
      class="rounded-xl border border-destructive/40 bg-destructive/10 p-6 text-sm text-destructive"
    >
      {error}
    </div>
  {:else if controlMode === "serialized"}
    <!-- Visão de Unidades Físicas Serializadas -->
    <div
      class="rounded-xl border border-border bg-card shadow-xs overflow-hidden"
    >
      {#if units.length === 0}
        <div class="p-10 text-center">
          <Boxes class="mx-auto h-8 w-8 text-muted-foreground mb-2" />
          <p class="text-sm font-medium text-foreground">
            Nenhuma unidade física tombada para este modelo
          </p>
          <p class="text-xs text-muted-foreground mt-1">
            Clique em "Nova Unidade Física" para registrar o primeiro número de
            série e gerar a etiqueta de patrimônio.
          </p>
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-left text-sm">
            <thead
              class="border-b border-border bg-muted/40 text-xs font-medium text-muted-foreground"
            >
              <tr>
                <th class="px-4 py-3">Patrimônio</th>
                <th class="px-4 py-3">Nº de Série</th>
                <th class="px-4 py-3">Depósito</th>
                <th class="px-4 py-3">Status</th>
                <th class="px-4 py-3">Condição</th>
                <th class="px-4 py-3">Vistoria / Fotos</th>
                <th class="px-4 py-3 text-right">Ações</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              {#each units as item (item.unit.id)}
                {@const badge = getStatusBadge(item.unit.status)}
                <tr class="hover:bg-muted/30 transition-colors">
                  <td class="px-4 py-3 font-mono font-semibold text-foreground">
                    {item.unit.patrimony_tag}
                  </td>
                  <td class="px-4 py-3 text-foreground">
                    {item.unit.serial_number || "—"}
                  </td>
                  <td class="px-4 py-3 text-muted-foreground">
                    {item.warehouse_name}
                  </td>
                  <td class="px-4 py-3">
                    <Badge variant={badge.variant}>{badge.label}</Badge>
                  </td>
                  <td class="px-4 py-3 text-muted-foreground">
                    {getConditionLabel(item.unit.physical_condition)}
                  </td>
                  <td class="px-4 py-3">
                    <button
                      type="button"
                      class="inline-flex items-center gap-1.5 text-xs text-primary hover:underline cursor-pointer"
                      onclick={() => openMediaModal(item)}
                    >
                      <Camera class="h-3.5 w-3.5" />
                      {item.media.length} foto(s)
                    </button>
                  </td>
                  <td class="px-4 py-3 text-right">
                    <div class="inline-flex items-center gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onclick={() => openEditModal(item)}
                        disabled={readonly}
                        aria-label={`Editar unidade ${item.unit.patrimony_tag}`}
                      >
                        <Wrench class="h-3.5 w-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        class="text-destructive hover:bg-destructive/10"
                        onclick={() =>
                          handleDeleteUnit(
                            item.unit.id,
                            item.unit.patrimony_tag,
                          )}
                        disabled={readonly}
                        aria-label={`Excluir unidade ${item.unit.patrimony_tag}`}
                      >
                        <Trash2 class="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {:else}
    <!-- Visão Quantitativa (Agregada por Depósito) -->
    <div
      class="rounded-xl border border-border bg-card shadow-xs overflow-hidden"
    >
      {#if balances.length === 0}
        <div class="p-10 text-center">
          <Boxes class="mx-auto h-8 w-8 text-muted-foreground mb-2" />
          <p class="text-sm font-medium text-foreground">
            Nenhum saldo registrado para este modelo
          </p>
          <p class="text-xs text-muted-foreground mt-1">
            Lance uma movimentação de entrada/compra para provisionar o estoque.
          </p>
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-left text-sm">
            <thead
              class="border-b border-border bg-muted/40 text-xs font-medium text-muted-foreground"
            >
              <tr>
                <th class="px-4 py-3">Depósito</th>
                <th class="px-4 py-3 text-right">Disponível (Em Estoque)</th>
                <th class="px-4 py-3 text-right">Alocado em Eventos</th>
                <th class="px-4 py-3 text-right">Em Manutenção</th>
                <th class="px-4 py-3 text-right">Total Físico</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              {#each balances as bal (bal.id)}
                {@const wh = warehouses.find((w) => w.id === bal.warehouse_id)}
                <tr class="hover:bg-muted/30 transition-colors">
                  <td class="px-4 py-3 font-medium text-foreground">
                    {wh?.name ?? bal.warehouse_id}
                    {#if wh?.is_default}
                      <Badge variant="outline" class="ml-2 text-xs"
                        >Padrão</Badge
                      >
                    {/if}
                  </td>
                  <td
                    class="px-4 py-3 text-right font-mono font-semibold text-foreground"
                  >
                    {bal.quantity_on_hand}
                  </td>
                  <td
                    class="px-4 py-3 text-right font-mono text-muted-foreground"
                  >
                    {bal.quantity_allocated}
                  </td>
                  <td
                    class="px-4 py-3 text-right font-mono text-muted-foreground"
                  >
                    {bal.quantity_maintenance}
                  </td>
                  <td
                    class="px-4 py-3 text-right font-mono font-bold text-foreground"
                  >
                    {bal.quantity_on_hand +
                      bal.quantity_allocated +
                      bal.quantity_maintenance}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Modal: Criar Unidade Física -->
<Dialog
  bind:open={createUnitModalOpen}
  title="Cadastrar Nova Unidade Física"
  description={`Tombamento de novo equipamento serializado para '${modelName}'.`}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void handleCreateUnit();
    }}
    class="space-y-4"
  >
    <div>
      <label
        for="new-unit-warehouse"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Depósito de Entrada *
      </label>
      <select
        id="new-unit-warehouse"
        bind:value={newUnitWarehouseId}
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <option value="">Depósito Padrão</option>
        {#each warehouses as w (w.id)}
          <option value={w.id}>
            {w.name} ({w.code}){w.is_default ? " — Padrão" : ""}
          </option>
        {/each}
      </select>
    </div>

    <div>
      <label
        for="new-unit-serial"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Número de Série do Fabricante
      </label>
      <Input
        id="new-unit-serial"
        placeholder="Ex: CDJ3K-2026-9900"
        bind:value={newUnitSerialNumber}
      />
    </div>

    <div>
      <label
        for="new-unit-condition"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Condição Física Inicial
      </label>
      <select
        id="new-unit-condition"
        bind:value={newUnitCondition}
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <option value="new">Novo (Lacrado)</option>
        <option value="good">Bom (Operacional)</option>
        <option value="fair">Regular (Marcas de uso)</option>
        <option value="poor">Ruim (Avariado)</option>
      </select>
    </div>

    <div>
      <label
        for="new-unit-date"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Data de Aquisição
      </label>
      <Input
        id="new-unit-date"
        type="date"
        bind:value={newUnitAcquisitionDate}
      />
    </div>

    <div>
      <label
        for="new-unit-notes"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Observações / Acessórios
      </label>
      <Input
        id="new-unit-notes"
        placeholder="Ex: Acompanha case rígido e cabo de força"
        bind:value={newUnitNotes}
      />
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button
        type="button"
        variant="ghost"
        onclick={() => {
          createUnitModalOpen = false;
        }}
      >
        Cancelar
      </Button>
      <Button type="submit" disabled={submittingNewUnit}>
        {submittingNewUnit ? "Tombando..." : "Cadastrar e Gerar Patrimônio"}
      </Button>
    </div>
  </form>
</Dialog>

<!-- Modal: Editar Unidade Física -->
<Dialog
  bind:open={editUnitModalOpen}
  title={`Editar Unidade ${selectedUnit?.unit.patrimony_tag ?? ""}`}
  description="Atualização de status operacional, depósito e vistorias."
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void handleSaveEditUnit();
    }}
    class="space-y-4"
  >
    <div>
      <label
        for="edit-unit-warehouse"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Localização / Depósito
      </label>
      <select
        id="edit-unit-warehouse"
        bind:value={editUnitWarehouseId}
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        {#each warehouses as w (w.id)}
          <option value={w.id}>
            {w.name} ({w.code})
          </option>
        {/each}
      </select>
    </div>

    <div>
      <label
        for="edit-unit-status"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Status Operacional
      </label>
      <select
        id="edit-unit-status"
        bind:value={editUnitStatus}
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <option value="available">Disponível para Locação</option>
        <option value="rented">Alugado em Evento</option>
        <option value="maintenance">Em Manutenção Técnica</option>
        <option value="damaged">Avariado (Indisponível)</option>
        <option value="lost">Extraviado / Perda</option>
      </select>
    </div>

    <div>
      <label
        for="edit-unit-condition"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Condição Física
      </label>
      <select
        id="edit-unit-condition"
        bind:value={editUnitCondition}
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <option value="new">Novo</option>
        <option value="good">Bom</option>
        <option value="fair">Regular</option>
        <option value="poor">Ruim</option>
      </select>
    </div>

    <div>
      <label
        for="edit-unit-notes"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Observações
      </label>
      <Input id="edit-unit-notes" bind:value={editUnitNotes} />
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button
        type="button"
        variant="ghost"
        onclick={() => {
          editUnitModalOpen = false;
        }}
      >
        Cancelar
      </Button>
      <Button type="submit" disabled={submittingEditUnit}>
        {submittingEditUnit ? "Salvando..." : "Salvar Alterações"}
      </Button>
    </div>
  </form>
</Dialog>

<!-- Modal: Lançar Movimentação Quantitativa -->
<Dialog
  bind:open={movementModalOpen}
  title="Movimentação de Estoque"
  description={`Lançamento no livro-razão para '${modelName}'.`}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void handleRecordMovement();
    }}
    class="space-y-4"
  >
    <div>
      <label
        for="mov-type"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Tipo de Movimentação *
      </label>
      <select
        id="mov-type"
        bind:value={movType}
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <option value="purchase_in">Entrada por Compra (Aumenta Estoque)</option
        >
        <option value="return_in">Retorno / Devolução (Aumenta Estoque)</option>
        <option value="transfer_out">Transferência entre Depósitos</option>
        <option value="adjustment">Ajuste de Inventário / Contagem</option>
        <option value="maintenance_out"
          >Envio para Manutenção (Reduz Estoque)</option
        >
        <option value="loss_out">Baixa por Perda/Avaria (Reduz Estoque)</option>
      </select>
    </div>

    <div>
      <label
        for="mov-wh"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Depósito Envolvido *
      </label>
      <select
        id="mov-wh"
        bind:value={movWarehouseId}
        required
        class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
      >
        {#each warehouses as w (w.id)}
          <option value={w.id}>{w.name} ({w.code})</option>
        {/each}
      </select>
    </div>

    <div>
      <label
        for="mov-qty"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Quantidade *
      </label>
      <Input
        id="mov-qty"
        type="number"
        min="1"
        bind:value={movQuantity}
        required
      />
    </div>

    <div>
      <label
        for="mov-reason"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Motivo / Justificativa *
      </label>
      <Input
        id="mov-reason"
        placeholder="Ex: Nota Fiscal 1234 / Contagem periódica"
        bind:value={movReason}
        required
      />
    </div>

    <div>
      <label
        for="mov-ref"
        class="block text-xs font-medium text-muted-foreground mb-1"
      >
        Nº de Documento / Referência (Opcional)
      </label>
      <Input id="mov-ref" placeholder="Ex: NF-009988" bind:value={movRefId} />
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button
        type="button"
        variant="ghost"
        onclick={() => {
          movementModalOpen = false;
        }}
      >
        Cancelar
      </Button>
      <Button type="submit" disabled={submittingMovement}>
        {submittingMovement ? "Registrando..." : "Confirmar Movimentação"}
      </Button>
    </div>
  </form>
</Dialog>

<!-- Modal: Fotos de Vistoria da Unidade -->
<Dialog
  bind:open={mediaModalOpen}
  title={`Vistorias e Avarias — ${selectedUnit?.unit.patrimony_tag ?? ""}`}
  description="Histórico de fotos e registros visuais desta unidade física."
>
  <div class="space-y-4">
    {#if selectedUnit && selectedUnit.media.length > 0}
      <div class="space-y-2">
        {#each selectedUnit.media as m (m.id)}
          <div
            class="flex items-center justify-between rounded-lg border border-border bg-muted/20 p-2 text-xs"
          >
            <div>
              <span class="font-semibold text-foreground">
                {m.media_type === "damage_photo"
                  ? "Avaria"
                  : m.media_type === "inspection_photo"
                    ? "Vistoria"
                    : "Serial"}
              </span>
              {#if m.caption}
                <p class="text-muted-foreground">{m.caption}</p>
              {/if}
            </div>
            {#if !readonly}
              <Button
                variant="ghost"
                size="sm"
                class="text-destructive hover:bg-destructive/10"
                onclick={() => handleDetachMedia(m.id)}
              >
                <Trash2 class="h-3.5 w-3.5" />
              </Button>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-xs text-muted-foreground italic">
        Nenhuma foto de vistoria anexada a esta unidade.
      </p>
    {/if}

    {#if !readonly}
      <div class="border-t border-border pt-4 space-y-3">
        <h4 class="text-xs font-semibold text-foreground">Anexar Nova Foto</h4>
        <div>
          <label
            for="media-blob-id"
            class="block text-xs font-medium text-muted-foreground mb-1"
          >
            ID do Arquivo de Mídia *
          </label>
          <Input
            id="media-blob-id"
            placeholder="Identificador do blob de foto"
            bind:value={newMediaBlobId}
          />
        </div>
        <div>
          <label
            for="media-type"
            class="block text-xs font-medium text-muted-foreground mb-1"
          >
            Tipo de Registro
          </label>
          <select
            id="media-type"
            bind:value={newMediaType}
            class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          >
            <option value="damage_photo">Foto de Avaria</option>
            <option value="inspection_photo"
              >Foto de Vistoria Pré/Pós Evento</option
            >
            <option value="serial_photo">Foto da Etiqueta de Série</option>
          </select>
        </div>
        <div>
          <label
            for="media-caption"
            class="block text-xs font-medium text-muted-foreground mb-1"
          >
            Descrição / Local do Dano
          </label>
          <Input
            id="media-caption"
            placeholder="Ex: Risco na lateral esquerda do chassi"
            bind:value={newMediaCaption}
          />
        </div>
        <div class="flex justify-end gap-2">
          <Button
            type="button"
            size="sm"
            onclick={handleAttachMedia}
            disabled={submittingMedia}
          >
            {submittingMedia ? "Anexando..." : "Anexar Foto"}
          </Button>
        </div>
      </div>
    {/if}
  </div>
</Dialog>
