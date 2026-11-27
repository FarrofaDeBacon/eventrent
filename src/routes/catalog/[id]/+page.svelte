<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import {
    ArrowLeft,
    Boxes,
    FileText,
    Image as ImageIcon,
    Power,
    Sliders,
    Trash2,
  } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import CatalogModelForm from "$lib/components/catalog-model-form.svelte";
  import CatalogModelInventory from "$lib/components/catalog-model-inventory.svelte";
  import CatalogModelMedia from "$lib/components/catalog-model-media.svelte";
  import {
    appSession,
    getCatalogRepository,
    hasPermission,
    PERMISSIONS,
    type CreateEquipmentModelInput,
    type EquipmentModelWithDetails,
  } from "$lib/data";
  import { Button } from "$lib/ui/button";

  const repo = getCatalogRepository();

  let modelId = $derived(page.params.id ?? "");
  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.CATALOG_MANAGE),
  );

  let details = $state<EquipmentModelWithDetails | null>(null);
  let loading = $state(true);
  let submitting = $state(false);
  let loadError = $state<string | null>(null);
  let activeTab = $state<"details" | "photos" | "documents" | "inventory">(
    "details",
  );

  async function loadModel() {
    if (!modelId) return;
    loading = true;
    loadError = null;
    try {
      details = await repo.getEquipmentModel(modelId);
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void modelId;
    void loadModel();
  });

  async function handleUpdate(payload: CreateEquipmentModelInput) {
    if (!details || !canManage) return;
    submitting = true;
    try {
      details = await repo.updateEquipmentModel(details.model.id, {
        ...payload,
        expected_updated_at: details.model.updated_at,
      });
      toast.success("Modelo atualizado com sucesso!");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submitting = false;
    }
  }

  async function handleToggleActive() {
    if (!details || !canManage) return;
    const next = !details.model.active;
    try {
      await repo.setEquipmentModelActive(details.model.id, next);
      await loadModel();
      toast.success(next ? "Modelo ativado." : "Modelo desativado.");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleDelete() {
    if (!details || !canManage) return;
    try {
      await repo.deleteEquipmentModel(details.model.id);
      toast.success("Modelo excluído do catálogo.");
      goto("/catalog");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  function handleTabKeydown(e: KeyboardEvent) {
    const order: Array<"details" | "photos" | "documents" | "inventory"> = [
      "details",
      "photos",
      "documents",
      "inventory",
    ];
    const idx = order.indexOf(activeTab);
    if (e.key === "ArrowRight") {
      e.preventDefault();
      activeTab = order[(idx + 1) % order.length];
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      activeTab = order[(idx - 1 + order.length) % order.length];
    }
  }
</script>

<div class="mx-auto max-w-4xl space-y-6">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <Button
        variant="ghost"
        size="icon"
        href="/catalog"
        aria-label="Voltar para catálogo"
      >
        <ArrowLeft class="h-4 w-4" />
      </Button>
      <div>
        <h1 class="text-2xl font-semibold tracking-tight text-foreground">
          {details ? details.model.name : "Detalhes do Modelo"}
        </h1>
        {#if details}
          <p class="text-sm text-muted-foreground">
            {details.category_name}{details.brand_name
              ? ` • ${details.brand_name}`
              : ""}
          </p>
        {/if}
      </div>
    </div>

    {#if details && canManage}
      <div class="flex items-center gap-2">
        <Button variant="outline" onclick={handleToggleActive}>
          <Power class="mr-2 h-4 w-4" />
          {details.model.active ? "Desativar" : "Ativar"}
        </Button>
        <Button variant="destructive" onclick={handleDelete}>
          <Trash2 class="mr-2 h-4 w-4" />
          Excluir
        </Button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div
      class="rounded-xl border border-border bg-card p-10 text-center text-sm text-muted-foreground"
    >
      Carregando modelo...
    </div>
  {:else if loadError}
    <div
      class="rounded-xl border border-destructive/40 bg-destructive/10 p-6 text-sm text-destructive"
    >
      {loadError}
    </div>
  {:else if details}
    <div
      role="tablist"
      aria-label="Seções da ficha do modelo"
      tabindex="0"
      onkeydown={handleTabKeydown}
      class="flex items-center gap-1 rounded-xl border border-border bg-card p-1 shadow-xs"
    >
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "details"}
        data-testid="tab-model-details"
        onclick={() => {
          activeTab = "details";
        }}
        class="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors {activeTab ===
        'details'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
      >
        <Sliders class="h-4 w-4" />
        Ficha Técnica
      </button>

      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "photos"}
        data-testid="tab-model-photos"
        onclick={() => {
          activeTab = "photos";
        }}
        class="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors {activeTab ===
        'photos'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
      >
        <ImageIcon class="h-4 w-4" />
        Fotos
      </button>

      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "documents"}
        data-testid="tab-model-documents"
        onclick={() => {
          activeTab = "documents";
        }}
        class="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors {activeTab ===
        'documents'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
      >
        <FileText class="h-4 w-4" />
        Documentos
      </button>

      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "inventory"}
        data-testid="tab-model-inventory"
        onclick={() => {
          activeTab = "inventory";
        }}
        class="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors {activeTab ===
        'inventory'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
      >
        <Boxes class="h-4 w-4" />
        Unidades & Estoque
      </button>
    </div>

    {#if activeTab === "details"}
      <CatalogModelForm
        initial={details}
        readonly={!canManage}
        {submitting}
        onSubmit={handleUpdate}
        onCancel={() => goto("/catalog")}
      />
    {:else if activeTab === "photos"}
      <CatalogModelMedia
        modelId={details.model.id}
        modelName={details.model.name}
        mode="photos"
        readonly={!canManage}
      />
    {:else if activeTab === "documents"}
      <CatalogModelMedia
        modelId={details.model.id}
        modelName={details.model.name}
        mode="documents"
        readonly={!canManage}
      />
    {:else}
      <CatalogModelInventory
        modelId={details.model.id}
        modelName={details.model.name}
        controlMode={details.model.control_mode}
        readonly={!canManage}
      />
    {/if}
  {/if}
</div>
