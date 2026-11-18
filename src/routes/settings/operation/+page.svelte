<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "svelte-sonner";
  import { SlidersHorizontal, Save, Clock, ShieldAlert } from "lucide-svelte";
  import {
    appSession,
    getSettingsRepository,
    hasPermission,
    PERMISSIONS,
  } from "$lib/data";
  import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
  } from "$lib/ui/card";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import { LoadingState } from "$lib/ui";

  let isLoading = $state(true);
  let isSaving = $state(false);
  let canEdit = $derived(
    hasPermission(appSession.session, PERMISSIONS.SETTINGS_MANAGE),
  );

  let changeoverMinutes = $state("30");
  let internalTransferMinutes = $state("20");
  let externalTransferMinutes = $state("60");
  let fieldErrors = $state<Record<string, string>>({});

  onMount(async () => {
    try {
      const settings = await getSettingsRepository().getAllocationSettings();
      changeoverMinutes = String(settings.changeover_minutes);
      internalTransferMinutes = String(settings.internal_transfer_minutes);
      externalTransferMinutes = String(settings.external_transfer_minutes);
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "Erro ao carregar configurações de operação.",
      );
    } finally {
      isLoading = false;
    }
  });

  function parseMinutes(
    raw: string | number,
    key: string,
    label: string,
    max: number,
  ): number | null {
    const trimmed = String(raw ?? "").trim();
    if (!/^\d+$/.test(trimmed)) {
      fieldErrors[key] = `${label} deve ser um número inteiro.`;
      return null;
    }
    const parsed = Number.parseInt(trimmed, 10);
    if (parsed < 0 || parsed > max) {
      fieldErrors[key] = `${label} deve estar entre 0 e ${max} minutos.`;
      return null;
    }
    return parsed;
  }

  async function handleSave(e: SubmitEvent) {
    e.preventDefault();
    if (!canEdit || isSaving) return;

    fieldErrors = {};
    const changeover = parseMinutes(
      changeoverMinutes,
      "changeover",
      "Virada no mesmo espaço",
      240,
    );
    const internalTransfer = parseMinutes(
      internalTransferMinutes,
      "internalTransfer",
      "Deslocamento interno",
      240,
    );
    const externalTransfer = parseMinutes(
      externalTransferMinutes,
      "externalTransfer",
      "Deslocamento externo",
      720,
    );

    if (
      changeover === null ||
      internalTransfer === null ||
      externalTransfer === null
    ) {
      toast.error("Corrija os campos destacados antes de salvar.");
      return;
    }

    isSaving = true;
    try {
      const updated = await getSettingsRepository().updateAllocationSettings({
        changeover_minutes: changeover,
        internal_transfer_minutes: internalTransfer,
        external_transfer_minutes: externalTransfer,
      });
      changeoverMinutes = String(updated.changeover_minutes);
      internalTransferMinutes = String(updated.internal_transfer_minutes);
      externalTransferMinutes = String(updated.external_transfer_minutes);
      toast.success("Configurações operacionais salvas com sucesso.");
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "Erro ao salvar configurações operacionais.",
      );
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div>
      <h1 class="flex items-center gap-2 text-2xl font-bold tracking-tight">
        <SlidersHorizontal class="size-6 text-primary" />
        Operação e Alocação
      </h1>
      <p class="text-sm text-muted-foreground">
        Defina os tempos operacionais padrão de virada e deslocamento aplicados
        pelo motor de disponibilidade (ADR-0012).
      </p>
    </div>
  </div>

  {#if !canEdit}
    <div
      class="flex items-center gap-3 rounded-lg border border-warning/40 bg-warning/10 p-4 text-sm text-foreground"
    >
      <ShieldAlert class="size-5 shrink-0 text-warning" />
      <span>
        Você possui acesso apenas para visualização. Para alterar os tempos
        padrão, solicite a permissão <code>settings:manage</code>.
      </span>
    </div>
  {/if}

  {#if isLoading}
    <LoadingState message="Carregando configurações operacionais..." />
  {:else}
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2 text-lg">
          <Clock class="size-5 text-primary" />
          Tempos Padrão de Alocação
        </CardTitle>
        <CardDescription>
          Virada e deslocamento interno aceitam de 0 a 240 minutos; deslocamento
          externo aceita de 0 a 720 minutos.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onsubmit={handleSave} class="space-y-5">
          <div class="grid gap-5 sm:grid-cols-3">
            <div class="space-y-2">
              <label for="changeover-minutes" class="text-sm font-medium">
                Virada no mesmo espaço (min)
              </label>
              <Input
                id="changeover-minutes"
                type="number"
                min={0}
                max={240}
                step={1}
                disabled={!canEdit || isSaving}
                bind:value={changeoverMinutes}
              />
              {#if fieldErrors.changeover}
                <p class="text-xs text-destructive">
                  {fieldErrors.changeover}
                </p>
              {:else}
                <p class="text-xs text-muted-foreground">
                  Intervalo entre eventos no mesmo espaço (padrão: 30 min).
                </p>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="transfer-minutes" class="text-sm font-medium">
                Deslocamento interno (min)
              </label>
              <Input
                id="transfer-minutes"
                type="number"
                min={0}
                max={240}
                step={1}
                disabled={!canEdit || isSaving}
                bind:value={internalTransferMinutes}
              />
              {#if fieldErrors.internalTransfer}
                <p class="text-xs text-destructive">
                  {fieldErrors.internalTransfer}
                </p>
              {:else}
                <p class="text-xs text-muted-foreground">
                  Entre espaços do mesmo local quando não definido no cadastro
                  (padrão: 20 min).
                </p>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="external-minutes" class="text-sm font-medium">
                Deslocamento externo (min)
              </label>
              <Input
                id="external-minutes"
                type="number"
                min={0}
                max={720}
                step={1}
                disabled={!canEdit || isSaving}
                bind:value={externalTransferMinutes}
              />
              {#if fieldErrors.externalTransfer}
                <p class="text-xs text-destructive">
                  {fieldErrors.externalTransfer}
                </p>
              {:else}
                <p class="text-xs text-muted-foreground">
                  Deslocamento entre locais diferentes (padrão: 60 min).
                </p>
              {/if}
            </div>
          </div>

          {#if canEdit}
            <div class="flex justify-end border-t border-border pt-4">
              <Button type="submit" disabled={isSaving}>
                <Save class="mr-2 size-4" />
                {isSaving ? "Salvando..." : "Salvar configurações"}
              </Button>
            </div>
          {/if}
        </form>
      </CardContent>
    </Card>
  {/if}
</div>
