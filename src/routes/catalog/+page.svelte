<script lang="ts">
  import { goto } from "$app/navigation";
  import { FolderTree, Images, Plus, Search, Speaker } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import CatalogThumb from "$lib/components/catalog-thumb.svelte";
  import VirtualList from "$lib/components/virtual-list.svelte";
  import {
    appSession,
    getCatalogRepository,
    hasPermission,
    PERMISSIONS,
    type Brand,
    type ControlMode,
    type EquipmentCategoryTreeItem,
    type EquipmentModelListItem,
  } from "$lib/data";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";

  const repo = getCatalogRepository();

  let items = $state<EquipmentModelListItem[]>([]);
  let nextCursor = $state<string | null>(null);
  let totalCount = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);

  let categories = $state<EquipmentCategoryTreeItem[]>([]);
  let brands = $state<Brand[]>([]);

  let searchQuery = $state("");
  let categoryFilter = $state("");
  let brandFilter = $state("");
  let controlModeFilter = $state<"" | ControlMode>("");
  let activeFilter = $state<"all" | "active" | "inactive">("active");

  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.CATALOG_MANAGE),
  );

  function formatCents(cents: number | null): string {
    if (cents === null || cents === undefined) return "—";
    return (cents / 100).toLocaleString("pt-BR", {
      style: "currency",
      currency: "BRL",
    });
  }

  async function loadFiltersMeta() {
    try {
      const [cats, brs] = await Promise.all([
        repo.listCategoriesTree(false),
        repo.listBrands(false),
      ]);
      categories = cats;
      brands = brs;
    } catch {
      // ignora erro secundário de filtro
    }
  }

  async function fetchModels(reset: boolean) {
    if (reset) {
      loading = true;
    } else {
      if (!nextCursor || loadingMore) return;
      loadingMore = true;
    }

    try {
      const res = await repo.listEquipmentModels({
        query: searchQuery.trim() || null,
        category_id: categoryFilter || null,
        brand_id: brandFilter || null,
        control_mode: controlModeFilter === "" ? null : controlModeFilter,
        active: activeFilter === "all" ? null : activeFilter === "active",
        cursor: reset ? null : nextCursor,
        limit: 50,
      });

      if (reset) {
        items = res.items;
      } else {
        items = [...items, ...res.items];
      }
      nextCursor = res.next_cursor;
      totalCount = res.total_count;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  $effect(() => {
    void loadFiltersMeta();
  });

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    void searchQuery;
    void categoryFilter;
    void brandFilter;
    void controlModeFilter;
    void activeFilter;

    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void fetchModels(true);
    }, 150);

    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });
</script>

