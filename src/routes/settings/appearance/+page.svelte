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
    AlertTriangle,
    Sun,
    Moon,
    Image as ImageIcon,
    ShieldAlert,
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
    DEFAULT_BRANDING_COLORS,
    THEME_PRESETS,
    type ThemePreset,
  } from "$lib/theme/tokens";
  import { getAccessibleForeground } from "$lib/theme/color";
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
    hasPermission(appSession.session, PERMISSIONS.BRANDING_MANAGE),
  );

  let companyName = $state(DEFAULT_COMPANY_NAME);
  let primaryColor = $state(DEFAULT_BRANDING_COLORS.primary as string);
  let secondaryColor = $state(DEFAULT_BRANDING_COLORS.secondary as string);
  let accentColor = $state(DEFAULT_BRANDING_COLORS.accent as string);

  let logoUrl = $state<string | null>(null);
  let logoDarkUrl = $state<string | null>(null);
  let iconUrl = $state<string | null>(null);

  let logoSizeKb = $state<number | null>(null);
  let logoDarkSizeKb = $state<number | null>(null);
  let iconSizeKb = $state<number | null>(null);

  let previewDarkMode = $state(false);

  // Análise de contraste acessível em tempo real (WCAG 2.1 AA >= 4.5:1)
  let contrastAnalysis = $derived(getAccessibleForeground(primaryColor));

  onMount(async () => {
    try {
      const repo = getBrandingRepository();
      const data = await repo.getBranding(appSession.defaultTenantId);
      if (data) {
        companyName = data.company_name;
        primaryColor = data.primary_color;
        secondaryColor = data.secondary_color;
        accentColor = data.accent_color;
        logoUrl = data.logo_url ?? null;
        logoDarkUrl = data.logo_dark_url ?? null;
        iconUrl = data.icon_url ?? null;

        if (logoUrl) {
          logoSizeKb = Math.round(((logoUrl.length * 3) / 4 / 1024) * 10) / 10;
        }
        if (logoDarkUrl) {
          logoDarkSizeKb =
            Math.round(((logoDarkUrl.length * 3) / 4 / 1024) * 10) / 10;
        }
        if (iconUrl) {
          iconSizeKb = Math.round(((iconUrl.length * 3) / 4 / 1024) * 10) / 10;
        }
      }
    } catch (err) {
      console.error("Erro ao carregar dados de branding:", err);
      toast.error($_("appearance.saveError"));
    } finally {
      isLoading = false;
    }
  });

  $effect(() => {
    // Atualiza em tempo real as variáveis CSS para que a interface reaja aos ajustes
    applyBranding({
      company_name: companyName,
      primary_color: primaryColor,
      secondary_color: secondaryColor,
      accent_color: accentColor,
      logo_url: logoUrl,
      logo_dark_url: logoDarkUrl,
      icon_url: iconUrl,
    });
  });

  function processImageFile(
    file: File,
    isIcon: boolean,
    onSuccess: (dataUrl: string, sizeKb: number) => void,
  ) {
    if (
      file.type === "image/svg+xml" ||
      file.name.toLowerCase().endsWith(".svg")
    ) {
      toast.error(
        "Imagens no formato SVG não são permitidas por motivos de segurança. Utilize PNG, JPEG ou WebP.",
      );
      return;
    }

    if (
      file.type !== "image/png" &&
      file.type !== "image/jpeg" &&
      file.type !== "image/webp"
    ) {
      toast.error("Formato não suportado. Utilize PNG, JPEG ou WebP.");
      return;
    }

    const reader = new FileReader();
    reader.onload = (e) => {
      const rawDataUrl = e.target?.result as string;
      const img = new Image();
      img.onload = () => {
        const maxDim = isIcon ? 128 : 512;
        let width = img.width;
        let height = img.height;

        const canvas = document.createElement("canvas");
        if (isIcon) {
          const size = Math.min(width, height);
          canvas.width = Math.min(size, maxDim);
          canvas.height = Math.min(size, maxDim);
          const ctx = canvas.getContext("2d");
          if (!ctx) return;
          const sx = (width - size) / 2;
          const sy = (height - size) / 2;
          ctx.drawImage(
            img,
            sx,
            sy,
            size,
            size,
            0,
            0,
            canvas.width,
            canvas.height,
          );
        } else {
          if (width > maxDim || height > maxDim) {
            if (width > height) {
              height = Math.round((height * maxDim) / width);
              width = maxDim;
            } else {
              width = Math.round((width * maxDim) / height);
              height = maxDim;
            }
          }
          canvas.width = width;
          canvas.height = height;
          const ctx = canvas.getContext("2d");
          if (!ctx) return;
          ctx.drawImage(img, 0, 0, width, height);
        }

        const mime = file.type === "image/jpeg" ? "image/jpeg" : "image/png";
        const resizedDataUrl = canvas.toDataURL(mime, 0.9);
        const approxBytes = Math.round((resizedDataUrl.length * 3) / 4);
        const sizeKb = Math.round((approxBytes / 1024) * 10) / 10;

        const maxLimitKb = isIcon ? 64 : 256;
        if (sizeKb > maxLimitKb) {
          toast.error(
            `A imagem excede o limite máximo permitido de ${maxLimitKb} KB (tamanho: ${sizeKb} KB).`,
          );
          return;
        }

        onSuccess(resizedDataUrl, sizeKb);
      };
      img.src = rawDataUrl;
    };
    reader.readAsDataURL(file);
  }

  function handleLogoUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      processImageFile(input.files[0], false, (dataUrl, sizeKb) => {
        logoUrl = dataUrl;
        logoSizeKb = sizeKb;
      });
    }
  }

  function handleLogoDarkUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      processImageFile(input.files[0], false, (dataUrl, sizeKb) => {
        logoDarkUrl = dataUrl;
        logoDarkSizeKb = sizeKb;
      });
    }
  }

  function handleIconUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      processImageFile(input.files[0], true, (dataUrl, sizeKb) => {
        iconUrl = dataUrl;
        iconSizeKb = sizeKb;
      });
    }
  }

  function applyPreset(preset: ThemePreset) {
    if (!canEdit) return;
    primaryColor = preset.primary;
    secondaryColor = preset.secondary;
    accentColor = preset.accent;
  }

  function handleResetDefaults() {
    if (!canEdit) return;
    companyName = $_("app.defaultCompanyName");
    primaryColor = DEFAULT_BRANDING_COLORS.primary;
    secondaryColor = DEFAULT_BRANDING_COLORS.secondary;
    accentColor = DEFAULT_BRANDING_COLORS.accent;
    logoUrl = null;
    logoDarkUrl = null;
    iconUrl = null;
    logoSizeKb = null;
    logoDarkSizeKb = null;
    iconSizeKb = null;

    appSession.setBranding(
      {
        company_name: companyName,
        primary_color: primaryColor,
        secondary_color: secondaryColor,
        accent_color: accentColor,
        logo_url: null,
        logo_dark_url: null,
        icon_url: null,
      },
      appSession.defaultTenantId,
    );

    appSession.setBranding(
      {
        company_name: companyName,
        primary_color: primaryColor,
        secondary_color: secondaryColor,
        accent_color: accentColor,
        logo_url: null,
        logo_dark_url: null,
        icon_url: null,
      },
      appSession.defaultTenantId,
    );
  }

  async function handleSave() {
    if (!canEdit) return;
    if (!companyName.trim()) {
      toast.error($_("appearance.companyNamePlaceholder"));
      return;
    }

    isSaving = true;
    try {
      const repo = getBrandingRepository();
      const updated = await repo.saveBranding(appSession.defaultTenantId, {
        company_name: companyName,
        primary_color: primaryColor,
        secondary_color: secondaryColor,
        accent_color: accentColor,
        logo_url: logoUrl,
        logo_dark_url: logoDarkUrl,
        icon_url: iconUrl,
      });

      applyBranding(updated);
      appSession.setBranding(updated, appSession.defaultTenantId);
      toast.success($_("appearance.saveSuccess"));
    } catch (err) {
      console.error("Falha ao salvar identidade visual:", err);
      toast.error($_("appearance.saveError"));
    } finally {
      isSaving = false;
    }
  }
