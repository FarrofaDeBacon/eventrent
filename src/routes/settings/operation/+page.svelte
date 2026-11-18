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

  let defaultSetupMinutes = $state("30");
  let defaultTeardownMinutes = $state("20");
  let defaultTurnoverBufferMinutes = $state("60");
  let fieldErrors = $state<Record<string, string>>({});

  onMount(async () => {
    try {
      const settings = await getSettingsRepository().getAllocationSettings();
      defaultSetupMinutes = String(settings.default_setup_minutes);
      defaultTeardownMinutes = String(settings.default_teardown_minutes);
      defaultTurnoverBufferMinutes = String(
        settings.default_turnover_buffer_minutes,
      );
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
    raw: string,
    key: string,
    label: string,
  ): number | null {
    const trimmed = raw.trim();
    if (!/^\d+$/.test(trimmed)) {
      fieldErrors[key] = `${label} deve ser um número inteiro.`;
      return null;
    }
    const parsed = Number.parseInt(trimmed, 10);
    if (parsed < 0 || parsed > 240) {
      fieldErrors[key] = `${label} deve estar entre 0 e 240 minutos.`;
      return null;
    }
    return parsed;
  }

  async function handleSave(e: SubmitEvent) {
    e.preventDefault();
    if (!canEdit || isSaving) return;

    fieldErrors = {};
    const setup = parseMinutes(
      defaultSetupMinutes,
      "setup",
      "Tempo padrão de montagem",
    );
    const teardown = parseMinutes(
      defaultTeardownMinutes,
      "teardown",
      "Tempo padrão de desmontagem",
    );
    const turnover = parseMinutes(
      defaultTurnoverBufferMinutes,
      "turnover",
      "Buffer padrão de giro",
    );

    if (setup === null || teardown === null || turnover === null) {
      toast.error("Corrija os campos destacados antes de salvar.");
      return;
    }

    isSaving = true;
    try {
      const updated = await getSettingsRepository().updateAllocationSettings({
        default_setup_minutes: setup,
        default_teardown_minutes: teardown,
        default_turnover_buffer_minutes: turnover,
      });
      defaultSetupMinutes = String(updated.default_setup_minutes);
      defaultTeardownMinutes = String(updated.default_teardown_minutes);
      defaultTurnoverBufferMinutes = String(
        updated.default_turnover_buffer_minutes,
      );
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
        Defina os tempos operacionais padrão aplicados em novas reservas e
        eventos (ADR-0012).
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
          Valores em minutos (0 a 240). Os eventos poderão sobrescrever estes
          tempos caso a caso.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onsubmit={handleSave} class="space-y-5">
          <div class="grid gap-5 sm:grid-cols-3">
            <div class="space-y-2">
              <label for="setup-minutes" class="text-sm font-medium">
                Montagem padrão (min)
              </label>
              <Input
                id="setup-minutes"
                type="number"
                min={0}
                max={240}
                step={1}
                disabled={!canEdit || isSaving}
                bind:value={defaultSetupMinutes}
              />
              {#if fieldErrors.setup}
                <p class="text-xs text-destructive">{fieldErrors.setup}</p>
              {:else}
                <p class="text-xs text-muted-foreground">
                  Tempo antes do início do evento (padrão: 30 min).
                </p>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="teardown-minutes" class="text-sm font-medium">
                Desmontagem padrão (min)
              </label>
              <Input
                id="teardown-minutes"
                type="number"
                min={0}
                max={240}
                step={1}
                disabled={!canEdit || isSaving}
                bind:value={defaultTeardownMinutes}
              />
              {#if fieldErrors.teardown}
                <p class="text-xs text-destructive">{fieldErrors.teardown}</p>
              {:else}
                <p class="text-xs text-muted-foreground">
                  Tempo após o encerramento (padrão: 20 min).
                </p>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="turnover-minutes" class="text-sm font-medium">
                Buffer de giro (min)
              </label>
              <Input
                id="turnover-minutes"
                type="number"
                min={0}
                max={240}
                step={1}
                disabled={!canEdit || isSaving}
                bind:value={defaultTurnoverBufferMinutes}
              />
              {#if fieldErrors.turnover}
                <p class="text-xs text-destructive">{fieldErrors.turnover}</p>
              {:else}
                <p class="text-xs text-muted-foreground">
                  Intervalo entre devolução e nova saída (padrão: 60 min).
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
