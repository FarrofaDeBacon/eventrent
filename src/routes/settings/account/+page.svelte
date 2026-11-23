<script lang="ts">
  import { appSession, getAuthRepository } from "$lib/data";
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
  import { toast } from "svelte-sonner";
  import RecoveryKeyModal from "$lib/components/recovery-key-modal.svelte";
  import { KeyRound, Shield, User } from "lucide-svelte";

  let passwordModalOpen = $state(false);
  let currentPassword = $state("");
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);

  let showNewKeyModal = $state(false);
  let newRecoveryKeyToDisplay = $state("");

  async function handleConfirmRegenerate(event: SubmitEvent) {
    event.preventDefault();
    if (!currentPassword) return;

    errorMessage = null;
    isSubmitting = true;

    try {
      const repo = getAuthRepository();
      const result = await repo.regenerateRecoveryKey({
        current_password: currentPassword,
      });

      passwordModalOpen = false;
      currentPassword = "";
      newRecoveryKeyToDisplay = result.new_recovery_key;
      showNewKeyModal = true;
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
      toast.error(errorMessage);
    } finally {
      isSubmitting = false;
    }
  }

  function handleCloseRecoveryModal() {
    newRecoveryKeyToDisplay = "";
    showNewKeyModal = false;
    toast.success($_("account.regenerateSuccessToast"));
  }
</script>

<div class="space-y-6 max-w-4xl">
  <div>
    <h1 class="text-2xl font-bold tracking-tight text-foreground">
      {$_("account.title")}
    </h1>
    <p class="text-sm text-muted-foreground">
      {$_("account.subtitle")}
    </p>
  </div>

  <!-- Dados da conta -->
  <Card>
    <CardHeader>
      <div class="flex items-center gap-3">
        <User class="h-5 w-5 text-primary" />
        <div>
          <CardTitle class="text-lg">{$_("account.profileCardTitle")}</CardTitle
          >
          <CardDescription>{$_("account.profileCardDesc")}</CardDescription>
        </div>
      </div>
    </CardHeader>
    <CardContent class="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
      <div class="space-y-1">
        <span class="text-xs text-muted-foreground uppercase font-semibold">
          {$_("account.nameLabel")}
        </span>
        <p class="font-medium text-foreground">
          {appSession.session?.name ?? "—"}
        </p>
      </div>

      <div class="space-y-1">
        <span class="text-xs text-muted-foreground uppercase font-semibold">
          {$_("account.emailLabel")}
        </span>
        <p class="font-medium text-foreground">
          {appSession.session?.email ?? "—"}
        </p>
      </div>

      <div class="space-y-1">
        <span class="text-xs text-muted-foreground uppercase font-semibold">
          {$_("account.roleLabel")}
        </span>
        <p class="font-medium text-foreground">
          {appSession.session?.role_name ?? "—"}
        </p>
      </div>
    </CardContent>
  </Card>

  <!-- Segurança e Recuperação -->
  <Card>
    <CardHeader>
      <div class="flex items-center gap-3">
        <Shield class="h-5 w-5 text-primary" />
        <div>
          <CardTitle class="text-lg"
            >{$_("account.securityCardTitle")}</CardTitle
          >
          <CardDescription>{$_("account.securityCardDesc")}</CardDescription>
        </div>
      </div>
    </CardHeader>
    <CardContent class="space-y-4">
      <div
        class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 rounded-[var(--radius)] border border-border p-4 bg-muted/30"
      >
        <div class="space-y-1 max-w-xl">
          <div class="flex items-center gap-2">
            <KeyRound class="h-4 w-4 text-primary" />
            <h4 class="text-sm font-semibold text-foreground">
              {$_("account.recoveryKeyHeading")}
            </h4>
          </div>
          <p class="text-xs text-muted-foreground">
            {$_("account.recoveryKeyInfo")}
          </p>
        </div>

        <Button
          type="button"
          variant="outline"
          onclick={() => {
            currentPassword = "";
            errorMessage = null;
            passwordModalOpen = true;
          }}
          data-testid="regenerate-key-btn"
          class="shrink-0"
        >
          {$_("account.regenerateKeyBtn")}
        </Button>
      </div>
    </CardContent>
  </Card>
</div>

<!-- Modal de confirmação de senha para regeneração -->
<Dialog
  bind:open={passwordModalOpen}
  title={$_("account.confirmPasswordDialogTitle")}
  description={$_("account.confirmPasswordDialogDesc")}
>
  <form onsubmit={handleConfirmRegenerate} class="space-y-4 pt-2">
    {#if errorMessage}
      <div
        class="rounded-[var(--radius)] bg-destructive/10 p-3 text-xs text-destructive"
      >
        {errorMessage}
      </div>
    {/if}

    <div class="space-y-2">
      <label
        for="current-password-input"
        class="text-sm font-medium text-foreground"
      >
        {$_("account.currentPasswordLabel")}
      </label>
      <Input
        id="current-password-input"
        data-testid="current-password-input"
        type="password"
        bind:value={currentPassword}
        placeholder={$_("account.currentPasswordPlaceholder")}
        required
      />
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button
        type="button"
        variant="ghost"
        onclick={() => {
          passwordModalOpen = false;
          currentPassword = "";
        }}
      >
        {$_("common.cancel")}
      </Button>
      <Button
        type="submit"
        disabled={isSubmitting || !currentPassword}
        data-testid="confirm-regenerate-btn"
      >
        {isSubmitting
          ? $_("account.regeneratingProgress")
          : $_("account.confirmRegenerateBtn")}
      </Button>
    </div>
  </form>
</Dialog>

<!-- Modal bloqueante de exibição da nova chave -->
<RecoveryKeyModal
  open={showNewKeyModal}
  recoveryKey={newRecoveryKeyToDisplay}
  title={$_("account.newRecoveryKeyTitle")}
  description={$_("account.newRecoveryKeyDesc")}
  onclose={handleCloseRecoveryModal}
/>