</script>

{#if isLoading}
  <LoadingState message={$_("appearance.loading")} />
{:else}
  <div class="space-y-6">
    <div
      class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4"
    >
      <div>
        <h2 class="text-2xl font-bold tracking-tight text-foreground">
          {$_("appearance.title")}
        </h2>
        <p class="text-sm text-muted-foreground mt-1">
          {$_("appearance.description")}
        </p>
      </div>

      {#if !canEdit}
        <div
          class="flex items-center gap-2 rounded-md bg-warning/15 px-3 py-1.5 text-xs text-warning-foreground border border-warning/30"
        >
          <ShieldAlert class="h-4 w-4 shrink-0" />
          <span>{$_("appearance.permissionWarning")}</span>
        </div>
      {/if}
    </div>

    <div class="grid gap-6 lg:grid-cols-3">
      <div class="lg:col-span-2 space-y-6">
        <!-- Identificação da Empresa -->
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
                disabled={!canEdit}
                maxlength={80}
              />
            </div>
          </CardContent>
        </Card>

        <!-- Temas Predefinidos -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2 text-base">
              <Palette class="h-4 w-4 text-primary" />
              {$_("appearance.presetsTitle")}
            </CardTitle>
            <CardDescription>{$_("appearance.presetsDesc")}</CardDescription>
          </CardHeader>
          <CardContent>
            <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3">
              {#each THEME_PRESETS as preset (preset.id)}
                <button
                  type="button"
                  class="flex items-center justify-between rounded-lg border border-border p-3 text-left transition-all hover:bg-muted/50 focus:outline-hidden focus:ring-2 focus:ring-ring disabled:opacity-50"
                  class:border-primary={primaryColor.toLowerCase() ===
                    preset.primary.toLowerCase()}
                  class:bg-muted={primaryColor.toLowerCase() ===
                    preset.primary.toLowerCase()}
                  onclick={() => applyPreset(preset)}
                  disabled={!canEdit}
                >
                  <span class="text-xs font-semibold text-foreground truncate"
                    >{preset.name}</span
                  >
                  <div class="flex items-center gap-1.5 shrink-0 ml-2">
                    <span
                      class="h-3.5 w-3.5 rounded-full border border-border shadow-2xs"
                      style="background-color: {preset.primary}"
                    ></span>
                    <span
                      class="h-3.5 w-3.5 rounded-full border border-border shadow-2xs"
                      style="background-color: {preset.secondary}"
                    ></span>
                    <span
                      class="h-3.5 w-3.5 rounded-full border border-border shadow-2xs"
                      style="background-color: {preset.accent}"
                    ></span>
                  </div>
                </button>
              {/each}
            </div>
          </CardContent>
        </Card>

        <!-- Cores Personalizadas -->
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
            <!-- Aviso de Contraste WCAG se < 4.5:1 -->
            {#if !contrastAnalysis.isAccessible}
              <div
                class="flex items-start gap-3 rounded-md bg-warning/15 p-3 text-xs text-warning-foreground border border-warning/30"
              >
                <AlertTriangle class="h-4 w-4 shrink-0 mt-0.5" />
                <div>
                  <span class="font-semibold"
                    >{$_("appearance.contrastWarning", {
                      values: { ratio: contrastAnalysis.contrast },
                    })}</span
                  >
                </div>
              </div>
            {/if}

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
                  disabled={!canEdit}
                  class="h-9 w-12 cursor-pointer rounded border border-input bg-transparent p-1 disabled:opacity-50"
                />
                <Input
                  bind:value={primaryColor}
                  aria-label={$_("appearance.primaryColorLabel")}
                  class="w-28 font-mono text-xs uppercase"
                  maxlength={7}
                  disabled={!canEdit}
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
                  disabled={!canEdit}
                  class="h-9 w-12 cursor-pointer rounded border border-input bg-transparent p-1 disabled:opacity-50"
                />
                <Input
                  bind:value={secondaryColor}
                  aria-label={$_("appearance.secondaryColorLabel")}
                  class="w-28 font-mono text-xs uppercase"
                  maxlength={7}
                  disabled={!canEdit}
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
                  disabled={!canEdit}
                  class="h-9 w-12 cursor-pointer rounded border border-input bg-transparent p-1 disabled:opacity-50"
                />
                <Input
                  bind:value={accentColor}
                  aria-label={$_("appearance.accentColorLabel")}
                  class="w-28 font-mono text-xs uppercase"
                  maxlength={7}
                  disabled={!canEdit}
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Logotipos e Imagens da Marca -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2 text-base">
              <ImageIcon class="h-4 w-4 text-primary" />
              {$_("appearance.logosSectionTitle")}
            </CardTitle>
            <CardDescription
              >{$_("appearance.logosSectionDesc")}</CardDescription
            >
          </CardHeader>
          <CardContent class="space-y-6">
            <!-- 1. Logo Claro -->
            <div
              class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-border pb-5"
            >
              <div class="space-y-1">
                <span class="text-sm font-medium text-foreground"
                  >{$_("appearance.logoLabel")}</span
                >
                <p class="text-xs text-muted-foreground">
                  {$_("appearance.logoDesc")}
                  {#if logoSizeKb !== null}
                    <span class="font-mono text-foreground font-semibold"
                      >({logoSizeKb} KB / 256 KB)</span
                    >
                  {/if}
                </p>
              </div>
              <div class="flex items-center gap-3">
                <div
                  class="flex h-16 w-28 shrink-0 items-center justify-center rounded-md border border-dashed border-border bg-card p-1 shadow-2xs"
                >
                  {#if logoUrl}
                    <img
                      src={logoUrl}
                      alt={$_("appearance.logoAlt")}
                      class="max-h-full max-w-full object-contain"
                    />
                  {:else}
                    <Building2 class="h-6 w-6 text-muted-foreground/40" />
                  {/if}
                </div>
                <div class="flex flex-col gap-2">
                  <label class="cursor-pointer" class:opacity-50={!canEdit}>
                    <span
                      class="inline-flex items-center justify-center gap-1.5 rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground shadow-2xs hover:bg-secondary/80 transition-colors"
                    >
                      <Upload class="h-3.5 w-3.5" />
                      {$_("appearance.logoUploadBtn")}
                    </span>
                    <input
                      type="file"
                      accept="image/png,image/jpeg,image/webp"
                      class="sr-only"
                      onchange={handleLogoUpload}
                      disabled={!canEdit}
                    />
                  </label>
                  {#if logoUrl}
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 px-2 text-xs text-destructive hover:bg-destructive/10"
                      onclick={() => {
                        logoUrl = null;
                        logoSizeKb = null;
                      }}
                      disabled={!canEdit}
                    >
                      <Trash2 class="h-3.5 w-3.5 mr-1" />
                      {$_("appearance.logoRemoveBtn")}
                    </Button>
                  {/if}
                </div>
              </div>
            </div>

            <!-- 2. Logo Escuro -->
            <div
              class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-border pb-5"
            >
              <div class="space-y-1">
                <span class="text-sm font-medium text-foreground"
                  >{$_("appearance.logoDarkLabel")}</span
                >
                <p class="text-xs text-muted-foreground">
                  {$_("appearance.logoDarkDesc")}
                  {#if logoDarkSizeKb !== null}
                    <span class="font-mono text-foreground font-semibold"
                      >({logoDarkSizeKb} KB / 256 KB)</span
                    >
                  {/if}
                </p>
              </div>
              <div class="flex items-center gap-3">
                <div
                  class="flex h-16 w-28 shrink-0 items-center justify-center rounded-md border border-dashed border-border bg-foreground/90 p-1 shadow-2xs"
                >
                  {#if logoDarkUrl}
                    <img
                      src={logoDarkUrl}
                      alt={$_("appearance.logoDarkAlt")}
                      class="max-h-full max-w-full object-contain"
                    />
                  {:else}
                    <Building2 class="h-6 w-6 text-background/50" />
                  {/if}
                </div>
                <div class="flex flex-col gap-2">
                  <label class="cursor-pointer" class:opacity-50={!canEdit}>
                    <span
                      class="inline-flex items-center justify-center gap-1.5 rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground shadow-2xs hover:bg-secondary/80 transition-colors"
                    >
                      <Upload class="h-3.5 w-3.5" />
                      {$_("appearance.logoUploadBtn")}
                    </span>
                    <input
                      type="file"
                      accept="image/png,image/jpeg,image/webp"
                      class="sr-only"
                      onchange={handleLogoDarkUpload}
                      disabled={!canEdit}
                    />
                  </label>
                  {#if logoDarkUrl}
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 px-2 text-xs text-destructive hover:bg-destructive/10"
                      onclick={() => {
                        logoDarkUrl = null;
                        logoDarkSizeKb = null;
                      }}
                      disabled={!canEdit}
                    >
                      <Trash2 class="h-3.5 w-3.5 mr-1" />
                      {$_("appearance.logoRemoveBtn")}
                    </Button>
                  {/if}
                </div>
              </div>
            </div>

            <!-- 3. Ícone Quadrado -->
            <div
              class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
            >
              <div class="space-y-1">
                <span class="text-sm font-medium text-foreground"
                  >{$_("appearance.iconLabel")}</span
                >
                <p class="text-xs text-muted-foreground">
                  {$_("appearance.iconDesc")}
                  {#if iconSizeKb !== null}
                    <span class="font-mono text-foreground font-semibold"
                      >({iconSizeKb} KB / 64 KB)</span
                    >
                  {/if}
                </p>
              </div>
              <div class="flex items-center gap-3">
                <div
                  class="flex h-14 w-14 shrink-0 items-center justify-center rounded-md border border-dashed border-border bg-card p-1 shadow-2xs"
                >
                  {#if iconUrl}
                    <img
                      src={iconUrl}
                      alt={$_("appearance.iconAlt")}
                      class="h-full w-full object-cover rounded-xs"
                    />
                  {:else}
                    <div
                      class="flex h-full w-full items-center justify-center rounded-xs bg-primary text-primary-foreground font-bold text-xs"
                    >
                      {companyName.slice(0, 2).toUpperCase()}
                    </div>
                  {/if}
                </div>
                <div class="flex flex-col gap-2">
                  <label class="cursor-pointer" class:opacity-50={!canEdit}>
                    <span
                      class="inline-flex items-center justify-center gap-1.5 rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground shadow-2xs hover:bg-secondary/80 transition-colors"
                    >
                      <Upload class="h-3.5 w-3.5" />
                      {$_("appearance.logoUploadBtn")}
                    </span>
                    <input
                      type="file"
                      accept="image/png,image/jpeg,image/webp"
                      class="sr-only"
                      onchange={handleIconUpload}
                      disabled={!canEdit}
                    />
                  </label>
                  {#if iconUrl}
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 px-2 text-xs text-destructive hover:bg-destructive/10"
                      onclick={() => {
                        iconUrl = null;
                        iconSizeKb = null;
                      }}
                      disabled={!canEdit}
                    >
                      <Trash2 class="h-3.5 w-3.5 mr-1" />
                      {$_("appearance.logoRemoveBtn")}
                    </Button>
                  {/if}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Ações do Formulário -->
        <div class="flex items-center justify-between pt-2">
          <Button
            variant="ghost"
            size="sm"
            onclick={handleResetDefaults}
            disabled={!canEdit}
          >
            <RotateCcw class="h-4 w-4 mr-1.5" />
            {$_("appearance.resetBtn")}
          </Button>

          <Button onclick={handleSave} disabled={isSaving || !canEdit}>
            <CheckCircle2 class="h-4 w-4 mr-1.5" />
            {isSaving ? $_("appearance.saving") : $_("appearance.saveBtn")}
          </Button>
        </div>
      </div>

      <!-- Coluna da Direita: Pré-visualização em Tempo Real com Toggle Claro / Escuro -->
      <div class="space-y-6">
        <Card class="sticky top-6">
          <CardHeader>
            <div class="flex items-center justify-between">
              <CardTitle class="text-base"
                >{$_("appearance.previewTitle")}</CardTitle
              >
              <div
                class="flex items-center rounded-lg border border-border p-0.5 bg-muted/60"
              >
                <button
                  type="button"
                  class="flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors"
                  class:bg-card={!previewDarkMode}
                  class:text-foreground={!previewDarkMode}
                  class:shadow-2xs={!previewDarkMode}
                  class:text-muted-foreground={previewDarkMode}
                  onclick={() => (previewDarkMode = false)}
                >
                  <Sun class="h-3 w-3" />
                  <span>{$_("appearance.previewLightMode")}</span>
                </button>
                <button
                  type="button"
                  class="flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors"
                  class:bg-card={previewDarkMode}
                  class:text-foreground={previewDarkMode}
                  class:shadow-2xs={previewDarkMode}
                  class:text-muted-foreground={!previewDarkMode}
                  onclick={() => (previewDarkMode = true)}
                >
                  <Moon class="h-3 w-3" />
                  <span>{$_("appearance.previewDarkMode")}</span>
                </button>
              </div>
            </div>
            <CardDescription>{$_("appearance.previewDesc")}</CardDescription>
          </CardHeader>

          <CardContent class="space-y-6">
            <!-- Container Mock com classe .dark dinâmica -->
            <div
              class="rounded-xl border border-border p-4 transition-colors duration-200"
              class:dark={previewDarkMode}
              class:bg-background={true}
              class:text-foreground={true}
            >
              <!-- 1. Cabeçalho / Barra de Aplicação -->
              <div
                class="flex items-center justify-between border-b border-border pb-3 mb-4"
              >
                <div class="flex items-center gap-2.5">
                  {#if previewDarkMode && logoDarkUrl}
                    <img
                      src={logoDarkUrl}
                      alt={$_("appearance.logoDarkAlt")}
                      class="h-7 max-w-28 object-contain"
                    />
                  {:else if logoUrl}
                    <img
                      src={logoUrl}
                      alt={$_("appearance.logoAlt")}
                      class="h-7 max-w-28 object-contain"
                    />
                  {:else if iconUrl}
                    <img
                      src={iconUrl}
                      alt={$_("appearance.iconAlt")}
                      class="h-7 w-7 object-cover rounded-sm"
                    />
                  {:else}
                    <div
                      class="flex h-7 w-7 items-center justify-center rounded-sm bg-primary text-primary-foreground font-bold text-xs"
                    >
                      {companyName.slice(0, 2).toUpperCase()}
                    </div>
                  {/if}
                  <span class="text-sm font-semibold truncate max-w-32"
                    >{companyName}</span
                  >
                </div>
                <Badge variant="outline" class="text-2xs font-mono"
                  >v0.3.1</Badge
                >
              </div>

              <!-- 2. Botões Semânticos -->
              <div class="space-y-2 mb-4">
                <span
                  class="text-2xs font-medium text-muted-foreground uppercase tracking-wider"
                  >{$_("appearance.previewButtonsLabel")}</span
                >
                <div class="flex flex-col gap-2">
                  <Button variant="default" size="sm" class="w-full">
                    {$_("appearance.previewBtnPrimary")}
                  </Button>
                  <Button variant="secondary" size="sm" class="w-full">
                    {$_("appearance.previewBtnSecondary")}
                  </Button>
                  <Button variant="outline" size="sm" class="w-full">
                    {$_("appearance.previewBtnOutline")}
                  </Button>
                </div>
              </div>

              <!-- 3. Badges e Tags -->
              <div class="space-y-2 mb-4">
                <span
                  class="text-2xs font-medium text-muted-foreground uppercase tracking-wider"
                  >{$_("appearance.previewBadgesLabel")}</span
                >
                <div class="flex flex-wrap gap-1.5">
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

              <!-- 4. Mini Card de Item com Preço -->
              <div
                class="rounded-lg border border-border bg-card p-3 shadow-2xs space-y-2"
              >
                <div class="flex items-center justify-between">
                  <span class="text-xs font-semibold"
                    >{$_("appearance.previewCardTitle")}</span
                  >
                  <span class="text-xs font-bold text-primary">R$ 150,00</span>
                </div>
                <p class="text-2xs text-muted-foreground">
                  {$_("appearance.previewCardDesc")}
                </p>
                <div class="pt-1">
                  <Button size="sm" class="w-full text-xs h-7">
                    Reservar Equipamento
                  </Button>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
{/if}
