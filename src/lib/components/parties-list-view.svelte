<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import {
    UserPlus,
    Search,
    Building2,
    User,
    ChevronRight,
  } from "lucide-svelte";
  import {
    appSession,
    getCustomerRepository,
    hasPermission,
    PERMISSIONS,
    type Customer,
    type CustomerRoleFilter,
    type PersonType,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { formatDocument } from "$lib/utils/document";
  import VirtualList from "$lib/components/virtual-list.svelte";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import { EmptyState, ErrorState, LoadingState } from "$lib/ui";

  interface Props {
    initialRoleTab?: "customer" | "supplier" | "all";
  }

  let { initialRoleTab = "customer" }: Props = $props();

  let canReadCustomer = $derived(
    hasPermission(appSession.session, PERMISSIONS.CUSTOMER_READ) ||
      hasPermission(appSession.session, PERMISSIONS.CUSTOMER_MANAGE),
  );
  let canReadSupplier = $derived(
    hasPermission(appSession.session, PERMISSIONS.SUPPLIER_READ) ||
      hasPermission(appSession.session, PERMISSIONS.SUPPLIER_MANAGE),
  );
  let canManageCustomer = $derived(
    hasPermission(appSession.session, PERMISSIONS.CUSTOMER_MANAGE),
  );
  let canManageSupplier = $derived(
    hasPermission(appSession.session, PERMISSIONS.SUPPLIER_MANAGE),
  );

  function resolveInitialTab(): CustomerRoleFilter {
    if (initialRoleTab === "supplier" && canReadSupplier) return "supplier";
    if (initialRoleTab === "customer" && canReadCustomer) return "customer";
    if (canReadCustomer && canReadSupplier) return "all";
    if (canReadSupplier) return "supplier";
    return "customer";
  }

  let roleTab = $state<CustomerRoleFilter>("customer");
  let isLoading = $state(true);
  let isLoadingMore = $state(false);
  let loadError = $state<string | null>(null);
  let customers = $state<Customer[]>([]);
  let nextCursor = $state<string | null>(null);
  let hasMore = $state(false);
  let totalCount = $state<number | null>(null);

  let searchQuery = $state("");
  let personTypeFilter = $state<"all" | PersonType>("all");
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let initialMounted = false;

  let canCreateInCurrentTab = $derived(
    roleTab === "supplier"
      ? canManageSupplier
      : roleTab === "customer"
        ? canManageCustomer
        : canManageCustomer || canManageSupplier,
  );

  let pageTitle = $derived(
    roleTab === "supplier"
      ? $_("customers.suppliersTitle")
      : roleTab === "all"
        ? $_("customers.allPartiesTitle")
        : $_("customers.title"),
  );

  let pageDescription = $derived(
    roleTab === "supplier"
      ? $_("customers.suppliersDescription")
      : roleTab === "all"
        ? $_("customers.allPartiesDescription")
        : $_("customers.description"),
  );

  async function fetchFirstPage() {
    isLoading = true;
    loadError = null;
    try {
      const repo = getCustomerRepository();
      const page = await repo.listCustomers({
        query: searchQuery.trim() || undefined,
        person_type: personTypeFilter === "all" ? undefined : personTypeFilter,
        role_filter: roleTab,
        limit: 50,
      });
      customers = page.items;
      nextCursor = page.next_cursor ?? null;
      hasMore = page.has_more;
      totalCount = page.total_count ?? null;
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  async function loadMore() {
    if (!hasMore || !nextCursor || isLoadingMore || isLoading) return;
    isLoadingMore = true;
    try {
      const repo = getCustomerRepository();
      const page = await repo.listCustomers({
        query: searchQuery.trim() || undefined,
        person_type: personTypeFilter === "all" ? undefined : personTypeFilter,
        role_filter: roleTab,
        cursor: nextCursor,
        limit: 50,
      });
      customers = [...customers, ...page.items];
      nextCursor = page.next_cursor ?? null;
      hasMore = page.has_more;
      if (page.total_count !== undefined && page.total_count !== null) {
        totalCount = page.total_count;
      }
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoadingMore = false;
    }
  }

  function handleSearchInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    searchQuery = input.value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      fetchFirstPage();
    }, 220);
  }

  function selectFilter(nextFilter: "all" | PersonType) {
    if (personTypeFilter === nextFilter) return;
    personTypeFilter = nextFilter;
    if (debounceTimer) clearTimeout(debounceTimer);
    fetchFirstPage();
  }

  function selectRoleTab(nextTab: CustomerRoleFilter) {
    if (roleTab === nextTab) return;
    roleTab = nextTab;
    if (debounceTimer) clearTimeout(debounceTimer);
    fetchFirstPage();
  }

  onMount(() => {
    initialMounted = true;
    roleTab = resolveInitialTab();
    fetchFirstPage();
    return () => {
      if (debounceTimer && initialMounted) clearTimeout(debounceTimer);
    };
  });
