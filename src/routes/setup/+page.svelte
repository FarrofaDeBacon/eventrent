<script lang="ts">
  import { goto } from "$app/navigation";
  import { toast } from "svelte-sonner";
  import {
    HardDrive,
    Cloud,
    ShieldAlert,
    KeyRound,
    Copy,
    CheckCircle2,
    Building2,
    ArrowRight,
  } from "lucide-svelte";
  import {
    appSession,
    getAuthRepository,
    type InitialSetupResult,
    type OperationMode,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/ui/card";
  import { Input } from "$lib/ui/input";

  let selectedMode = $state<OperationMode>("local");
  let companyName = $state("");
  let ownerName = $state("");
  let ownerEmail = $state("");
  let ownerPassword = $state("");
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);

  let setupResult = $state<InitialSetupResult | null>(null);
  let confirmedRecoveryKey = $state(false);

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    errorMessage = null;
    isSubmitting = true;

    try {
      const repo = getAuthRepository();
      const result = await repo.completeInitialSetup({
        mode: selectedMode,
        company_name: companyName,
        owner_name: ownerName,
        owner_email: ownerEmail,
        owner_password: ownerPassword,
      });

      setupResult = result;
      appSession.setBranding(result.branding);
      toast.success($_("setup.successToast"));
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
      toast.error(errorMessage);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleCopyRecoveryKey() {
    if (!setupResult) return;
    try {
      await navigator.clipboard.writeText(setupResult.recovery_key);
      toast.success($_("setup.copiedToast"));
    } catch {
      toast.info(setupResult.recovery_key);
    }
  }

  async function handleEnterSystem() {
    if (!setupResult || !confirmedRecoveryKey) return;
    appSession.setSession(setupResult.session);
    await goto("/");
  }
</script>

<div
  class="mx-auto flex min-h-screen max-w-3xl flex-col justify-center px-6 py-12"
>
  {#if !setupResult}
    <div class="mb-8 space-y-2 text-center">
      <div class="flex justify-center">
        <Badge variant="default">{$_("setup.badge")}</Badge>
      </div>
      <h1 class="text-3xl font-bold tracking-tight text-foreground">
        {$_("setup.title")}
      </h1>
      <p class="text-sm text-muted-foreground">
        {$_("setup.subtitle")}
      </p>
    </div>

    <form onsubmit={handleSubmit} class="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle class="text-base">{$_("setup.modeSectionTitle")}</CardTitle
          >
          <CardDescription>{$_("setup.modeSectionDesc")}</CardDescription>
        </CardHeader>
        <CardContent class="grid gap-4 sm:grid-cols-2">
          <button
            type="button"
            onclick={() => (selectedMode = "local")}
            data-testid="mode-local-card"
            class="flex flex-col items-start gap-3 rounded-[var(--radius)] border-2 p-4 text-left transition-all cursor-pointer {selectedMode ===
            'local'
              ? 'border-primary bg-primary/5 shadow-xs'
              : 'border-border bg-card hover:bg-muted/50'}"
          >
            <div class="flex w-full items-center justify-between">
              <div
                class="flex h-10 w-10 items-center justify-center rounded-[var(--radius)] bg-primary text-primary-foreground"
              >
                <HardDrive class="h-5 w-5" />
              </div>
              <Badge variant="default">{$_("setup.modeLocalBadge")}</Badge>
            </div>
            <div class="space-y-1">
              <div class="font-semibold text-foreground">
                {$_("setup.modeLocalTitle")}
              </div>
              <p class="text-xs text-muted-foreground leading-relaxed">
                {$_("setup.modeLocalDesc")}
              </p>
            </div>
          </button>

          <div
            data-testid="mode-cloud-card"
            aria-disabled="true"
            class="flex flex-col items-start gap-3 rounded-[var(--radius)] border border-dashed border-border bg-muted/30 p-4 text-left opacity-65 select-none cursor-not-allowed"
          >
            <div class="flex w-full items-center justify-between">
              <div
                class="flex h-10 w-10 items-center justify-center rounded-[var(--radius)] bg-muted text-muted-foreground"
              >
                <Cloud class="h-5 w-5" />
              </div>
              <Badge variant="secondary">{$_("setup.modeCloudBadge")}</Badge>
            </div>
            <div class="space-y-1">
              <div class="font-semibold text-foreground">
                {$_("setup.modeCloudTitle")}
              </div>
              <p class="text-xs text-muted-foreground leading-relaxed">
                {$_("setup.modeCloudDesc")}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2 text-base">
            <Building2 class="h-4 w-4 text-primary" />
            {$_("setup.ownerSectionTitle")}
          </CardTitle>
          <CardDescription>{$_("setup.ownerSectionDesc")}</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          {#if errorMessage}
            <div
              role="alert"
              class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-4 py-3 text-xs font-medium text-destructive"
            >
              {errorMessage}
            </div>
          {/if}

          <div class="space-y-2">
            <label
              for="setup-company-name"
              class="text-sm font-medium text-foreground"
            >
              {$_("setup.companyNameLabel")}
            </label>
            <Input
              id="setup-company-name"
              bind:value={companyName}
              placeholder={$_("setup.companyNamePlaceholder")}
              required
            />
          </div>

          <div class="grid gap-4 sm:grid-cols-2">
            <div class="space-y-2">
              <label
                for="setup-owner-name"
                class="text-sm font-medium text-foreground"
              >
                {$_("setup.ownerNameLabel")}
              </label>
              <Input
                id="setup-owner-name"
                bind:value={ownerName}
                placeholder={$_("setup.ownerNamePlaceholder")}
                required
              />
            </div>

            <div class="space-y-2">
              <label
                for="setup-owner-email"
                class="text-sm font-medium text-foreground"
              >
                {$_("setup.ownerEmailLabel")}
              </label>
              <Input
                id="setup-owner-email"
                type="email"
                bind:value={ownerEmail}
                placeholder={$_("setup.ownerEmailPlaceholder")}
                required
              />
            </div>
          </div>

          <div class="space-y-2">
            <label
              for="setup-owner-password"
              class="text-sm font-medium text-foreground"
            >
              {$_("setup.ownerPasswordLabel")}
            </label>
            <Input
              id="setup-owner-password"
              type="password"
              bind:value={ownerPassword}
              placeholder={$_("setup.ownerPasswordPlaceholder")}
              minlength={8}
              required
            />
          </div>

          <div class="pt-2 flex justify-end">
            <Button type="submit" disabled={isSubmitting}>
              <CheckCircle2 class="h-4 w-4" />
              {isSubmitting ? $_("setup.submitting") : $_("setup.submitBtn")}
            </Button>
          </div>
        </CardContent>
      </Card>
    </form>
  {:else}
    <Card>
      <CardHeader>
        <div class="flex items-center gap-2 text-warning mb-1">
          <ShieldAlert class="h-5 w-5" />
          <span class="text-xs font-semibold uppercase tracking-wider">
            {$_("setup.recoveryWarning")}
          </span>
        </div>
        <CardTitle class="text-2xl">{$_("setup.recoveryTitle")}</CardTitle>
        <CardDescription>{$_("setup.recoverySubtitle")}</CardDescription>
      </CardHeader>
      <CardContent class="space-y-6">
        <div class="space-y-2">
          <span class="text-xs font-medium text-muted-foreground">
            {$_("setup.recoveryKeyLabel")}
          </span>
          <div
            class="flex flex-col sm:flex-row items-center justify-between gap-3 rounded-[var(--radius)] border border-primary/40 bg-primary/5 p-4"
          >
            <div class="flex items-center gap-3">
              <KeyRound class="h-5 w-5 text-primary shrink-0" />
              <code
                data-testid="recovery-key-value"
                class="font-mono text-lg font-bold tracking-wider text-foreground select-all"
              >
                {setupResult.recovery_key}
              </code>
            </div>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onclick={handleCopyRecoveryKey}
            >
              <Copy class="h-4 w-4" />
              {$_("setup.copyKeyBtn")}
            </Button>
          </div>
        </div>

        <label
          class="flex items-center gap-3 rounded-[var(--radius)] border border-border bg-muted/40 p-3.5 text-sm font-medium text-foreground cursor-pointer"
        >
          <input
            id="confirm-recovery-key"
            type="checkbox"
            bind:checked={confirmedRecoveryKey}
            class="h-4 w-4 rounded border-input accent-primary"
          />
          <span>{$_("setup.confirmSavedLabel")}</span>
        </label>

        <div class="flex justify-end">
          <Button
            type="button"
            onclick={handleEnterSystem}
            disabled={!confirmedRecoveryKey}
          >
            <span>{$_("setup.enterSystemBtn")}</span>
            <ArrowRight class="h-4 w-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  {/if}
</div>
