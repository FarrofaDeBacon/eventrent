<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { ArrowLeft, Power, Trash2 } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import CatalogModelForm from "$lib/components/catalog-model-form.svelte";
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
    <CatalogModelForm
      initial={details}
      readonly={!canManage}
      {submitting}
      onSubmit={handleUpdate}
      onCancel={() => goto("/catalog")}
    />
  {/if}
</div>