</script>

<div class="space-y-6">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <div class="flex items-center gap-3">
        <h2 class="text-2xl font-bold tracking-tight text-foreground">
          {pageTitle}
        </h2>
        {#if totalCount !== null}
          <Badge variant="secondary" data-testid="customers-total-badge">
            {$_("customers.totalCountLabel", { values: { count: totalCount } })}
          </Badge>
        {/if}
      </div>
      <p class="text-sm text-muted-foreground mt-1">
        {pageDescription}
      </p>
    </div>

    {#if canCreateInCurrentTab}
      <Button
        onclick={() =>
          goto(
            roleTab === "supplier"
              ? "/customers/new?role=supplier"
              : "/customers/new",
          )}
        data-testid="new-customer-btn"
      >
        <UserPlus class="h-4 w-4" />
        {roleTab === "supplier"
          ? $_("customers.newSupplierBtn")
          : $_("customers.newCustomerBtn")}
      </Button>
    {/if}
  </div>

  <!-- Abas de Papéis (Clientes, Fornecedores, Todos) -->
  <div
    class="inline-flex rounded-[var(--radius)] border border-border p-1 bg-muted/40"
    role="tablist"
  >
    {#if canReadCustomer}
      <button
        type="button"
        role="tab"
        aria-selected={roleTab === "customer"}
        onclick={() => selectRoleTab("customer")}
        data-testid="tab-customers"
        class="px-4 py-1.5 text-sm font-medium rounded-sm transition-colors cursor-pointer {roleTab ===
        'customer'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:text-foreground'}"
      >
        {$_("customers.tabCustomers")}
      </button>
    {/if}
    {#if canReadSupplier}
      <button
        type="button"
        role="tab"
        aria-selected={roleTab === "supplier"}
        onclick={() => selectRoleTab("supplier")}
        data-testid="tab-suppliers"
        class="px-4 py-1.5 text-sm font-medium rounded-sm transition-colors cursor-pointer {roleTab ===
        'supplier'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:text-foreground'}"
      >
        {$_("customers.tabSuppliers")}
      </button>
    {/if}
    {#if canReadCustomer && canReadSupplier}
      <button
        type="button"
        role="tab"
        aria-selected={roleTab === "all"}
        onclick={() => selectRoleTab("all")}
        data-testid="tab-all"
        class="px-4 py-1.5 text-sm font-medium rounded-sm transition-colors cursor-pointer {roleTab ===
        'all'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:text-foreground'}"
      >
        {$_("customers.tabAll")}
      </button>
    {/if}
  </div>

  <!-- Barra de busca e filtros -->
  <div class="flex flex-col md:flex-row items-stretch md:items-center gap-3">
    <div class="relative flex-1">
      <Search
        class="h-4 w-4 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none"
      />
      <Input
        type="search"
        value={searchQuery}
        oninput={handleSearchInput}
        placeholder={$_("customers.searchPlaceholder")}
        class="pl-9"
        data-testid="customers-search-input"
      />
    </div>

    <div
      class="inline-flex rounded-[var(--radius)] border border-border p-1 bg-muted/40 self-start"
    >
      <button
        type="button"
        onclick={() => selectFilter("all")}
        data-testid="filter-all"
        class="px-3 py-1.5 text-xs font-medium rounded-sm transition-colors cursor-pointer {personTypeFilter ===
        'all'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:text-foreground'}"
      >
        {$_("customers.filterAll")}
      </button>
      <button
        type="button"
        onclick={() => selectFilter("individual")}
        data-testid="filter-individual"
        class="px-3 py-1.5 text-xs font-medium rounded-sm transition-colors cursor-pointer {personTypeFilter ===
        'individual'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:text-foreground'}"
      >
        {$_("customers.filterIndividual")}
      </button>
      <button
        type="button"
        onclick={() => selectFilter("legal")}
        data-testid="filter-legal"
        class="px-3 py-1.5 text-xs font-medium rounded-sm transition-colors cursor-pointer {personTypeFilter ===
        'legal'
          ? 'bg-primary text-primary-foreground shadow-xs'
          : 'text-muted-foreground hover:text-foreground'}"
      >
        {$_("customers.filterLegal")}
      </button>
    </div>
  </div>

  {#if isLoading}
    <LoadingState message={$_("customers.loading")} />
  {:else if loadError}
    <ErrorState message={loadError} onRetry={fetchFirstPage} />
  {:else if customers.length === 0}
    <EmptyState
      title={$_("customers.emptyTitle")}
      description={$_("customers.emptyMessage")}
    />
  {:else}
    <VirtualList
      items={customers}
      itemHeight={76}
      height={520}
      getKey={(item) => item.id}
      onEndReached={loadMore}
    >
      {#snippet children(customer)}
        <a
          href={`/customers/${customer.id}`}
          data-testid={`customer-row-${customer.id}`}
          class="flex h-full items-center justify-between gap-4 px-4 hover:bg-muted/40 transition-colors"
        >
          <div class="flex items-center gap-3 min-w-0">
            <div
              class="flex h-10 w-10 shrink-0 items-center justify-center rounded-[var(--radius)] bg-muted text-muted-foreground"
            >
              {#if customer.person_type === "legal"}
                <Building2 class="h-5 w-5" />
              {:else}
                <User class="h-5 w-5" />
              {/if}
            </div>
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-medium text-sm text-foreground truncate">
                  {customer.name}
                </span>
                <Badge variant="outline" class="shrink-0">
                  {customer.person_type === "legal"
                    ? $_("customers.personTypeLegal")
                    : $_("customers.personTypeIndividual")}
                </Badge>
                {#if customer.is_supplier && customer.is_customer}
                  <Badge variant="secondary" class="shrink-0">
                    {$_("customers.roleBoth")}
                  </Badge>
                {:else if customer.is_supplier}
                  <Badge variant="secondary" class="shrink-0">
                    {$_("customers.roleSupplier")}
                  </Badge>
                {/if}
              </div>
              <div
                class="flex items-center gap-2 text-xs text-muted-foreground truncate"
              >
                <span>
                  {formatDocument(
                    customer.document_number,
                    customer.person_type,
                  )}
                </span>
                {#if customer.trade_name}
                  <span>•</span>
                  <span class="truncate">{customer.trade_name}</span>
                {/if}
              </div>
            </div>
          </div>

          <ChevronRight class="h-4 w-4 text-muted-foreground shrink-0" />
        </a>
      {/snippet}
    </VirtualList>

    {#if hasMore}
      <div class="flex justify-center pt-2">
        <Button
          variant="outline"
          onclick={loadMore}
          disabled={isLoadingMore}
          data-testid="load-more-customers-btn"
        >
          {isLoadingMore
            ? $_("customers.loadingMore")
            : $_("customers.loadMoreBtn")}
        </Button>
      </div>
    {/if}
  {/if}
</div>
