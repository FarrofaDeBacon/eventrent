<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { MapPin, Plus, Search, ChevronRight, Layers } from "lucide-svelte";
  import {
    appSession,
    getVenueRepository,
    hasPermission,
    PERMISSIONS,
    type VenueListItem,
  } from "$lib/data";
  import VirtualList from "$lib/components/virtual-list.svelte";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import { EmptyState, ErrorState, LoadingState } from "$lib/ui";

  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.VENUE_MANAGE),
  );

  let items = $state<VenueListItem[]>([]);
  let isLoading = $state(true);
  let errorMessage = $state<string | null>(null);
  let searchQuery = $state("");
  let totalCount = $state(0);

  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  async function loadVenues(queryText = searchQuery) {
    isLoading = true;
    errorMessage = null;
    try {
      const page = await getVenueRepository().list({
        query: queryText.trim() || undefined,
        limit: 100,
      });
      items = page.items;
      totalCount = page.total_count ?? page.items.length;
    } catch (err) {
      errorMessage =
        err instanceof Error ? err.message : "Erro ao carregar locais.";
    } finally {
      isLoading = false;
    }
  }

  function handleSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    searchQuery = val;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      loadVenues(val);
    }, 200);
  }

  onMount(() => {
    loadVenues();
  });
</script>

<div class="mx-auto max-w-6xl space-y-6 p-6">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div>
      <h1 class="flex items-center gap-2 text-2xl font-bold tracking-tight">
        <MapPin class="size-6 text-primary" />
        Locais de Evento
      </h1>
      <p class="text-sm text-muted-foreground">
        {totalCount} local(is) cadastrado(s) com espaços e ficha técnica.
      </p>
    </div>

    {#if canManage}
      <Button href="/venues/new">
        <Plus class="mr-2 size-4" />
        Novo local
      </Button>
    {/if}
  </div>

  <div class="relative max-w-md">
    <Search
      class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
    />
    <Input
      type="search"
      placeholder="Buscar por nome, cidade, espaço ou contato..."
      value={searchQuery}
      oninput={handleSearchInput}
      class="pl-9"
    />
  </div>

  {#if isLoading}
    <LoadingState message="Carregando locais de evento..." />
  {:else if errorMessage}
    <ErrorState
      title="Falha ao carregar locais"
      message={errorMessage}
      onRetry={() => loadVenues()}
    />
  {:else if items.length === 0}
    <EmptyState
      title="Nenhum local encontrado"
      description={searchQuery.trim()
        ? "Nenhum local corresponde à busca informada."
        : "Cadastre o primeiro local de evento para organizar espaços e logística."}
    />
    {#if canManage && !searchQuery.trim()}
      <div class="flex justify-center">
        <Button href="/venues/new">
          <Plus class="mr-2 size-4" />
          Cadastrar primeiro local
        </Button>
      </div>
    {/if}
  {:else}
    <div class="overflow-hidden rounded-xl border border-border bg-card">
      <VirtualList
        {items}
        itemHeight={72}
        height={520}
        getKey={(item) => item.id}
      >
        {#snippet children(venue)}
          <button
            type="button"
            onclick={() => goto(`/venues/${venue.id}`)}
            class="flex h-full w-full items-center justify-between gap-4 border-b border-border px-5 text-left transition-colors hover:bg-muted/40"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="truncate font-semibold text-foreground">
                  {venue.name}
                </span>
                {#if !venue.active}
                  <Badge variant="secondary">Inativo</Badge>
                {/if}
              </div>
              <p class="truncate text-xs text-muted-foreground">
                {[venue.neighborhood, venue.city, venue.state]
                  .filter(Boolean)
                  .join(" • ") || "Endereço não informado"}
              </p>
            </div>

            <div class="flex items-center gap-3">
              <Badge variant="outline" class="gap-1">
                <Layers class="size-3" />
                {venue.spaces_count}
                {venue.spaces_count === 1 ? "espaço" : "espaços"}
              </Badge>
              <ChevronRight class="size-4 text-muted-foreground" />
            </div>
          </button>
        {/snippet}
      </VirtualList>
    </div>
  {/if}
</div>
