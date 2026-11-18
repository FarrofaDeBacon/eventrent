<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { toast } from "svelte-sonner";
  import { MapPin, ArrowLeft, Power, Trash2 } from "lucide-svelte";
  import VenueForm from "$lib/components/venue-form.svelte";
  import {
    appSession,
    getVenueRepository,
    hasPermission,
    PERMISSIONS,
    type CreateVenueInput,
    type VenueWithDetails,
  } from "$lib/data";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import { ErrorState, LoadingState } from "$lib/ui";

  let venueId = $derived(page.params.id ?? "");
  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.VENUE_MANAGE),
  );

  let venue = $state<VenueWithDetails | null>(null);
  let isLoading = $state(true);
  let isSaving = $state(false);
  let errorMessage = $state<string | null>(null);

  async function loadVenue() {
    if (!venueId) return;
    isLoading = true;
    errorMessage = null;
    try {
      venue = await getVenueRepository().getById(venueId);
    } catch (err) {
      errorMessage =
        err instanceof Error ? err.message : "Local de evento não encontrado.";
    } finally {
      isLoading = false;
    }
  }

  async function handleSubmit(payload: CreateVenueInput) {
    if (!venue) return;
    isSaving = true;
    try {
      venue = await getVenueRepository().update({
        id: venue.id,
        expected_updated_at: venue.updated_at,
        ...payload,
      });
      toast.success("Local atualizado com sucesso.");
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Erro ao atualizar local.",
      );
    } finally {
      isSaving = false;
    }
  }

  async function handleToggleActive() {
    if (!venue || !canManage) return;
    try {
      venue = await getVenueRepository().setActive(venue.id, !venue.active);
      toast.success(venue.active ? "Local ativado." : "Local desativado.");
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Erro ao alterar status.",
      );
    }
  }

  async function handleDelete() {
    if (!venue || !canManage) return;
    try {
      await getVenueRepository().delete(venue.id);
      toast.success("Local excluído com sucesso.");
      goto("/venues");
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Erro ao excluir local.",
      );
    }
  }

  onMount(() => {
    loadVenue();
  });
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
  {#if isLoading}
    <LoadingState message="Carregando local de evento..." />
  {:else if errorMessage || !venue}
    <ErrorState
      title="Local não encontrado"
      message={errorMessage ?? "Não foi possível carregar o local."}
      onRetry={loadVenue}
    />
  {:else}
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" href="/venues" aria-label="Voltar">
          <ArrowLeft class="size-4" />
        </Button>
        <div>
          <div class="flex items-center gap-2">
            <h1
              class="flex items-center gap-2 text-2xl font-bold tracking-tight"
            >
              <MapPin class="size-6 text-primary" />
              {venue.name}
            </h1>
            <Badge variant={venue.active ? "default" : "secondary"}>
              {venue.active ? "Ativo" : "Inativo"}
            </Badge>
          </div>
          <p class="text-sm text-muted-foreground">
            {[venue.city, venue.state].filter(Boolean).join(" / ") ||
              "Sem cidade informada"}
          </p>
        </div>
      </div>

      {#if canManage}
        <div class="flex items-center gap-2">
          <Button variant="outline" size="sm" onclick={handleToggleActive}>
            <Power class="mr-2 size-4" />
            {venue.active ? "Desativar" : "Ativar"}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onclick={handleDelete}
            class="text-destructive hover:text-destructive"
          >
            <Trash2 class="mr-2 size-4" />
            Excluir
          </Button>
        </div>
      {/if}
    </div>

    <VenueForm
      initialVenue={venue}
      readOnly={!canManage}
      {isSaving}
      onSubmit={handleSubmit}
      onCancel={() => goto("/venues")}
    />
  {/if}
</div>
