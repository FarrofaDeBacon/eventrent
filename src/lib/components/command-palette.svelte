<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { toggleMode } from "mode-watcher";
  import {
    Search,
    LayoutDashboard,
    Users,
    UserPlus,
    UserCog,
    Shield,
    Palette,
    SunMoon,
    LogOut,
    ArrowRight,
  } from "lucide-svelte";
  import {
    appSession,
    getCustomerRepository,
    hasPermission,
    PERMISSIONS,
    type Customer,
  } from "$lib/data";
  import { formatDocument } from "$lib/utils/document";
  import { _ } from "$lib/i18n";
  import { Dialog } from "$lib/ui/dialog";

  interface Props {
    open?: boolean;
    onLogout?: () => void;
  }

  let { open = $bindable(false), onLogout }: Props = $props();
  let query = $state("");
  let selectedIndex = $state(0);
  let matchedCustomers = $state<Customer[]>([]);
  let isSearchingCustomers = $state(false);
  let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined;

  interface CommandItem {
    id: string;
    label: string;
    subtitle?: string;
    category: string;
    icon: typeof LayoutDashboard;
    action: () => void;
  }

  interface BaseCommandDef {
    id: string;
    labelKey: string;
    categoryKey: string;
    icon: typeof LayoutDashboard;
    permission?: string;
    action: () => void;
  }

  const allCommands: BaseCommandDef[] = [
    {
      id: "dashboard",
      labelKey: "command.goToDashboard",
      categoryKey: "command.navigation",
      icon: LayoutDashboard,
      permission: undefined,
      action: () => {
        open = false;
        goto("/");
      },
    },
    {
      id: "customers",
      labelKey: "command.goToCustomers",
      categoryKey: "command.navigation",
      icon: Users,
      permission: PERMISSIONS.CUSTOMER_READ,
      action: () => {
        open = false;
        goto("/customers");
      },
    },
    {
      id: "new-customer",
      labelKey: "command.newCustomer",
      categoryKey: "command.actions",
      icon: UserPlus,
      permission: PERMISSIONS.CUSTOMER_MANAGE,
      action: () => {
        open = false;
        goto("/customers/new");
      },
    },
    {
      id: "users",
      labelKey: "command.goToUsers",
      categoryKey: "command.navigation",
      icon: UserCog,
      permission: PERMISSIONS.USER_READ,
      action: () => {
        open = false;
        goto("/settings/users");
      },
    },
    {
      id: "roles",
      labelKey: "command.goToRoles",
      categoryKey: "command.navigation",
      icon: Shield,
      permission: PERMISSIONS.ROLE_READ,
      action: () => {
        open = false;
        goto("/settings/roles");
      },
    },
    {
      id: "appearance",
      labelKey: "command.goToAppearance",
      categoryKey: "command.navigation",
      icon: Palette,
      permission: PERMISSIONS.BRANDING_READ,
      action: () => {
        open = false;
        goto("/settings/appearance");
      },
    },
    {
      id: "theme",
      labelKey: "command.toggleTheme",
      categoryKey: "command.actions",
      icon: SunMoon,
      action: () => {
        open = false;
        toggleMode();
      },
    },
    {
      id: "logout",
      labelKey: "command.logout",
      categoryKey: "command.actions",
      icon: LogOut,
      action: () => {
        open = false;
        onLogout?.();
      },
    },
  ];

  let canReadCustomers = $derived(
    hasPermission(appSession.session, PERMISSIONS.CUSTOMER_READ),
  );

  let permittedBaseCommands = $derived<CommandItem[]>(
    allCommands
      .filter(
        (cmd) =>
          !cmd.permission || hasPermission(appSession.session, cmd.permission),
      )
      .map((cmd) => ({
        id: cmd.id,
        label: $_(cmd.labelKey),
        category: $_(cmd.categoryKey),
        icon: cmd.icon,
        action: cmd.action,
      })),
  );

  let filteredStaticCommands = $derived(
    query.trim() === ""
      ? permittedBaseCommands
      : permittedBaseCommands.filter(
          (cmd) =>
            cmd.label.toLowerCase().includes(query.toLowerCase()) ||
            cmd.category.toLowerCase().includes(query.toLowerCase()),
        ),
  );

  let customerCommands = $derived<CommandItem[]>(
    matchedCustomers.map((cust) => ({
      id: `customer-${cust.id}`,
      label: cust.trade_name ? `${cust.name} (${cust.trade_name})` : cust.name,
      subtitle: formatDocument(cust.document_number, cust.person_type),
      category: $_("command.customerSection"),
      icon: Users,
      action: () => {
        open = false;
        goto(`/customers/${cust.id}`);
      },
    })),
  );

  let combinedItems = $derived<CommandItem[]>([
    ...filteredStaticCommands,
    ...customerCommands,
  ]);

  $effect(() => {
    const trimmed = query.trim();
    if (!open || !canReadCustomers || trimmed.length < 2) {
      if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
      matchedCustomers = [];
      isSearchingCustomers = false;
      return;
    }

    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    isSearchingCustomers = true;

    searchDebounceTimer = setTimeout(async () => {
      try {
        const res = await getCustomerRepository().listCustomers({
          query: trimmed,
          limit: 5,
        });
        matchedCustomers = res.items;
      } catch {
        matchedCustomers = [];
      } finally {
        isSearchingCustomers = false;
      }
    }, 150);

    return () => {
      if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    };
  });

  $effect(() => {
    if (selectedIndex >= combinedItems.length) {
      selectedIndex = 0;
    }
  });

  function handleKeyDown(event: KeyboardEvent) {
    if (!appSession.session) return;

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      open = !open;
      return;
    }

    if (!open) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(1, combinedItems.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex =
        (selectedIndex - 1 + Math.max(1, combinedItems.length)) %
        Math.max(1, combinedItems.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const selected = combinedItems[selectedIndex];
      if (selected) {
        selected.action();
      }
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });
</script>

