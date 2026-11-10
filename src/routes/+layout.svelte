<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import type { Snippet } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { ModeWatcher } from "mode-watcher";
  import { Toaster, toast } from "svelte-sonner";
  import { setupI18n, _ } from "$lib/i18n";
  import { appSession, canAccessRoute, getAuthRepository } from "$lib/data";
  import type { BrandingConfig } from "$lib/theme/tokens";
  import Sidebar from "$lib/components/sidebar.svelte";
  import Header from "$lib/components/header.svelte";
  import CommandPalette from "$lib/components/command-palette.svelte";
  import { LoadingState } from "$lib/ui";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let sidebarCollapsed = $state(false);
  let commandPaletteOpen = $state(false);

  setupI18n();

  let isStandaloneRoute = $derived(
    page.url.pathname.startsWith("/setup") ||
      page.url.pathname.startsWith("/login"),
  );

  async function handleLogout() {
    try {
      const repo = getAuthRepository();
      await repo.logout();
      appSession.setSession(null);
      toast.success($_("header.loggedOutToast"));
      await goto("/login");
    } catch (err) {
      console.error("Erro ao encerrar sessão:", err);
    }
  }

  $effect(() => {
    if (!appSession.initialized) return;
    const pathname = page.url.pathname;

    if (!appSession.hasOwner) {
      if (!pathname.startsWith("/setup")) {
        goto("/setup");
      }
      return;
    }

    if (!appSession.session) {
      if (!pathname.startsWith("/login") && !pathname.startsWith("/setup")) {
        goto("/login");
      }
      return;
    }

    if (pathname.startsWith("/login")) {
      goto("/");
      return;
    }

    if (!canAccessRoute(appSession.session, pathname)) {
      goto("/");
    }
  });

  onMount(() => {
    const handleBrandingEvent = (event: Event) => {
      const custom = event as CustomEvent<BrandingConfig>;
      if (custom.detail) {
        appSession.companyName = custom.detail.company_name;
        appSession.logoUrl = custom.detail.logo_url ?? null;
      }
    };

    window.addEventListener("eventrent:branding", handleBrandingEvent);

    (async () => {
      try {
        await appSession.refresh();
      } catch (err) {
        console.error("Falha ao carregar status inicial da aplicação:", err);
        appSession.initialized = true;
      }
    })();

    return () => {
      window.removeEventListener("eventrent:branding", handleBrandingEvent);
    };
  });
</script>

<ModeWatcher defaultMode="system" />
<Toaster position="top-right" richColors />

{#if !appSession.initialized}
  <div
    class="flex h-screen w-screen items-center justify-center bg-background text-foreground"
  >
    <LoadingState message={$_("app.initializing")} />
  </div>
{:else if isStandaloneRoute}
  <div
    class="min-h-screen w-screen overflow-y-auto bg-background text-foreground"
  >
    {@render children()}
  </div>
{:else}
  <CommandPalette bind:open={commandPaletteOpen} onLogout={handleLogout} />

  <div
    class="flex h-screen w-screen overflow-hidden bg-background text-foreground"
  >
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      companyName={appSession.companyName}
      logoUrl={appSession.logoUrl}
      logoDarkUrl={appSession.logoDarkUrl}
      iconUrl={appSession.iconUrl}
      onToggleCollapse={() => (sidebarCollapsed = !sidebarCollapsed)}
    />

    <div class="flex flex-1 flex-col overflow-hidden">
      <Header
        title={appSession.companyName}
        onOpenCommand={() => (commandPaletteOpen = true)}
        onLogout={handleLogout}
      />

      <main class="flex-1 overflow-y-auto p-6 md:p-8">
        <div class="mx-auto max-w-6xl">
          {@render children()}
        </div>
      </main>
    </div>
  </div>
{/if}
