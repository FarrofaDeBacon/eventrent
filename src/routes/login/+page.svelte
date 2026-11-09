<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { toast } from "svelte-sonner";
  import { KeyRound, LogIn } from "lucide-svelte";
  import { mode } from "mode-watcher";
  import {
    appSession,
    getAuthRepository,
    getBrandingRepository,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { Button } from "$lib/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/ui/card";
  import { Dialog } from "$lib/ui/dialog";
  import { Input } from "$lib/ui/input";

  let email = $state("");
  let password = $state("");
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);

  let recoveryOpen = $state(false);
  let recoveryEmail = $state("");
  let recoveryKey = $state("");
  let recoveryNewPassword = $state("");
  let isRecovering = $state(false);
  let recoveryError = $state<string | null>(null);

  async function handleLogin(event: SubmitEvent) {
    event.preventDefault();
    errorMessage = null;
    isSubmitting = true;

    try {
      const repo = getAuthRepository();
      const session = await repo.login({ email, password });
      appSession.setSession(session);
      toast.success($_("login.welcomeToast"));
      await goto("/");
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
      toast.error(errorMessage);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleRecovery(event: SubmitEvent) {
    event.preventDefault();
    recoveryError = null;
    isRecovering = true;

    try {
      const repo = getAuthRepository();
      await repo.recoverPasswordWithKey({
        email: recoveryEmail,
        recovery_key: recoveryKey,
        new_password: recoveryNewPassword,
      });

      recoveryOpen = false;
      email = recoveryEmail;
      password = "";
      recoveryKey = "";
      recoveryNewPassword = "";
      toast.success($_("login.recoverySuccessToast"));
    } catch (err) {
      recoveryError = err instanceof Error ? err.message : String(err);
      toast.error(recoveryError);
    } finally {
      isRecovering = false;
    }
  }

  onMount(async () => {
    try {
      const repo = getBrandingRepository();
      const pub = await repo.getPublicBranding();
      if (pub) {
        appSession.setBranding({
          ...pub,
          logo_dark_url: pub.logo_dark_url ?? null,
          icon_url: pub.icon_url ?? null,
        });
      }
    } catch {
      // Ignora erro no carregamento da marca pública
    }
  });
</script>

<div
  class="mx-auto flex min-h-screen max-w-md flex-col justify-center px-6 py-12"
>
  <div class="mb-6 flex flex-col items-center gap-3 text-center">
    {#if $mode === "dark" && appSession.logoDarkUrl}
      <img
        src={appSession.logoDarkUrl}
        alt={appSession.companyName}
        class="h-12 max-w-48 rounded-sm object-contain"
      />
    {:else if appSession.logoUrl}
      <img
        src={appSession.logoUrl}
        alt={appSession.companyName}
        class="h-12 max-w-48 rounded-sm object-contain"
      />
    {:else if appSession.iconUrl}
      <img
        src={appSession.iconUrl}
        alt={appSession.companyName}
        class="h-12 w-12 rounded-sm object-cover"
      />
    {:else}
      <div
        class="flex h-12 w-12 items-center justify-center rounded-[var(--radius)] bg-primary font-bold text-lg text-primary-foreground shadow-xs"
      >
        {appSession.companyName.slice(0, 2).toUpperCase()}
      </div>
    {/if}
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-foreground">
        {appSession.companyName}
      </h1>
      <p class="text-xs text-muted-foreground mt-0.5">
        {$_("app.subtitle")}
      </p>
    </div>
  </div>

  <Card>
    <CardHeader>
      <CardTitle class="text-lg">{$_("login.title")}</CardTitle>
      <CardDescription>{$_("login.subtitle")}</CardDescription>
    </CardHeader>
    <CardContent>
      <form onsubmit={handleLogin} class="space-y-4">
        {#if errorMessage}
          <div
            role="alert"
            class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-3.5 py-2.5 text-xs font-medium text-destructive"
          >
            {errorMessage}
          </div>
        {/if}

        <div class="space-y-2">
          <label for="login-email" class="text-sm font-medium text-foreground">
            {$_("login.emailLabel")}
          </label>
          <Input
            id="login-email"
            type="email"
            bind:value={email}
            placeholder={$_("login.emailPlaceholder")}
            required
          />
        </div>

        <div class="space-y-2">
          <label
            for="login-password"
            class="text-sm font-medium text-foreground"
          >
            {$_("login.passwordLabel")}
          </label>
          <Input
            id="login-password"
            type="password"
            bind:value={password}
            placeholder={$_("login.passwordPlaceholder")}
            required
          />
        </div>

        <Button type="submit" class="w-full" disabled={isSubmitting}>
          <LogIn class="h-4 w-4" />
          {isSubmitting ? $_("login.submitting") : $_("login.submitBtn")}
        </Button>

        <div class="pt-2 text-center">
          <button
            type="button"
            onclick={() => {
              recoveryEmail = email;
              recoveryError = null;
              recoveryOpen = true;
            }}
            class="inline-flex items-center gap-1.5 text-xs font-medium text-primary hover:underline cursor-pointer"
          >
            <KeyRound class="h-3.5 w-3.5" />
            {$_("login.forgotPasswordBtn")}
          </button>
        </div>
      </form>
    </CardContent>
  </Card>
</div>

<Dialog
  bind:open={recoveryOpen}
  title={$_("login.recoveryModalTitle")}
  description={$_("login.recoveryModalDesc")}
>
  <form onsubmit={handleRecovery} class="space-y-4">
    {#if recoveryError}
      <div
        role="alert"
        class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs font-medium text-destructive"
      >
        {recoveryError}
      </div>
    {/if}

    <div class="space-y-2">
      <label for="recovery-email" class="text-sm font-medium text-foreground">
        {$_("login.emailLabel")}
      </label>
      <Input
        id="recovery-email"
        type="email"
        bind:value={recoveryEmail}
        placeholder={$_("login.emailPlaceholder")}
        required
      />
    </div>

    <div class="space-y-2">
      <label for="recovery-key" class="text-sm font-medium text-foreground">
        {$_("login.recoveryKeyLabel")}
      </label>
      <Input
        id="recovery-key"
        bind:value={recoveryKey}
        placeholder={$_("login.recoveryKeyPlaceholder")}
        class="font-mono uppercase"
        required
      />
    </div>

    <div class="space-y-2">
      <label
        for="recovery-new-password"
        class="text-sm font-medium text-foreground"
      >
        {$_("login.newPasswordLabel")}
      </label>
      <Input
        id="recovery-new-password"
        type="password"
        bind:value={recoveryNewPassword}
        placeholder={$_("login.newPasswordPlaceholder")}
        minlength={8}
        required
      />
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button
        type="button"
        variant="ghost"
        onclick={() => (recoveryOpen = false)}
      >
        {$_("common.close")}
      </Button>
      <Button type="submit" disabled={isRecovering}>
        {isRecovering
          ? $_("login.recoverySubmitting")
          : $_("login.recoverySubmitBtn")}
      </Button>
    </div>
  </form>
</Dialog>
