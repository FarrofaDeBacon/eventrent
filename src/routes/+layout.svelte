<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import { ModeWatcher } from 'mode-watcher';
  import { Toaster } from 'svelte-sonner';
  import { setupI18n } from '$lib/i18n';
  import { getBrandingRepository } from '$lib/data';
  import { applyBranding } from '$lib/theme/branding';
  import type { BrandingConfig } from '$lib/theme/tokens';
  import Sidebar from '$lib/components/sidebar.svelte';
  import Header from '$lib/components/header.svelte';
  import CommandPalette from '$lib/components/command-palette.svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let sidebarCollapsed = $state(false);
  let commandPaletteOpen = $state(false);
  let companyName = $state('EventRent Audiovisual');
  let logoUrl = $state<string | null>(null);

  setupI18n();

  onMount(() => {
    const handleBrandingEvent = (event: Event) => {
      const custom = event as CustomEvent<BrandingConfig>;
      if (custom.detail) {
        companyName = custom.detail.company_name;
        logoUrl = custom.detail.logo_url ?? null;
      }
    };

    window.addEventListener('eventrent:branding', handleBrandingEvent);

    (async () => {
      try {
        const repo = getBrandingRepository();
        const defaultTenantId = '018f0000-0000-7000-8000-000000000001';
        const branding = await repo.getBranding(defaultTenantId);

        if (branding) {
          companyName = branding.company_name;
          logoUrl = branding.logo_url;
          applyBranding(branding);
        }
      } catch (err) {
        console.error('Falha ao inicializar branding:', err);
      }
    })();

    return () => {
      window.removeEventListener('eventrent:branding', handleBrandingEvent);
    };
  });
</script>

<ModeWatcher defaultMode="system" />
<Toaster position="top-right" richColors />
<CommandPalette bind:open={commandPaletteOpen} />

<div class="flex h-screen w-screen overflow-hidden bg-background text-foreground">
  <Sidebar
    bind:collapsed={sidebarCollapsed}
    {companyName}
    {logoUrl}
    onToggleCollapse={() => (sidebarCollapsed = !sidebarCollapsed)}
  />

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