<Dialog bind:open>
  <div class="flex flex-col gap-3 -mt-2">
    <div class="flex items-center gap-3 border-b border-border pb-3 px-1">
      <Search class="h-4 w-4 text-muted-foreground shrink-0" />
      <input
        type="text"
        bind:value={query}
        placeholder={$_("command.placeholder")}
        aria-label={$_("command.placeholder")}
        class="w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
      />
    </div>

    <div class="max-h-72 overflow-y-auto space-y-1">
      {#if isSearchingCustomers}
        <div class="px-3 py-1.5 text-xs text-muted-foreground">
          {$_("command.customerSearching")}
        </div>
      {/if}

      {#if combinedItems.length === 0 && !isSearchingCustomers}
        <div class="py-6 text-center text-sm text-muted-foreground">
          {$_("command.noResults")}
        </div>
      {:else}
        {#each combinedItems as item, idx (item.id)}
          {@const isSelected = idx === selectedIndex}
          <button
            type="button"
            onclick={() => item.action()}
            class="flex w-full items-center justify-between gap-3 rounded-[var(--radius)] px-3 py-2 text-sm text-left transition-colors cursor-pointer {isSelected
              ? 'bg-primary text-primary-foreground'
              : 'text-foreground hover:bg-muted'}"
          >
            <div class="flex items-center gap-3 min-w-0">
              <item.icon class="h-4 w-4 shrink-0" />
              <div class="flex flex-col min-w-0">
                <span class="truncate">{item.label}</span>
                {#if item.subtitle}
                  <span
                    class="text-[11px] truncate {isSelected
                      ? 'text-primary-foreground/80'
                      : 'text-muted-foreground'}"
                  >
                    {item.subtitle}
                  </span>
                {/if}
              </div>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <span
                class="text-[11px] {isSelected
                  ? 'text-primary-foreground/80'
                  : 'text-muted-foreground'}"
              >
                {item.category}
              </span>
              <ArrowRight class="h-3.5 w-3.5 opacity-60" />
            </div>
          </button>
        {/each}
      {/if}
    </div>

    <div
      class="flex items-center justify-between border-t border-border pt-2 text-[11px] text-muted-foreground px-1"
    >
      <span>{$_("command.hintArrows")}</span>
      <span>{$_("command.hintEnter")}</span>
    </div>
  </div>
</Dialog>
