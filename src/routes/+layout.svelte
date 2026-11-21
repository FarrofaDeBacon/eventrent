<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import type { Snippet } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { ModeWatcher } from "mode-watcher";
  import { Toaster, toast } from "svelte-sonner";
  import { setupI18n, _ } from "$lib/i18n";
  import {
    appSession,
    canAccessRoute,
    getAuthRepository,
    isTauriRuntime,
  } from "$lib/data";
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

  let isTauri = $state(isTauriRuntime());
  let isDev = import.meta.env.DEV;
  let isDemoMockMode = $derived(!isTauri && isDev);
  let isUnsupportedProductionWeb = $derived(!isTauri && !isDev);

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
    if (isUnsupportedProductionWeb) return;
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
      if (isUnsupportedProductionWeb) {
        appSession.initialized = true;
        return;
      }
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

{#if isUnsupportedProductionWeb}
  <div
    class="flex min-h-screen w-screen flex-col items-center justify-center bg-background p-6 text-center text-foreground"
  >
    <div
      class="max-w-md space-y-4 rounded-xl border border-border bg-card p-8 shadow-lg"
    >
      <div
        class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary/10 text-primary"
      >
        <svg
          class="h-6 w-6"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <rect width="20" height="14" x="2" y="3" rx="2" />
          <line x1="8" x2="16" y1="21" y2="21" />
          <line x1="12" x2="12" y1="17" y2="21" />
        </svg>
      </div>
      <h1 class="text-xl font-bold tracking-tight">
        Execute o aplicativo desktop EventRent
      </h1>
      <p class="text-sm text-muted-foreground">
        O EventRent foi desenvolvido para execução nativa em desktop com banco
        de dados local seguro. O acesso direto via navegador web em modo de
        produção não é suportado.
      </p>
    </div>
  </div>
{:else}
  {#if isDemoMockMode}
    <aside
      class="sticky top-0 z-50 flex items-center justify-center gap-2 border-b border-warning/40 bg-warning/15 px-4 py-1.5 text-xs font-semibold text-warning"
      aria-label="Aviso de Modo de Demonstração"
    >
      <svg
        class="h-4 w-4 shrink-0"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        />
      </svg>
      <span>MODO DE DEMONSTRAÇÃO: Não use dados reais</span>
    </aside>
  {/if}

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
{/if}
