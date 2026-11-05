<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { toggleMode } from "mode-watcher";
  import {
    Search,
    LayoutDashboard,
    UserCog,
    Shield,
    Palette,
    SunMoon,
    LogOut,
    ArrowRight,
  } from "lucide-svelte";
  import { appSession, hasPermission, PERMISSIONS } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { Dialog } from "$lib/ui/dialog";

  interface Props {
    open?: boolean;
    onLogout?: () => void;
  }

  let { open = $bindable(false), onLogout }: Props = $props();
  let query = $state("");
  let selectedIndex = $state(0);

  interface CommandItem {
    id: string;
    labelKey: string;
    categoryKey: string;
    icon: typeof LayoutDashboard;
    permission?: string;
    action: () => void;
  }

  const allCommands: CommandItem[] = [
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

  let permittedCommands = $derived(
    allCommands.filter(
      (cmd) =>
        !cmd.permission || hasPermission(appSession.session, cmd.permission),
    ),
  );

  let filteredCommands = $derived(
    query.trim() === ""
      ? permittedCommands
      : permittedCommands.filter(
          (cmd) =>
            $_(cmd.labelKey).toLowerCase().includes(query.toLowerCase()) ||
            $_(cmd.categoryKey).toLowerCase().includes(query.toLowerCase()),
        ),
  );

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
      selectedIndex =
        (selectedIndex + 1) % Math.max(1, filteredCommands.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex =
        (selectedIndex - 1 + filteredCommands.length) %
        Math.max(1, filteredCommands.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const selected = filteredCommands[selectedIndex];
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
      {#if filteredCommands.length === 0}
        <div class="py-6 text-center text-sm text-muted-foreground">
          {$_("command.noResults")}
        </div>
      {:else}
        {#each filteredCommands as item, idx (item.id)}
          {@const isSelected = idx === selectedIndex}
          <button
            type="button"
            onclick={() => item.action()}
            class="flex w-full items-center justify-between gap-3 rounded-[var(--radius)] px-3 py-2 text-sm text-left transition-colors cursor-pointer {isSelected
              ? 'bg-primary text-primary-foreground'
              : 'text-foreground hover:bg-muted'}"
          >
            <div class="flex items-center gap-3">
              <item.icon class="h-4 w-4 shrink-0" />
              <span>{$_(item.labelKey)}</span>
            </div>
            <div class="flex items-center gap-2">
              <span
                class="text-[11px] {isSelected
                  ? 'text-primary-foreground/80'
                  : 'text-muted-foreground'}"
              >
                {$_(item.categoryKey)}
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