<div class="mx-auto max-w-6xl space-y-6">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <div
        class="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary"
      >
        <Speaker class="h-5 w-5" />
      </div>
      <div>
        <h1 class="text-2xl font-semibold tracking-tight text-foreground">
          Catálogo de Equipamentos
        </h1>
        <p class="text-sm text-muted-foreground">
          {totalCount} modelo{totalCount === 1 ? "" : "s"} encontrado{totalCount ===
          1
            ? ""
            : "s"}
        </p>
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <Button variant="outline" href="/catalog/categories">
        <FolderTree class="mr-2 h-4 w-4" />
        Categorias e Atributos
      </Button>
      {#if canManage}
        <Button variant="outline" href="/catalog/import-photos">
          <Images class="mr-2 h-4 w-4" />
          Importar fotos em lote
        </Button>
        <Button href="/catalog/new">
          <Plus class="mr-2 h-4 w-4" />
          Novo modelo
        </Button>
      {/if}
    </div>
  </div>

  <div class="rounded-xl border border-border bg-card p-4 shadow-xs">
    <div class="grid gap-3 md:grid-cols-5">
      <div class="relative md:col-span-2">
        <Search
          class="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted-foreground"
        />
        <Input
          bind:value={searchQuery}
          placeholder="Buscar por nome, SKU, código, EAN, marca..."
          aria-label="Buscar modelos no catálogo"
          class="pl-9"
        />
      </div>

      <select
        bind:value={categoryFilter}
        aria-label="Filtrar por categoria"
        class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <option value="">Todas as categorias</option>
        {#each categories as item (item.category.id)}
          <option value={item.category.id}>
            {"\u00A0".repeat((item.depth - 1) * 2)}{item.category.name}
          </option>
        {/each}
      </select>

      <select
        bind:value={brandFilter}
        aria-label="Filtrar por marca"
        class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <option value="">Todas as marcas</option>
        {#each brands as b (b.id)}
          <option value={b.id}>{b.name}</option>
        {/each}
      </select>

      <div class="grid grid-cols-2 gap-2">
        <select
          bind:value={controlModeFilter}
          aria-label="Filtrar por controle"
          class="flex h-10 w-full rounded-md border border-input bg-background px-2 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="">Controle</option>
          <option value="serialized">Serializado</option>
          <option value="quantity">Quantidade</option>
        </select>

        <select
          bind:value={activeFilter}
          aria-label="Filtrar por status"
          class="flex h-10 w-full rounded-md border border-input bg-background px-2 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="active">Ativos</option>
          <option value="inactive">Inativos</option>
          <option value="all">Todos</option>
        </select>
      </div>
    </div>
  </div>

  {#if loading}
    <div
      class="rounded-xl border border-border bg-card p-10 text-center text-sm text-muted-foreground"
    >
      Carregando catálogo...
    </div>
  {:else if items.length === 0}
    <div
      class="flex flex-col items-center justify-center rounded-xl border border-dashed border-border bg-card p-12 text-center"
    >
      <Speaker class="mb-3 h-10 w-10 text-muted-foreground/60" />
      <h2 class="text-base font-medium text-foreground">
        Nenhum modelo encontrado
      </h2>
      <p class="mt-1 max-w-md text-sm text-muted-foreground">
        Cadastre categorias técnicas (ou carregue a árvore sugerida audiovisual)
        e adicione seus modelos de equipamentos.
      </p>
      <div class="mt-5 flex items-center gap-2">
        <Button variant="outline" href="/catalog/categories">
          <FolderTree class="mr-2 h-4 w-4" />
          Configurar categorias
        </Button>
        {#if canManage}
          <Button href="/catalog/new">
            <Plus class="mr-2 h-4 w-4" />
            Novo modelo
          </Button>
        {/if}
      </div>
    </div>
  {:else}
    <div
      class="overflow-hidden rounded-xl border border-border bg-card shadow-xs"
    >
      <VirtualList
        {items}
        itemHeight={76}
        height={560}
        getKey={(item) => item.model.id}
        onEndReached={() => fetchModels(false)}
      >
        {#snippet children(item)}
          {@const m = item.model}
          <button
            type="button"
            onclick={() => goto(`/catalog/${m.id}`)}
            class="flex h-[76px] w-full items-center justify-between gap-4 border-b border-border px-5 text-left transition-colors hover:bg-muted/40"
          >
            <CatalogThumb
              blobId={item.primary_thumb_blob_id}
              mimeType={item.primary_thumb_mime_type}
              alt={m.name}
            />
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="truncate font-medium text-foreground"
                  >{m.name}</span
                >
                {#if !m.active}
                  <span
                    class="rounded bg-muted px-2 py-0.5 text-[11px] font-medium text-muted-foreground"
                  >
                    Inativo
                  </span>
                {/if}
              </div>
              <div
                class="mt-0.5 flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
              >
                <span>{item.category_name}</span>
                {#if item.brand_name}
                  <span>• {item.brand_name}</span>
                {/if}
                {#if m.sku}
                  <span>• SKU: {m.sku}</span>
                {/if}
                <span>
                  • {m.control_mode === "serialized"
                    ? "Serializado"
                    : "Quantidade"}
                </span>
              </div>
            </div>

            <div class="shrink-0 text-right">
              <div class="text-sm font-medium text-foreground">
                {formatCents(m.daily_rate_cents)}
              </div>
              <div class="text-[11px] text-muted-foreground">diária</div>
            </div>
          </button>
        {/snippet}
      </VirtualList>
      {#if loadingMore}
        <div
          class="border-t border-border p-3 text-center text-xs text-muted-foreground"
        >
          Carregando mais modelos...
        </div>
      {/if}
    </div>
  {/if}
</div>
