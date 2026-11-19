<script lang="ts">
  import { goto } from "$app/navigation";
  import { ArrowLeft } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import CatalogModelForm from "$lib/components/catalog-model-form.svelte";
  import {
    getCatalogRepository,
    type CreateEquipmentModelInput,
  } from "$lib/data";
  import { Button } from "$lib/ui/button";

  const repo = getCatalogRepository();
  let submitting = $state(false);

  async function handleSubmit(payload: CreateEquipmentModelInput) {
    submitting = true;
    try {
      const created = await repo.createEquipmentModel(payload);
      toast.success(`Modelo '${created.model.name}' cadastrado com sucesso!`);
      goto(`/catalog/${created.model.id}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      submitting = false;
    }
  }
</script>

<div class="mx-auto max-w-4xl space-y-6">
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
        Novo Modelo de Equipamento
      </h1>
      <p class="text-sm text-muted-foreground">
        Cadastre especificações técnicas, dimensões, energia e valores de
        diária.
      </p>
    </div>
  </div>

  <CatalogModelForm
    {submitting}
    onSubmit={handleSubmit}
    onCancel={() => goto("/catalog")}
  />
</div>
