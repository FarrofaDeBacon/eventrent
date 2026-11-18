<script lang="ts">
  import { goto } from "$app/navigation";
  import { toast } from "svelte-sonner";
  import { MapPin, ArrowLeft } from "lucide-svelte";
  import VenueForm from "$lib/components/venue-form.svelte";
  import { getVenueRepository, type CreateVenueInput } from "$lib/data";
  import { Button } from "$lib/ui/button";

  let isSaving = $state(false);

  async function handleSubmit(payload: CreateVenueInput) {
    isSaving = true;
    try {
      const created = await getVenueRepository().create(payload);
      toast.success(`Local "${created.name}" cadastrado com sucesso.`);
      goto(`/venues/${created.id}`);
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Erro ao cadastrar local.",
      );
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
  <div class="flex items-center gap-3">
    <Button variant="ghost" size="icon" href="/venues" aria-label="Voltar">
      <ArrowLeft class="size-4" />
    </Button>
    <div>
      <h1 class="flex items-center gap-2 text-2xl font-bold tracking-tight">
        <MapPin class="size-6 text-primary" />
        Novo Local de Evento
      </h1>
      <p class="text-sm text-muted-foreground">
        Preencha os dados do local, espaços internos e contatos operacionais.
      </p>
    </div>
  </div>

  <VenueForm
    {isSaving}
    onSubmit={handleSubmit}
    onCancel={() => goto("/venues")}
  />
</div>
