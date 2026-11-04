<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "svelte-sonner";
  import {
    Palette,
    Upload,
    Trash2,
    CheckCircle2,
    RotateCcw,
    Building2,
  } from "lucide-svelte";
  import {
    appSession,
    DEFAULT_COMPANY_NAME,
    getBrandingRepository,
    hasPermission,
    PERMISSIONS,
  } from "$lib/data";
  import { applyBranding } from "$lib/theme/branding";
  import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
  } from "$lib/ui/card";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import { Badge } from "$lib/ui/badge";
  import { LoadingState } from "$lib/ui";
  import { _ } from "$lib/i18n";

  let isLoading = $state(true);
  let isSaving = $state(false);
  let canEdit = $derived(
    hasPermission(appSession.session, PERMISSIONS.BRANDING_EDIT),
  );

  let companyName = $state(DEFAULT_COMPANY_NAME);
  let primaryColor = $state("#2563eb");
  let secondaryColor = $state("#475569");
  let accentColor = $state("#7c3aed");
  let logoUrl = $state<string | null>(null);

  onMount(async () => {
    try {
      const repo = getBrandingRepository();
      const data = await repo.getBranding(appSession.defaultTenantId);
      if (data) {
        companyName = data.company_name;
        primaryColor = data.primary_color;
        secondaryColor = data.secondary_color;
        accentColor = data.accent_color;
        logoUrl = data.logo_url;
      }
    } catch (err) {
      console.error("Erro ao carregar dados de branding:", err);
      toast.error($_("appearance.saveError"));
    } finally {
      isLoading = false;
    }
  });

  $effect(() => {
    applyBranding({
      company_name: companyName,
      primary_color: primaryColor,
      secondary_color: secondaryColor,
      accent_color: accentColor,
      logo_url: logoUrl,
    });
  });

  async function handleSave() {
    isSaving = true;
    try {
      const repo = getBrandingRepository();
      const updated = await repo.saveBranding(appSession.defaultTenantId, {
        company_name: companyName,
        primary_color: primaryColor,
        secondary_color: secondaryColor,
        accent_color: accentColor,
        logo_url: logoUrl,
      });

      applyBranding(updated);
      toast.success($_("appearance.saveSuccess"));
    } catch (err) {
      console.error("Falha ao salvar branding no SQLite:", err);
      toast.error($_("appearance.saveError"));
    } finally {
      isSaving = false;
    }
  }

  function handleLogoUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      const reader = new FileReader();
      reader.onload = (e) => {
        logoUrl = e.target?.result as string;
      };
      reader.readAsDataURL(file);
    }
  }

  function handleRemoveLogo() {
    logoUrl = null;
  }

  function handleResetDefaults() {
    companyName = $_("app.defaultCompanyName");
    primaryColor = "#2563eb";
    secondaryColor = "#475569";
    accentColor = "#7c3aed";
    logoUrl = null;
  }
</script>

