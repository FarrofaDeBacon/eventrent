<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import { ModeWatcher } from 'mode-watcher';
  import { Toaster } from 'svelte-sonner';
  import { setupI18n, waitLocale } from '$lib/i18n';
  import { getBrandingRepository } from '$lib/data';
  import { applyBranding } from '$lib/theme/branding';
  import Sidebar from '$lib/components/sidebar.svelte';
  import Header from '$lib/components/header.svelte';
  import CommandPalette from '$lib/components/command-palette.svelte';
  import { LoadingState } from '$lib/ui';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let isReady = $state(false);
  let sidebarCollapsed = $state(false);
  let commandPaletteOpen = $state(false);
  let companyName = $state('EventRent Audiovisual');
  let logoUrl = $state<string | null>(null);

  // Inicializa a camada de internacionalização
  setupI18n();

  onMount(async () => {
    try {
      await waitLocale();

      // Carrega o branding do tenant padrão
      const repo = getBrandingRepository();
      const defaultTenantId = '018f0000-0000-7000-8000-000000000001';
      const branding = await repo.getBranding(defaultTenantId);

      if (branding) {
        companyName = branding.company_name;
        logoUrl = branding.logo_url;
        applyBranding(branding);
      }
    } catch (err) {
      console.error('Falha ao inicializar branding ou i18n:', err);
    } finally {
      isReady = true;
    }
  });
</script>

<ModeWatcher defaultMode="system" />
<Toaster position="top-right" richColors />
<CommandPalette bind:open={commandPaletteOpen} />

{#if !isReady}
  <div class="flex h-screen w-screen items-center justify-center bg-background">
    <LoadingState message="Inicializando EventRent..." />
  </div>
{:else}
  <div class="flex h-screen w-screen overflow-hidden bg-background text-foreground">
    <!-- Barra Lateral Recolhível -->
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      {companyName}
      {logoUrl}
      onToggleCollapse={() => (sidebarCollapsed = !sidebarCollapsed)}
    />

    <!-- Área de Conteúdo Principal -->
    <div class="flex flex-1 flex-col overflow-hidden">
      <Header
        title={companyName}
        onOpenCommand={() => (commandPaletteOpen = true)}
      />

      <main class="flex-1 overflow-y-auto p-6 md:p-8">
        <div class="mx-auto max-w-6xl">
          {@render children()}
        </div>
      </main>
    </div>
  </div>
{/if}
