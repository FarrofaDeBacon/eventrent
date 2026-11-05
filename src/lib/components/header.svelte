<script lang="ts">
  import { Search, Sun, Moon, Sparkles, LogOut, User } from "lucide-svelte";
  import { toggleMode, mode } from "mode-watcher";
  import { appSession, DEFAULT_COMPANY_NAME } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";

  interface Props {
    title?: string;
    onOpenCommand?: () => void;
    onLogout?: () => void;
  }

  let {
    title = DEFAULT_COMPANY_NAME,
    onOpenCommand,
    onLogout,
  }: Props = $props();
</script>

<header
  class="flex h-16 items-center justify-between border-b border-border bg-card px-6 select-none z-20"
>
  <div class="flex items-center gap-4">
    <h1 class="text-lg font-semibold tracking-tight text-foreground">
      {title}
    </h1>
    <span
      class="hidden sm:inline-flex items-center gap-1.5 rounded-full bg-primary/10 px-2.5 py-0.5 text-xs font-medium text-primary"
    >
      <Sparkles class="h-3 w-3" />
      {$_("app.phaseBadge")}
    </span>
  </div>

  <div class="flex items-center gap-3">
    <button
      type="button"
      onclick={onOpenCommand}
      class="flex items-center gap-2 rounded-[var(--radius)] border border-input bg-background/50 px-3 py-1.5 text-xs text-muted-foreground shadow-2xs hover:bg-muted hover:text-foreground transition-colors cursor-pointer"
    >
      <Search class="h-3.5 w-3.5" />
      <span class="hidden md:inline">{$_("command.placeholder")}</span>
      <kbd
        class="pointer-events-none rounded border border-border bg-muted px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground"
      >
        {$_("command.shortcutLabel")}
      </kbd>
    </button>

    <Button
      variant="ghost"
      size="icon"
      onclick={toggleMode}
      title={$_("command.toggleTheme")}
      aria-label={$_("common.toggleThemeAria")}
    >
      {#if $mode === "dark"}
        <Sun class="h-4 w-4 text-warning" />
      {:else}
        <Moon class="h-4 w-4" />
      {/if}
    </Button>

    {#if appSession.session}
      <div
        class="flex items-center gap-2.5 border-l border-border pl-3"
        data-testid="header-user-info"
      >
        <div
          class="hidden sm:flex items-center gap-2 rounded-[var(--radius)] bg-muted/60 px-2.5 py-1 text-xs"
        >
          <User class="h-3.5 w-3.5 text-primary shrink-0" />
          <span class="font-medium text-foreground truncate max-w-36">
            {appSession.session.name}
          </span>
          <Badge variant="secondary">
            {appSession.session.role_name}
          </Badge>
        </div>

        <Button
          variant="outline"
          size="sm"
          onclick={onLogout}
          title={$_("header.logoutTooltip")}
          aria-label={$_("header.logout")}
        >
          <LogOut class="h-3.5 w-3.5" />
          <span>{$_("header.logout")}</span>
        </Button>
      </div>
    {/if}
  </div>
</header>