{#if isLoading}
  <LoadingState message={$_("appearance.loading")} />
{:else}
  <div class="space-y-6">
    <div>
      <h2 class="text-2xl font-bold tracking-tight text-foreground">
        {$_("appearance.title")}
      </h2>
      <p class="text-sm text-muted-foreground mt-1">
        {$_("appearance.description")}
      </p>
    </div>

    <div class="grid gap-6 lg:grid-cols-3">
      <div class="lg:col-span-2 space-y-6">
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2 text-base">
              <Building2 class="h-4 w-4 text-primary" />
              {$_("appearance.companySectionTitle")}
            </CardTitle>
            <CardDescription
              >{$_("appearance.companySectionDesc")}</CardDescription
            >
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="space-y-2">
              <label
                for="companyName"
                class="text-sm font-medium text-foreground"
              >
                {$_("appearance.companyNameLabel")}
              </label>
              <Input
                id="companyName"
                bind:value={companyName}
                placeholder={$_("appearance.companyNamePlaceholder")}
              />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2 text-base">
              <Palette class="h-4 w-4 text-primary" />
              {$_("appearance.paletteSectionTitle")}
            </CardTitle>
            <CardDescription
              >{$_("appearance.paletteSectionDesc")}</CardDescription
            >
          </CardHeader>
          <CardContent class="space-y-5">
            <div
              class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-border pb-4"
            >
              <div>
                <label
                  for="primaryColor"
                  class="text-sm font-medium text-foreground"
                >
                  {$_("appearance.primaryColorLabel")}
                </label>
                <p class="text-xs text-muted-foreground">
                  {$_("appearance.primaryColorDesc")}
                </p>
              </div>
              <div class="flex items-center gap-3">
                <input
                  id="primaryColor"
                  type="color"
                  bind:value={primaryColor}
                  class="h-9 w-12 cursor-pointer rounded border border-input bg-transparent p-1"
                />
                <Input
                  bind:value={primaryColor}
                  aria-label={$_("appearance.primaryColorLabel")}
                  class="w-28 font-mono text-xs uppercase"
                  maxlength={7}
                />
              </div>
            </div>

            <div
              class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-border pb-4"
            >
              <div>
                <label
                  for="secondaryColor"
                  class="text-sm font-medium text-foreground"
                >
                  {$_("appearance.secondaryColorLabel")}
                </label>
                <p class="text-xs text-muted-foreground">
                  {$_("appearance.secondaryColorDesc")}
                </p>
              </div>
              <div class="flex items-center gap-3">
                <input
                  id="secondaryColor"
                  type="color"
                  bind:value={secondaryColor}
                  class="h-9 w-12 cursor-pointer rounded border border-input bg-transparent p-1"
                />
                <Input
                  bind:value={secondaryColor}
                  aria-label={$_("appearance.secondaryColorLabel")}
                  class="w-28 font-mono text-xs uppercase"
                  maxlength={7}
                />
              </div>
            </div>

            <div
              class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
            >
              <div>
                <label
                  for="accentColor"
                  class="text-sm font-medium text-foreground"
                >
                  {$_("appearance.accentColorLabel")}
                </label>
                <p class="text-xs text-muted-foreground">
                  {$_("appearance.accentColorDesc")}
                </p>
              </div>
              <div class="flex items-center gap-3">
                <input
                  id="accentColor"
                  type="color"
                  bind:value={accentColor}
                  class="h-9 w-12 cursor-pointer rounded border border-input bg-transparent p-1"
                />
                <Input
                  bind:value={accentColor}
                  aria-label={$_("appearance.accentColorLabel")}
                  class="w-28 font-mono text-xs uppercase"
                  maxlength={7}
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2 text-base">
              <Upload class="h-4 w-4 text-primary" />
              {$_("appearance.logoLabel")}
            </CardTitle>
            <CardDescription>{$_("appearance.logoDesc")}</CardDescription>
          </CardHeader>
          <CardContent>
            <div class="flex flex-col sm:flex-row items-center gap-6">
              <div
                class="flex h-24 w-24 shrink-0 items-center justify-center rounded-[var(--radius)] border border-dashed border-border bg-muted/40 p-2"
              >
                {#if logoUrl}
                  <img
                    src={logoUrl}
                    alt={$_("appearance.logoAlt")}
                    class="max-h-full max-w-full object-contain"
                  />
                {:else}
                  <Building2 class="h-8 w-8 text-muted-foreground/50" />
                {/if}
              </div>
              <div class="flex flex-wrap items-center gap-3">
                <label class="cursor-pointer">
                  <span
                    class="inline-flex items-center justify-center gap-2 rounded-[var(--radius)] bg-secondary px-4 py-2 text-sm font-medium text-secondary-foreground shadow-xs hover:opacity-85 transition-opacity"
                  >
                    <Upload class="h-4 w-4" />
                    {$_("appearance.logoUploadBtn")}
                  </span>
                  <input
                    type="file"
                    accept="image/png, image/jpeg, image/svg+xml"
                    class="sr-only"
                    onchange={handleLogoUpload}
                  />
                </label>
                {#if logoUrl}
                  <Button
                    variant="outline"
                    size="sm"
                    onclick={handleRemoveLogo}
                  >
                    <Trash2 class="h-4 w-4 text-destructive" />
                    {$_("appearance.logoRemoveBtn")}
                  </Button>
                {/if}
              </div>
            </div>
          </CardContent>
        </Card>

        <div class="flex items-center justify-between pt-2">
          <Button
            variant="ghost"
            size="sm"
            onclick={handleResetDefaults}
            disabled={!canEdit}
          >
            <RotateCcw class="h-4 w-4" />
            {$_("appearance.resetBtn")}
          </Button>

          <Button onclick={handleSave} disabled={isSaving || !canEdit}>
            <CheckCircle2 class="h-4 w-4" />
            {isSaving ? $_("appearance.saving") : $_("appearance.saveBtn")}
          </Button>
        </div>
      </div>

      <div class="space-y-6">
        <Card class="sticky top-6">
          <CardHeader>
            <CardTitle class="text-base"
              >{$_("appearance.previewTitle")}</CardTitle
            >
            <CardDescription>{$_("appearance.previewDesc")}</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div
              class="flex items-center gap-3 rounded-[var(--radius)] border border-border bg-card p-3 shadow-2xs"
            >
              {#if logoUrl}
                <img
                  src={logoUrl}
                  alt={$_("appearance.logoAlt")}
                  class="h-8 w-8 object-contain rounded-xs"
                />
              {:else}
                <div
                  class="flex h-8 w-8 items-center justify-center rounded-sm bg-primary text-primary-foreground font-bold text-xs"
                >
                  {companyName.slice(0, 2).toUpperCase()}
                </div>
              {/if}
              <div class="truncate text-sm font-semibold">{companyName}</div>
            </div>

            <div class="space-y-2">
              <span class="text-xs font-medium text-muted-foreground"
                >{$_("appearance.previewButtonsLabel")}</span
              >
              <div class="flex flex-col gap-2">
                <Button variant="default"
                  >{$_("appearance.previewBtnPrimary")}</Button
                >
                <Button variant="secondary"
                  >{$_("appearance.previewBtnSecondary")}</Button
                >
                <Button variant="outline"
                  >{$_("appearance.previewBtnOutline")}</Button
                >
              </div>
            </div>

            <div class="space-y-2">
              <span class="text-xs font-medium text-muted-foreground"
                >{$_("appearance.previewBadgesLabel")}</span
              >
              <div class="flex flex-wrap gap-2">
                <Badge variant="default"
                  >{$_("appearance.previewBadgePrimary")}</Badge
                >
                <Badge variant="secondary"
                  >{$_("appearance.previewBadgeSecondary")}</Badge
                >
                <Badge variant="outline"
                  >{$_("appearance.previewBadgeAccent")}</Badge
                >
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
{/if}
