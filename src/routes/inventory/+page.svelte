<script lang="ts">
  import { onMount } from "svelte";
  import {
    ArrowRightLeft,
    Boxes,
    Building2,
    History,
    Pencil,
    Plus,
    Search,
    Star,
    Wrench,
  } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    appSession,
    getCatalogRepository,
    getInventoryRepository,
    hasPermission,
    PERMISSIONS,
    type CreateWarehouseInput,
    type EquipmentUnitDetails,
    type PhysicalCondition,
    type RecordStockMovementInput,
    type StockMovement,
    type StockMovementType,
    type UnitStatus,
    type UpdateWarehouseInput,
    type Warehouse,
  } from "$lib/data";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import { Dialog } from "$lib/ui/dialog";
  import { Input } from "$lib/ui/input";

  const repo = getInventoryRepository();
  const catalogRepo = getCatalogRepository();

  let canOperate = $derived(
    hasPermission(appSession.session, PERMISSIONS.WAREHOUSE_OPERATE),
  );

  let activeTab = $state<"warehouses" | "units" | "movements">("warehouses");
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Dados
  let warehouses = $state<Warehouse[]>([]);
  let units = $state<EquipmentUnitDetails[]>([]);
  let movements = $state<StockMovement[]>([]);
  let modelNames = $state<Record<string, string>>({});
  let modelList = $state<Array<{ id: string; name: string }>>([]);

  // Filtros de Unidades
  let unitSearch = $state("");
  let unitWarehouseFilter = $state("");
  let unitStatusFilter = $state("");

  // Filtros de Movimentações
  let movementTypeFilter = $state("");
  let movementWarehouseFilter = $state("");

  // Modais de Depósito
  let createWarehouseOpen = $state(false);
  let editWarehouseOpen = $state(false);
  let selectedWarehouse = $state<Warehouse | null>(null);

  let newWarehouseName = $state("");
  let newWarehouseCode = $state("");
  let newWarehouseDescription = $state("");
  let newWarehouseAddress = $state("");
  let newWarehouseCity = $state("");
  let newWarehouseState = $state("");
  let newWarehousePostalCode = $state("");
  let newWarehouseIsDefault = $state(false);
  let submittingWarehouse = $state(false);

  let editWarehouseName = $state("");
  let editWarehouseCode = $state("");
  let editWarehouseDescription = $state("");
  let editWarehouseAddress = $state("");
  let editWarehouseCity = $state("");
  let editWarehouseState = $state("");
  let editWarehousePostalCode = $state("");
  let editWarehouseActive = $state(true);

  // Modal Edição de Unidade
  let editUnitOpen = $state(false);
  let selectedUnit = $state<EquipmentUnitDetails | null>(null);
  let editUnitWarehouseId = $state("");
  let editUnitStatus = $state<UnitStatus>("available");
  let editUnitCondition = $state<PhysicalCondition>("good");
  let editUnitNotes = $state("");
  let submittingEditUnit = $state(false);

  // Modal de Nova Movimentação
  let newMovementOpen = $state(false);
  let movModelId = $state("");
  let movType = $state<StockMovementType>("purchase_in");
  let movFromWarehouseId = $state("");
  let movToWarehouseId = $state("");
  let movQuantity = $state(1);
  let movReason = $state("");
  let movRefId = $state("");
  let submittingMovement = $state(false);

  // Funções de carregamento
  async function loadAllData() {
    loading = true;
    error = null;
    try {
      const [whList, uList, mvList, catRes] = await Promise.all([
        repo.listWarehouses(true),
        repo.listEquipmentUnits(),
        repo.listStockMovements({ limit: 200 }),
        catalogRepo
          .listEquipmentModels({ limit: 500 })
          .catch(() => ({ items: [] })),
      ]);

      warehouses = whList;
      units = uList.items;
      movements = mvList;

      const map: Record<string, string> = {};
      const list: Array<{ id: string; name: string }> = [];
      for (const item of catRes.items) {
        map[item.model.id] = item.model.name;
        list.push({ id: item.model.id, name: item.model.name });
      }
      modelNames = map;
      modelList = list;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  // Ações de Depósito
  function openCreateWarehouse() {
    newWarehouseName = "";
    newWarehouseCode = "";
    newWarehouseDescription = "";
    newWarehouseAddress = "";
    newWarehouseCity = "";
    newWarehouseState = "";
    newWarehousePostalCode = "";
    newWarehouseIsDefault = warehouses.length === 0;
    createWarehouseOpen = true;
  }

  async function handleCreateWarehouse() {
    if (!newWarehouseName.trim() || !newWarehouseCode.trim()) {
      toast.error("Nome e código do depósito são obrigatórios.");
      return;
    }
    submittingWarehouse = true;
    try {
      const input: CreateWarehouseInput = {
        name: newWarehouseName.trim(),
        code: newWarehouseCode.trim().toUpperCase(),
        description: newWarehouseDescription.trim() || undefined,
        address_line1: newWarehouseAddress.trim() || undefined,
        city: newWarehouseCity.trim() || undefined,
        state: newWarehouseState.trim().toUpperCase() || undefined,
        postal_code: newWarehousePostalCode.trim() || undefined,
        is_default: newWarehouseIsDefault,
      };
      await repo.createWarehouse(input);
      toast.success("Depósito criado com sucesso!");
      createWarehouseOpen = false;
      await loadAllData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingWarehouse = false;
    }
  }

  function openEditWarehouse(wh: Warehouse) {
    selectedWarehouse = wh;
    editWarehouseName = wh.name;
    editWarehouseCode = wh.code;
    editWarehouseDescription = wh.description || "";
    editWarehouseAddress = wh.address_line1 || "";
    editWarehouseCity = wh.city || "";
    editWarehouseState = wh.state || "";
    editWarehousePostalCode = wh.postal_code || "";
    editWarehouseActive = wh.active;
    editWarehouseOpen = true;
  }

  async function handleUpdateWarehouse() {
    if (!selectedWarehouse) return;
    if (!editWarehouseName.trim() || !editWarehouseCode.trim()) {
      toast.error("Nome e código do depósito são obrigatórios.");
      return;
    }
    submittingWarehouse = true;
    try {
      const input: UpdateWarehouseInput = {
        expected_updated_at: selectedWarehouse.updated_at,
        name: editWarehouseName.trim(),
        code: editWarehouseCode.trim().toUpperCase(),
        description: editWarehouseDescription.trim() || undefined,
        address_line1: editWarehouseAddress.trim() || undefined,
        city: editWarehouseCity.trim() || undefined,
        state: editWarehouseState.trim().toUpperCase() || undefined,
        postal_code: editWarehousePostalCode.trim() || undefined,
        active: editWarehouseActive,
      };
      await repo.updateWarehouse(selectedWarehouse.id, input);
      toast.success("Depósito atualizado com sucesso!");
      editWarehouseOpen = false;
      await loadAllData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingWarehouse = false;
    }
  }

  async function handleSetDefaultWarehouse(wh: Warehouse) {
    try {
      await repo.setDefaultWarehouse(wh.id);
      toast.success(`Depósito "${wh.name}" definido como padrão!`);
      await loadAllData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  // Ações de Unidade
  function openEditUnit(u: EquipmentUnitDetails) {
    selectedUnit = u;
    editUnitWarehouseId = u.unit.warehouse_id;
    editUnitStatus = u.unit.status;
    editUnitCondition = u.unit.physical_condition;
    editUnitNotes = u.unit.notes || "";
    editUnitOpen = true;
  }

  async function handleUpdateUnit() {
    if (!selectedUnit) return;
    submittingEditUnit = true;
    try {
      await repo.updateEquipmentUnit(selectedUnit.unit.id, {
        expected_updated_at: selectedUnit.unit.updated_at,
        warehouse_id: editUnitWarehouseId,
        status: editUnitStatus,
        physical_condition: editUnitCondition,
        notes: editUnitNotes.trim() || undefined,
      });
      toast.success(`Unidade ${selectedUnit.unit.patrimony_tag} atualizada!`);
      editUnitOpen = false;
      await loadAllData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingEditUnit = false;
    }
  }

  // Ações de Movimentação
  function openNewMovement() {
    movModelId = modelList.length > 0 ? modelList[0].id : "";
    movType = "purchase_in";
    const defaultWh = warehouses.find((w) => w.is_default) || warehouses[0];
    movToWarehouseId = defaultWh ? defaultWh.id : "";
    movFromWarehouseId = "";
    movQuantity = 1;
    movReason = "";
    movRefId = "";
    newMovementOpen = true;
  }

  async function handleCreateMovement() {
    if (!movModelId) {
      toast.error("Selecione um modelo de equipamento.");
      return;
    }
    if (!movReason.trim()) {
      toast.error("Informe a justificativa/motivo da movimentação.");
      return;
    }
    if (movQuantity < 1) {
      toast.error("Quantidade deve ser no mínimo 1.");
      return;
    }

    submittingMovement = true;
    try {
      const input: RecordStockMovementInput = {
        equipment_model_id: movModelId,
        movement_type: movType,
        quantity: movQuantity,
        reason: movReason.trim(),
        reference_id: movRefId.trim() || undefined,
        from_warehouse_id:
          movType === "transfer_out" ||
          movType === "loss_out" ||
          movType === "decommission_out"
            ? movFromWarehouseId || undefined
            : undefined,
        to_warehouse_id:
          movType === "purchase_in" ||
          movType === "return_in" ||
          movType === "transfer_in" ||
          movType === "adjustment"
            ? movToWarehouseId || undefined
            : undefined,
      };

      await repo.recordStockMovement(input);
      toast.success("Movimentação registrada com sucesso no livro-razão!");
      newMovementOpen = false;
      await loadAllData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submittingMovement = false;
    }
  }

  // Formatação e Helpers
  function getStatusBadge(status: UnitStatus): {
    label: string;
    variant: "default" | "secondary" | "destructive" | "outline";
  } {
    switch (status) {
      case "available":
        return { label: "Disponível", variant: "default" };
      case "maintenance":
        return { label: "Em Manutenção", variant: "destructive" };
      case "damaged":
        return { label: "Avariado", variant: "destructive" };
      case "lost":
        return { label: "Extraviado / Perdido", variant: "destructive" };
      case "decommissioned":
        return { label: "Baixado", variant: "secondary" };
      default:
        return { label: status, variant: "secondary" };
    }
  }

  function getConditionLabel(cond: PhysicalCondition): string {
    switch (cond) {
      case "new":
        return "Novo";
      case "good":
        return "Bom";
      case "fair":
        return "Regular";
      case "poor":
        return "Ruim / Desgastado";
      default:
        return cond;
    }
  }

  function getMovementTypeLabel(type: StockMovementType): {
    label: string;
    variant: "default" | "secondary" | "destructive" | "outline";
  } {
    switch (type) {
      case "purchase_in":
        return { label: "Entrada (Compra)", variant: "default" };
      case "return_in":
        return { label: "Retorno (Devolução)", variant: "default" };
      case "transfer_out":
        return { label: "Transferência (Saída)", variant: "outline" };
      case "transfer_in":
        return { label: "Transferência (Entrada)", variant: "outline" };
      case "status_change":
        return { label: "Mudança de Status", variant: "secondary" };
      case "adjustment":
        return { label: "Ajuste de Balanço", variant: "secondary" };
      case "loss_out":
        return { label: "Baixa / Perda", variant: "destructive" };
      case "decommission_out":
        return { label: "Baixa Definitiva", variant: "destructive" };
      default:
        return { label: type, variant: "secondary" };
    }
  }

  // Filtragem Reativa
  let filteredUnits = $derived(
    units.filter((u) => {
      if (unitSearch.trim()) {
        const q = unitSearch.trim().toLowerCase();
        const matchesTag = u.unit.patrimony_tag.toLowerCase().includes(q);
        const matchesSerial = (u.unit.serial_number || "")
          .toLowerCase()
          .includes(q);
        const matchesModel = u.model_name.toLowerCase().includes(q);
        if (!matchesTag && !matchesSerial && !matchesModel) return false;
      }
      if (unitWarehouseFilter && u.unit.warehouse_id !== unitWarehouseFilter) {
        return false;
      }
      if (unitStatusFilter && u.unit.status !== unitStatusFilter) {
        return false;
      }
      return true;
    }),
  );

  let filteredMovements = $derived(
    movements.filter((m) => {
      if (movementTypeFilter && m.movement_type !== movementTypeFilter) {
        return false;
      }
      if (movementWarehouseFilter) {
        if (
          m.from_warehouse_id !== movementWarehouseFilter &&
          m.to_warehouse_id !== movementWarehouseFilter
        ) {
          return false;
        }
      }
      return true;
    }),
  );

  onMount(() => {
    loadAllData();
  });
</script>

<div class="mx-auto max-w-7xl space-y-6 p-6">
  <!-- Cabeçalho -->
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div>
      <h1
        class="flex items-center gap-2 text-2xl font-bold tracking-tight text-foreground"
      >
        <Boxes class="h-6 w-6 text-primary" />
        Gestão de Estoque & Depósitos
      </h1>
      <p class="text-sm text-muted-foreground">
        Controle multidepósito, tombamento de patrimônio e livro-razão de
        movimentações físicas e quantitativas.
      </p>
    </div>

    <div class="flex items-center gap-2">
      {#if activeTab === "warehouses" && canOperate}
        <Button onclick={openCreateWarehouse} data-testid="btn-new-warehouse">
          <Plus class="mr-2 h-4 w-4" />
          Novo Depósito
        </Button>
      {:else if activeTab === "movements" && canOperate}
        <Button onclick={openNewMovement} data-testid="btn-new-movement">
          <ArrowRightLeft class="mr-2 h-4 w-4" />
          Registrar Movimentação
        </Button>
      {/if}
    </div>
  </div>

  <!-- Abas de Navegação -->
  <div
    class="flex border-b border-border space-x-2"
    role="tablist"
    aria-label="Abas de Gestão de Estoque"
  >
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "warehouses"}
      data-testid="tab-warehouses"
      onclick={() => {
        activeTab = "warehouses";
      }}
      class="inline-flex items-center gap-2 rounded-t-lg px-4 py-2.5 text-sm font-medium transition-colors border-b-2 {activeTab ===
      'warehouses'
        ? 'border-primary text-primary font-semibold'
        : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'}"
    >
      <Building2 class="h-4 w-4" />
      Depósitos ({warehouses.length})
    </button>

    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "units"}
      data-testid="tab-units"
      onclick={() => {
        activeTab = "units";
      }}
      class="inline-flex items-center gap-2 rounded-t-lg px-4 py-2.5 text-sm font-medium transition-colors border-b-2 {activeTab ===
      'units'
        ? 'border-primary text-primary font-semibold'
        : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'}"
    >
      <Boxes class="h-4 w-4" />
      Unidades Físicas ({units.length})
    </button>

    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "movements"}
      data-testid="tab-movements"
      onclick={() => {
        activeTab = "movements";
      }}
      class="inline-flex items-center gap-2 rounded-t-lg px-4 py-2.5 text-sm font-medium transition-colors border-b-2 {activeTab ===
      'movements'
        ? 'border-primary text-primary font-semibold'
        : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'}"
    >
      <History class="h-4 w-4" />
      Livro-Razão & Movimentações ({movements.length})
    </button>
  </div>

  {#if loading}
    <div
      class="rounded-xl border border-border bg-card p-12 text-center text-sm text-muted-foreground"
    >
      Carregando estoque e depósitos...
    </div>
  {:else if error}
    <div
      role="alert"
      class="rounded-xl border border-destructive/40 bg-destructive/10 p-6 text-sm text-destructive"
    >
      {error}
    </div>
  {:else if activeTab === "warehouses"}
    <!-- ========================================== -->
    <!-- TAB 1: DEPÓSITOS -->
    <!-- ========================================== -->
    <div
      class="rounded-xl border border-border bg-card shadow-xs overflow-hidden"
    >
      {#if warehouses.length === 0}
        <div class="p-12 text-center">
          <Building2 class="mx-auto h-8 w-8 text-muted-foreground mb-2" />
          <p class="text-sm font-medium text-foreground">
            Nenhum depósito cadastrado
          </p>
          {#if canOperate}
            <Button class="mt-4" onclick={openCreateWarehouse}>
              <Plus class="mr-2 h-4 w-4" />
              Cadastrar Primeiro Depósito
            </Button>
          {/if}
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table
            class="w-full text-left text-sm"
            data-testid="table-warehouses"
          >
            <thead
              class="border-b border-border bg-muted/40 text-xs font-medium text-muted-foreground"
            >
              <tr>
                <th class="px-4 py-3">Código</th>
                <th class="px-4 py-3">Nome do Depósito</th>
                <th class="px-4 py-3">Endereço / Localização</th>
                <th class="px-4 py-3">Status</th>
                <th class="px-4 py-3 text-right">Ações</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              {#each warehouses as wh (wh.id)}
                <tr class="hover:bg-muted/30 transition-colors">
                  <td class="px-4 py-3 font-mono font-semibold text-foreground">
                    {wh.code}
                  </td>
                  <td class="px-4 py-3">
                    <div
                      class="font-medium text-foreground flex items-center gap-2"
                    >
                      {wh.name}
                      {#if wh.is_default}
                        <Badge variant="default" class="gap-1 text-xs">
                          <Star class="h-3 w-3 fill-current" />
                          Padrão
                        </Badge>
                      {/if}
                    </div>
                    {#if wh.description}
                      <p class="text-xs text-muted-foreground">
                        {wh.description}
                      </p>
                    {/if}
                  </td>
                  <td class="px-4 py-3 text-muted-foreground">
                    {#if wh.city || wh.state}
                      <span
                        >{wh.city || ""}{wh.city && wh.state
                          ? " / "
                          : ""}{wh.state || ""}</span
                      >
                    {:else if wh.address_line1}
                      <span>{wh.address_line1}</span>
                    {:else}
                      <span class="text-muted-foreground/60">—</span>
                    {/if}
                  </td>
                  <td class="px-4 py-3">
                    {#if wh.active}
                      <Badge
                        variant="outline"
                        class="text-foreground border-border">Ativo</Badge
                      >
                    {:else}
                      <Badge variant="destructive">Inativo</Badge>
                    {/if}
                  </td>
                  <td class="px-4 py-3 text-right">
                    <div class="inline-flex items-center gap-1">
                      {#if !wh.is_default && canOperate && wh.active}
                        <Button
                          variant="ghost"
                          size="sm"
                          onclick={() => handleSetDefaultWarehouse(wh)}
                          title="Definir como galpão padrão"
                          aria-label={`Definir ${wh.name} como padrão`}
                        >
                          <Star
                            class="h-3.5 w-3.5 text-muted-foreground hover:text-foreground"
                          />
                        </Button>
                      {/if}
                      {#if canOperate}
                        <Button
                          variant="ghost"
                          size="sm"
                          onclick={() => openEditWarehouse(wh)}
                          title="Editar depósito"
                          aria-label={`Editar depósito ${wh.name}`}
                        >
                          <Pencil class="h-3.5 w-3.5" />
                        </Button>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {:else if activeTab === "units"}
    <!-- ========================================== -->
    <!-- TAB 2: UNIDADES FÍSICAS -->
    <!-- ========================================== -->
    <div class="space-y-4">
      <!-- Filtros -->
      <div
        class="flex flex-wrap items-center gap-3 rounded-xl border border-border bg-card p-4"
      >
        <div class="relative flex-1 min-w-[200px]">
          <Search
            class="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground"
          />
          <Input
            placeholder="Buscar por patrimônio, número de série ou modelo..."
            class="pl-9"
            bind:value={unitSearch}
          />
        </div>

        <select
          bind:value={unitWarehouseFilter}
          aria-label="Filtrar por depósito"
          class="rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="">Todos os Depósitos</option>
          {#each warehouses as wh (wh.id)}
            <option value={wh.id}>{wh.name} ({wh.code})</option>
          {/each}
        </select>

        <select
          bind:value={unitStatusFilter}
          aria-label="Filtrar por status"
          class="rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="">Todos os Status</option>
          <option value="available">Disponível</option>
          <option value="rented">Alugado / Em Evento</option>
          <option value="maintenance">Em Manutenção</option>
          <option value="damaged">Avariado</option>
          <option value="lost">Extraviado / Perdido</option>
        </select>
      </div>

      <!-- Tabela de Unidades Físicas -->
      <div
        class="rounded-xl border border-border bg-card shadow-xs overflow-hidden"
      >
        {#if filteredUnits.length === 0}
          <div class="p-12 text-center">
            <Boxes class="mx-auto h-8 w-8 text-muted-foreground mb-2" />
            <p class="text-sm font-medium text-foreground">
              Nenhuma unidade física encontrada
            </p>
            <p class="text-xs text-muted-foreground mt-1">
              Unidades físicas são tombadas na aba "Unidades & Estoque" da ficha
              de cada equipamento do catálogo.
            </p>
          </div>
        {:else}
          <div class="overflow-x-auto">
            <table class="w-full text-left text-sm" data-testid="table-units">
              <thead
                class="border-b border-border bg-muted/40 text-xs font-medium text-muted-foreground"
              >
                <tr>
                  <th class="px-4 py-3">Patrimônio</th>
                  <th class="px-4 py-3">Modelo</th>
                  <th class="px-4 py-3">Nº de Série</th>
                  <th class="px-4 py-3">Depósito</th>
                  <th class="px-4 py-3">Status</th>
                  <th class="px-4 py-3">Condição</th>
                  <th class="px-4 py-3">Origem</th>
                  <th class="px-4 py-3 text-right">Ações</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-border">
                {#each filteredUnits as item (item.unit.id)}
                  {@const badge = getStatusBadge(item.unit.status)}
                  <tr class="hover:bg-muted/30 transition-colors">
                    <td
                      class="px-4 py-3 font-mono font-semibold text-foreground"
                    >
                      {item.unit.patrimony_tag}
                    </td>
                    <td class="px-4 py-3 font-medium text-foreground">
                      {item.model_name}
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
                    <td class="px-4 py-3 text-xs text-muted-foreground">
                      {item.unit.origin === "third_party"
                        ? "Terceiro"
                        : "Próprio"}
                    </td>
                    <td class="px-4 py-3 text-right">
                      {#if canOperate}
                        <Button
                          variant="ghost"
                          size="sm"
                          onclick={() => openEditUnit(item)}
                          aria-label={`Editar unidade ${item.unit.patrimony_tag}`}
                        >
                          <Wrench class="h-3.5 w-3.5 mr-1" />
                          Alterar
                        </Button>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    </div>
  {:else if activeTab === "movements"}
    <!-- ========================================== -->
    <!-- TAB 3: LIVRO-RAZÃO E MOVIMENTAÇÕES -->
    <!-- ========================================== -->
    <div class="space-y-4">
      <!-- Filtros -->
      <div
        class="flex flex-wrap items-center gap-3 rounded-xl border border-border bg-card p-4"
      >
        <select
          bind:value={movementTypeFilter}
          aria-label="Filtrar por tipo de movimentação"
          class="rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="">Todos os Tipos de Movimentação</option>
          <option value="purchase_in">Entrada por Compra</option>
          <option value="rental_out">Saída para Evento / Locação</option>
          <option value="return_in">Retorno de Evento</option>
          <option value="transfer_out">Transferência entre Galpões</option>
          <option value="adjustment">Ajuste de Balanço / Inventário</option>
          <option value="maintenance_out">Envio para Manutenção</option>
          <option value="maintenance_in">Retorno de Manutenção</option>
          <option value="loss_out">Baixa / Perda / Extravio</option>
        </select>

        <select
          bind:value={movementWarehouseFilter}
          aria-label="Filtrar por galpão envolvido"
          class="rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="">Todos os Galpões Envolvidos</option>
          {#each warehouses as wh (wh.id)}
            <option value={wh.id}>{wh.name} ({wh.code})</option>
          {/each}
        </select>
      </div>

      <!-- Tabela do Livro-Razão -->
      <div
        class="rounded-xl border border-border bg-card shadow-xs overflow-hidden"
      >
        {#if filteredMovements.length === 0}
          <div class="p-12 text-center">
            <History class="mx-auto h-8 w-8 text-muted-foreground mb-2" />
            <p class="text-sm font-medium text-foreground">
              Nenhuma movimentação registrada
            </p>
            <p class="text-xs text-muted-foreground mt-1">
              Todas as entradas, saídas, transferências e alocações formam o
              livro-razão imutável de estoque.
            </p>
          </div>
        {:else}
          <div class="overflow-x-auto">
            <table
              class="w-full text-left text-sm"
              data-testid="table-movements"
            >
              <thead
                class="border-b border-border bg-muted/40 text-xs font-medium text-muted-foreground"
              >
                <tr>
                  <th class="px-4 py-3">Data / Hora</th>
                  <th class="px-4 py-3">Tipo</th>
                  <th class="px-4 py-3">Modelo</th>
                  <th class="px-4 py-3">Origem &rarr; Destino</th>
                  <th class="px-4 py-3 text-center">Qtd</th>
                  <th class="px-4 py-3">Motivo / Justificativa</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-border">
                {#each filteredMovements as m (m.id)}
                  {@const badge = getMovementTypeLabel(m.movement_type)}
                  {@const fromWh = warehouses.find(
                    (w) => w.id === m.from_warehouse_id,
                  )}
                  {@const toWh = warehouses.find(
                    (w) => w.id === m.to_warehouse_id,
                  )}
                  <tr class="hover:bg-muted/30 transition-colors">
                    <td
                      class="px-4 py-3 font-mono text-xs text-muted-foreground whitespace-nowrap"
                    >
                      {new Date(m.created_at).toLocaleString("pt-BR")}
                    </td>
                    <td class="px-4 py-3">
                      <Badge variant={badge.variant}>{badge.label}</Badge>
                    </td>
                    <td class="px-4 py-3 font-medium text-foreground">
                      {modelNames[m.equipment_model_id] ||
                        m.equipment_model_id.slice(0, 12)}
                    </td>
                    <td class="px-4 py-3 text-xs text-muted-foreground">
                      <span>{fromWh ? fromWh.name : "—"}</span>
                      <span class="mx-1 font-bold text-foreground">&rarr;</span>
                      <span>{toWh ? toWh.name : "—"}</span>
                    </td>
                    <td
                      class="px-4 py-3 text-center font-mono font-semibold text-foreground"
                    >
                      {m.quantity}
                    </td>
                    <td class="px-4 py-3 text-foreground">
                      {m.reason}
                      {#if m.reference_id}
                        <span
                          class="text-xs font-mono text-muted-foreground ml-1"
                        >
                          (Ref: {m.reference_id})
                        </span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- ========================================== -->
<!-- MODAL: NOVO DEPÓSITO -->
<!-- ========================================== -->
<Dialog
  bind:open={createWarehouseOpen}
  title="Cadastrar Novo Depósito"
  description="Adicione um novo galpão ou sede física para armazenar equipamentos."
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      handleCreateWarehouse();
    }}
    class="space-y-4"
  >
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="space-y-1">
        <label for="new-wh-name" class="text-xs font-medium text-foreground">
          Nome do Depósito *
        </label>
        <Input
          id="new-wh-name"
          placeholder="Ex: Galpão Central - Barra Funda"
          bind:value={newWarehouseName}
          required
        />
      </div>

      <div class="space-y-1">
        <label for="new-wh-code" class="text-xs font-medium text-foreground">
          Código / Sigla *
        </label>
        <Input
          id="new-wh-code"
          placeholder="Ex: GALP-01"
          bind:value={newWarehouseCode}
          required
        />
      </div>
    </div>

    <div class="space-y-1">
      <label for="new-wh-desc" class="text-xs font-medium text-foreground">
        Descrição / Observações
      </label>
      <Input
        id="new-wh-desc"
        placeholder="Ex: Sede principal com doca de carga"
        bind:value={newWarehouseDescription}
      />
    </div>

    <div class="space-y-1">
      <label for="new-wh-addr" class="text-xs font-medium text-foreground">
        Endereço Completo
      </label>
      <Input
        id="new-wh-addr"
        placeholder="Rua, número e bairro"
        bind:value={newWarehouseAddress}
      />
    </div>

    <div class="grid grid-cols-3 gap-3">
      <div class="space-y-1">
        <label for="new-wh-city" class="text-xs font-medium text-foreground">
          Cidade
        </label>
        <Input
          id="new-wh-city"
          placeholder="São Paulo"
          bind:value={newWarehouseCity}
        />
      </div>

      <div class="space-y-1">
        <label for="new-wh-state" class="text-xs font-medium text-foreground">
          UF
        </label>
        <Input
          id="new-wh-state"
          placeholder="SP"
          maxlength={2}
          bind:value={newWarehouseState}
        />
      </div>

      <div class="space-y-1">
        <label for="new-wh-cep" class="text-xs font-medium text-foreground">
          CEP
        </label>
        <Input
          id="new-wh-cep"
          placeholder="01139-000"
          bind:value={newWarehousePostalCode}
        />
      </div>
    </div>

    <div class="flex items-center gap-2 pt-2">
      <input
        type="checkbox"
        id="new-wh-default"
        bind:checked={newWarehouseIsDefault}
        class="h-4 w-4 rounded border-border text-primary focus:ring-primary"
      />
      <label for="new-wh-default" class="text-sm font-medium text-foreground">
        Definir como Depósito Padrão para novas alocações
      </label>
    </div>

    <div class="flex justify-end gap-2 pt-4">
      <Button
        type="button"
        variant="outline"
        onclick={() => {
          createWarehouseOpen = false;
        }}
      >
        Cancelar
      </Button>
      <Button type="submit" disabled={submittingWarehouse}>
        {submittingWarehouse ? "Salvando..." : "Criar Depósito"}
      </Button>
    </div>
  </form>
</Dialog>

<!-- ========================================== -->
<!-- MODAL: EDITAR DEPÓSITO -->
<!-- ========================================== -->
<Dialog
  bind:open={editWarehouseOpen}
  title="Editar Depósito"
  description="Atualize as informações do galpão de armazenamento."
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      handleUpdateWarehouse();
    }}
    class="space-y-4"
  >
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="space-y-1">
        <label for="edit-wh-name" class="text-xs font-medium text-foreground">
          Nome do Depósito *
        </label>
        <Input id="edit-wh-name" bind:value={editWarehouseName} required />
      </div>

      <div class="space-y-1">
        <label for="edit-wh-code" class="text-xs font-medium text-foreground">
          Código / Sigla *
        </label>
        <Input id="edit-wh-code" bind:value={editWarehouseCode} required />
      </div>
    </div>

    <div class="space-y-1">
      <label for="edit-wh-desc" class="text-xs font-medium text-foreground">
        Descrição
      </label>
      <Input id="edit-wh-desc" bind:value={editWarehouseDescription} />
    </div>

    <div class="space-y-1">
      <label for="edit-wh-addr" class="text-xs font-medium text-foreground">
        Endereço
      </label>
      <Input id="edit-wh-addr" bind:value={editWarehouseAddress} />
    </div>

    <div class="grid grid-cols-3 gap-3">
      <div class="space-y-1">
        <label for="edit-wh-city" class="text-xs font-medium text-foreground">
          Cidade
        </label>
        <Input id="edit-wh-city" bind:value={editWarehouseCity} />
      </div>

      <div class="space-y-1">
        <label for="edit-wh-state" class="text-xs font-medium text-foreground">
          UF
        </label>
        <Input
          id="edit-wh-state"
          maxlength={2}
          bind:value={editWarehouseState}
        />
      </div>

      <div class="space-y-1">
        <label for="edit-wh-cep" class="text-xs font-medium text-foreground">
          CEP
        </label>
        <Input id="edit-wh-cep" bind:value={editWarehousePostalCode} />
      </div>
    </div>

    <div class="flex items-center gap-2 pt-2">
      <input
        type="checkbox"
        id="edit-wh-active"
        bind:checked={editWarehouseActive}
        class="h-4 w-4 rounded border-border text-primary focus:ring-primary"
      />
      <label for="edit-wh-active" class="text-sm font-medium text-foreground">
        Depósito Ativo
      </label>
    </div>

    <div class="flex justify-end gap-2 pt-4">
      <Button
        type="button"
        variant="outline"
        onclick={() => {
          editWarehouseOpen = false;
        }}
      >
        Cancelar
      </Button>
      <Button type="submit" disabled={submittingWarehouse}>
        {submittingWarehouse ? "Salvando..." : "Salvar Alterações"}
      </Button>
    </div>
  </form>
</Dialog>

<!-- ========================================== -->
<!-- MODAL: EDITAR UNIDADE FÍSICA -->
<!-- ========================================== -->
<Dialog
  bind:open={editUnitOpen}
  title="Editar Unidade Física"
  description={`Atualize o status, depósito e condição física de ${selectedUnit?.unit.patrimony_tag || ""}.`}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      handleUpdateUnit();
    }}
    class="space-y-4"
  >
    <div class="space-y-1">
      <label for="edit-unit-wh" class="text-xs font-medium text-foreground">
        Depósito Atual
      </label>
      <select
        id="edit-unit-wh"
        bind:value={editUnitWarehouseId}
        class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
      >
        {#each warehouses as wh (wh.id)}
          <option value={wh.id}>{wh.name} ({wh.code})</option>
        {/each}
      </select>
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div class="space-y-1">
        <label
          for="edit-unit-status"
          class="text-xs font-medium text-foreground"
        >
          Status Operacional
        </label>
        <select
          id="edit-unit-status"
          bind:value={editUnitStatus}
          class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="available">Disponível</option>
          <option value="rented">Alugado / Em Evento</option>
          <option value="maintenance">Em Manutenção</option>
          <option value="damaged">Avariado</option>
          <option value="lost">Extraviado / Perdido</option>
        </select>
      </div>

      <div class="space-y-1">
        <label
          for="edit-unit-condition"
          class="text-xs font-medium text-foreground"
        >
          Condição Física
        </label>
        <select
          id="edit-unit-condition"
          bind:value={editUnitCondition}
          class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="new">Novo</option>
          <option value="good">Bom</option>
          <option value="fair">Regular</option>
          <option value="poor">Ruim / Desgastado</option>
        </select>
      </div>
    </div>

    <div class="space-y-1">
      <label for="edit-unit-notes" class="text-xs font-medium text-foreground">
        Observações / Histórico de Vistoria
      </label>
      <textarea
        id="edit-unit-notes"
        rows={3}
        bind:value={editUnitNotes}
        class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        placeholder="Detalhes sobre manutenção, marcas de uso ou testes realizados..."
      ></textarea>
    </div>

    <div class="flex justify-end gap-2 pt-4">
      <Button
        type="button"
        variant="outline"
        onclick={() => {
          editUnitOpen = false;
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

<!-- ========================================== -->
<!-- MODAL: NOVA MOVIMENTAÇÃO -->
<!-- ========================================== -->
<Dialog
  bind:open={newMovementOpen}
  title="Registrar Movimentação de Estoque"
  description="Lançamento imutável no livro-razão de estoque (quantitativo ou transferências)."
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      handleCreateMovement();
    }}
    class="space-y-4"
  >
    <div class="space-y-1">
      <label for="mov-model" class="text-xs font-medium text-foreground">
        Modelo de Equipamento *
      </label>
      <select
        id="mov-model"
        bind:value={movModelId}
        required
        class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
      >
        {#each modelList as m (m.id)}
          <option value={m.id}>{m.name}</option>
        {/each}
      </select>
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div class="space-y-1">
        <label for="mov-type" class="text-xs font-medium text-foreground">
          Tipo de Movimentação *
        </label>
        <select
          id="mov-type"
          bind:value={movType}
          class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option value="purchase_in">Entrada por Compra</option>
          <option value="transfer_out">Transferência entre Galpões</option>
          <option value="adjustment">Ajuste de Balanço / Inventário</option>
          <option value="loss_out">Baixa / Perda / Extravio</option>
        </select>
      </div>

      <div class="space-y-1">
        <label for="mov-qty" class="text-xs font-medium text-foreground">
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
    </div>

    {#if movType === "transfer_out"}
      <div class="grid grid-cols-2 gap-4">
        <div class="space-y-1">
          <label for="mov-from-wh" class="text-xs font-medium text-foreground">
            Galpão de Origem *
          </label>
          <select
            id="mov-from-wh"
            bind:value={movFromWarehouseId}
            required
            class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option value="">Selecione a origem</option>
            {#each warehouses as wh (wh.id)}
              <option value={wh.id}>{wh.name} ({wh.code})</option>
            {/each}
          </select>
        </div>

        <div class="space-y-1">
          <label for="mov-to-wh" class="text-xs font-medium text-foreground">
            Galpão de Destino *
          </label>
          <select
            id="mov-to-wh"
            bind:value={movToWarehouseId}
            required
            class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option value="">Selecione o destino</option>
            {#each warehouses as wh (wh.id)}
              <option value={wh.id}>{wh.name} ({wh.code})</option>
            {/each}
          </select>
        </div>
      </div>
    {:else if movType === "purchase_in" || movType === "adjustment"}
      <div class="space-y-1">
        <label for="mov-dest-wh" class="text-xs font-medium text-foreground">
          Depósito de Destino *
        </label>
        <select
          id="mov-dest-wh"
          bind:value={movToWarehouseId}
          required
          class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          {#each warehouses as wh (wh.id)}
            <option value={wh.id}>{wh.name} ({wh.code})</option>
          {/each}
        </select>
      </div>
    {:else if movType === "loss_out"}
      <div class="space-y-1">
        <label for="mov-orig-wh" class="text-xs font-medium text-foreground">
          Depósito de Origem *
        </label>
        <select
          id="mov-orig-wh"
          bind:value={movFromWarehouseId}
          required
          class="w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        >
          {#each warehouses as wh (wh.id)}
            <option value={wh.id}>{wh.name} ({wh.code})</option>
          {/each}
        </select>
      </div>
    {/if}

    <div class="space-y-1">
      <label for="mov-reason" class="text-xs font-medium text-foreground">
        Motivo / Justificativa *
      </label>
      <Input
        id="mov-reason"
        placeholder="Ex: Nota Fiscal 12345 / Ajuste contagem semestral"
        bind:value={movReason}
        required
      />
    </div>

    <div class="space-y-1">
      <label for="mov-ref" class="text-xs font-medium text-foreground">
        ID de Referência (Opcional)
      </label>
      <Input
        id="mov-ref"
        placeholder="Ex: NF-12345 ou ORD-998"
        bind:value={movRefId}
      />
    </div>

    <div class="flex justify-end gap-2 pt-4">
      <Button
        type="button"
        variant="outline"
        onclick={() => {
          newMovementOpen = false;
        }}
      >
        Cancelar
      </Button>
      <Button type="submit" disabled={submittingMovement}>
        {submittingMovement ? "Registrando..." : "Registrar no Livro-Razão"}
      </Button>
    </div>
  </form>
</Dialog>
