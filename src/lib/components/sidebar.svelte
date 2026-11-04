<script lang="ts">
  import {
    LayoutDashboard,
    FileSpreadsheet,
    CalendarDays,
    Speaker,
    Users,
    Palette,
    ChevronLeft,
    ChevronRight,
  } from "lucide-svelte";
  import { page } from "$app/state";
  import { _ } from "$lib/i18n";
  import { cn } from "$lib/ui/utils";

  interface Props {
    collapsed?: boolean;
    companyName?: string;
    logoUrl?: string | null;
    onToggleCollapse?: () => void;
  }

  let {
    collapsed = $bindable(false),
    companyName = "EventRent",
    logoUrl = null,
    onToggleCollapse,
  }: Props = $props();

  const navItems = [
    { labelKey: "nav.dashboard", href: "/", icon: LayoutDashboard },
    {
      labelKey: "nav.quotes",
      href: "#quotes",
      icon: FileSpreadsheet,
      disabled: true,
    },
    {
      labelKey: "nav.reservations",
      href: "#reservations",
      icon: CalendarDays,
      disabled: true,
    },
    {
      labelKey: "nav.catalog",
      href: "#catalog",
      icon: Speaker,
      disabled: true,
    },
    {
      labelKey: "nav.customers",
      href: "#customers",
      icon: Users,
      disabled: true,
    },
    {
      labelKey: "nav.appearance",
      href: "/configuracoes/aparencia",
      icon: Palette,
    },
  ];
</script>

<aside
  class={cn(
    "relative flex flex-col border-r border-border bg-card transition-all duration-300 select-none z-30",
    collapsed ? "w-18" : "w-64",
  )}
>
  <div
    class="flex h-16 items-center gap-3 border-b border-border px-4 overflow-hidden"
  >
    {#if logoUrl}
      <img
        src={logoUrl}
        alt={companyName}
        class="h-8 w-8 object-contain rounded-sm shrink-0"
      />
    {:else}
      <div
        class="flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius)] bg-primary text-primary-foreground font-bold text-sm shadow-xs"
      >
        {companyName.slice(0, 2).toUpperCase()}
      </div>
    {/if}

    {#if !collapsed}
      <div class="flex flex-col min-w-0 transition-opacity duration-200">
        <span
          class="truncate font-semibold text-sm leading-tight text-foreground"
          >{companyName}</span
        >
        <span class="text-xs text-muted-foreground truncate"
          >{$_("app.subtitle")}</span
        >
      </div>
    {/if}
  </div>

  <nav class="flex-1 space-y-1.5 p-3 overflow-y-auto">
    {#each navItems as item (item.labelKey)}
      {@const isActive = page.url.pathname === item.href}
      <a
        href={item.disabled ? undefined : item.href}
        class={cn(
          "flex items-center gap-3 rounded-[var(--radius)] px-3 py-2 text-sm font-medium transition-colors",
          isActive
            ? "bg-primary text-primary-foreground shadow-xs"
            : item.disabled
              ? "opacity-40 cursor-not-allowed text-muted-foreground"
              : "text-muted-foreground hover:bg-muted hover:text-foreground",
        )}
        title={collapsed ? $_(item.labelKey) : undefined}
      >
        <item.icon class="h-5 w-5 shrink-0" />
        {#if !collapsed}
          <span class="truncate">{$_(item.labelKey)}</span>
          {#if item.disabled}
            <span
              class="ml-auto text-[10px] uppercase tracking-wider bg-muted text-muted-foreground px-1.5 py-0.5 rounded-xs"
            >
              {$_("nav.soonBadge")}
            </span>
          {/if}
        {/if}
      </a>
    {/each}
  </nav>

  <div class="border-t border-border p-3">
    <button
      type="button"
      onclick={onToggleCollapse}
      class="flex w-full items-center justify-center gap-2 rounded-[var(--radius)] px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors cursor-pointer"
      title={collapsed ? $_("nav.expand") : $_("nav.collapse")}
    >
      {#if collapsed}
        <ChevronRight class="h-4 w-4" />
      {:else}
        <ChevronLeft class="h-4 w-4" />
        <span>{$_("nav.collapse")}</span>
      {/if}
    </button>
  </div>
</aside>
