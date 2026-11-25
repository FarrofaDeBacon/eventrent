<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "svelte-sonner";
  import {
    Archive,
    FolderOpen,
    RefreshCw,
    Play,
    CheckCircle2,
    AlertTriangle,
    XCircle,
    ShieldAlert,
    HardDrive,
    Lock,
    FileArchive,
    ShieldCheck,
  } from "lucide-svelte";
  import {
    appSession,
    getBackupRepository,
    hasPermission,
    PERMISSIONS,
    type BackupStatus,
    type BackupMetadataView,
    type SetBackupSettingsInput,
  } from "$lib/data";
  import {
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
    CardFooter,
  } from "$lib/ui/card";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import { Badge } from "$lib/ui/badge";
  import { LoadingState } from "$lib/ui";

  let isLoading = $state(true);
  let isSavingSettings = $state(false);
  let isRunningBackup = $state(false);
  let isRefreshingList = $state(false);

  let status = $state<BackupStatus | null>(null);
  let backups = $state<BackupMetadataView[]>([]);

  // Campos de configuração editáveis
  let destinationFolder = $state("");
  let autoBackupEnabled = $state(true);
  let retentionDaily = $state(7);
  let retentionWeekly = $state(4);
  let retentionMonthly = $state(12);

  // Modal de Restauração Bloqueante
  let restoreModalOpen = $state(false);
  let backupToRestore = $state<BackupMetadataView | null>(null);
  let restoreConfirmationInput = $state("");
  let restorePasswordInput = $state("");
  let isSchedulingRestore = $state(false);

  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.BACKUP_MANAGE),
  );

  let isRestoreConfirmed = $derived(
    restoreConfirmationInput.trim() === "RESTAURAR" &&
      restorePasswordInput.trim().length > 0,
  );

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  function formatDate(isoString: string | null): string {
    if (!isoString) return "Nunca";
    try {
      const date = new Date(isoString);
      return new Intl.DateTimeFormat("pt-BR", {
        dateStyle: "short",
        timeStyle: "medium",
      }).format(date);
    } catch {
      return isoString;
    }
  }

  async function loadData() {
    try {
      const repo = getBackupRepository();
      const currentStatus = await repo.getStatus();
      status = currentStatus;
      destinationFolder = currentStatus.settings.destination_folder;
      autoBackupEnabled = currentStatus.settings.schedule_enabled;
      retentionDaily = currentStatus.settings.retention_daily;
      retentionWeekly = currentStatus.settings.retention_weekly;
      retentionMonthly = currentStatus.settings.retention_monthly;

      if (destinationFolder.trim().length > 0) {
        backups = await repo.listBackups();
      } else {
        backups = [];
      }
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "Erro ao carregar configurações de backup.",
      );
    } finally {
      isLoading = false;
    }
  }

  async function refreshBackupList() {
    if (!destinationFolder.trim()) return;
    isRefreshingList = true;
    try {
      backups = await getBackupRepository().listBackups();
      toast.success("Lista de backups atualizada.");
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Erro ao listar backups locais.",
      );
    } finally {
      isRefreshingList = false;
    }
  }

  async function handleChooseFolder() {
    if (!canManage) return;
    try {
      const selected = await getBackupRepository().chooseFolder();
      if (selected) {
        destinationFolder = selected;
      }
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Erro ao abrir diálogo de pastas.",
      );
    }
  }

  async function handleSaveSettings() {
    if (!canManage || isSavingSettings) return;

    if (!destinationFolder.trim()) {
      toast.error("Informe uma pasta de destino válida.");
      return;
    }

    isSavingSettings = true;
    try {
      const input: SetBackupSettingsInput = {
        destination_folder: destinationFolder.trim(),
        schedule_enabled: autoBackupEnabled,
        retention_daily: Math.max(
          1,
          Math.min(365, Number(retentionDaily) || 7),
        ),
        retention_weekly: Math.max(
          1,
          Math.min(52, Number(retentionWeekly) || 4),
        ),
        retention_monthly: Math.max(
          1,
          Math.min(120, Number(retentionMonthly) || 12),
        ),
      };

      const updatedSettings = await getBackupRepository().updateSettings(input);
      if (status) {
        status.settings = updatedSettings;
      }
      toast.success("Configurações de backup salvas com sucesso.");
      await refreshBackupList();
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "Falha ao salvar configurações de backup.",
      );
    } finally {
      isSavingSettings = false;
    }
  }

  async function handleRunBackupNow() {
    if (!canManage || isRunningBackup) return;

    if (!destinationFolder.trim()) {
      toast.error(
        "Selecione uma pasta de destino antes de executar o backup imediato.",
      );
      return;
    }

    isRunningBackup = true;
    try {
      const result = await getBackupRepository().runBackupNow();
      if (result.skipped_media_files && result.skipped_media_files > 0) {
        toast.warning(
          `Backup concluído com ressalvas: ${result.file_name} (${formatBytes(result.byte_size)}). ${result.skipped_media_files} arquivo(s) espúrios/não-canônicos ignorados na pasta de mídia.`,
        );
      } else {
        toast.success(
          `Backup concluído: ${result.file_name} (${formatBytes(result.byte_size)})`,
        );
      }
      await loadData();
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Falha ao executar backup.",
      );
      await loadData();
    } finally {
      isRunningBackup = false;
    }
  }

  let verifyingFiles = $state<Record<string, boolean>>({});
  let verifiedFiles = $state<Record<string, boolean>>({});

  async function handleVerifyBackup(backup: BackupMetadataView) {
    if (!canManage) return;
    verifyingFiles[backup.file_name] = true;
    try {
      const res = await getBackupRepository().verifyBackup(backup.file_path);
      if (res.valid) {
        verifiedFiles[backup.file_name] = true;
        toast.success(
          `Integridade física de ${backup.file_name} verificada com sucesso!`,
        );
      } else {
        verifiedFiles[backup.file_name] = false;
        toast.error(
          `Backup inválido: ${res.error_message || "Falha na verificação de integridade"}`,
        );
      }
    } catch (err) {
      verifiedFiles[backup.file_name] = false;
      toast.error(
        err instanceof Error
          ? err.message
          : "Erro ao verificar integridade do backup.",
      );
    } finally {
      verifyingFiles[backup.file_name] = false;
    }
  }

  function openRestoreModal(backup: BackupMetadataView) {
    backupToRestore = backup;
    restoreConfirmationInput = "";
    restorePasswordInput = "";
    restoreModalOpen = true;
  }

  function closeRestoreModal() {
    restoreModalOpen = false;
    backupToRestore = null;
    restoreConfirmationInput = "";
    restorePasswordInput = "";
  }

  async function handleConfirmRestore() {
    if (!canManage || !backupToRestore || !isRestoreConfirmed) return;

    isSchedulingRestore = true;
    try {
      await getBackupRepository().scheduleRestore(
        backupToRestore.file_path,
        restorePasswordInput,
      );
      toast.success(
        "Restauração agendada com sucesso. Feche e reinicie o EventRent para concluir.",
      );
      closeRestoreModal();
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Falha ao agendar restauração.",
      );
    } finally {
      isSchedulingRestore = false;
    }
  }

  onMount(() => {
    void loadData();
    const unsubscribe = getBackupRepository().onStatusChanged?.(() => {
      void loadData();
    });
    return () => {
      unsubscribe?.();
    };
  });
</script>

<div class="space-y-6 p-6 max-w-6xl mx-auto">
  <!-- Cabeçalho -->
  <div class="flex items-center justify-between">
    <div>
      <h1
        class="text-2xl font-bold tracking-tight text-foreground flex items-center gap-2"
      >
        <Archive class="h-6 w-6 text-primary" />
        Backup e Restauração
      </h1>
      <p class="text-sm text-muted-foreground mt-1">
        Cópia de segurança transacional offline, retenção automatizada e
        restauração segura a frio.
      </p>
    </div>
  </div>

  <!-- Disclaimer de Privacidade e LGPD -->
  <div
    class="flex items-start gap-3 rounded-lg border border-border bg-card p-4 text-sm text-foreground shadow-sm"
  >
    <Lock class="h-5 w-5 text-primary shrink-0 mt-0.5" />
    <div class="space-y-1">
      <p class="font-semibold text-foreground">
        Aviso de Privacidade e Conformidade LGPD
      </p>
      <p class="text-muted-foreground text-xs leading-relaxed">
        Os arquivos de backup gerados contêm a integridade total do banco de
        dados operacional, abrangendo registros cadastrais, dados pessoais de
        clientes, contatos de fornecedores e operadores do sistema. Recomenda-se
        armazenar os backups em discos seguros, mídias externas com criptografia
        e controlar rigorosamente o acesso físico e lógico aos arquivos gerados.
      </p>
    </div>
  </div>

  {#if isLoading}
    <div class="flex justify-center py-12">
      <LoadingState message="Carregando configurações de backup..." />
    </div>
  {:else}
    <!-- Alerta de Backup Desatualizado (is_stale) -->
    {#if status?.is_stale}
      <div
        class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/10 p-4 text-destructive shadow-sm"
      >
        <AlertTriangle class="h-5 w-5 shrink-0 mt-0.5" />
        <div class="space-y-1">
          <p class="font-semibold text-sm">
            Atenção: Nenhum backup recente encontrado!
          </p>
          <p class="text-xs leading-relaxed opacity-90">
            O último backup foi realizado há mais de 7 dias ou nunca foi
            executado. Para proteger seus dados contra falhas de hardware ou
            perda acidental, configure uma pasta e execute um backup agora.
          </p>
        </div>
      </div>
    {/if}

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Coluna da Esquerda: Configuração e Retenção -->
      <div class="lg:col-span-2 space-y-6">
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2 text-lg">
              <HardDrive class="h-5 w-5 text-primary" />
              Pasta de Destino e Agendamento
            </CardTitle>
            <CardDescription>
              Defina o diretório local ou unidade externa onde os pacotes de
              backup zipados serão gravados.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <!-- Seletor de Pasta -->
            <div class="space-y-2">
              <label
                for="destination-folder"
                class="text-sm font-medium text-foreground block"
              >
                Pasta de Armazenamento
              </label>
              <div class="flex gap-2">
                <Input
                  id="destination-folder"
                  type="text"
                  placeholder="Ex: D:\EventRent-Backups ou E:\PenDrive"
                  bind:value={destinationFolder}
                  disabled={!canManage || isSavingSettings}
                  class="font-mono text-xs flex-1"
                />
                <Button
                  variant="outline"
                  onclick={handleChooseFolder}
                  disabled={!canManage || isSavingSettings}
                  class="flex items-center gap-2"
                >
                  <FolderOpen class="h-4 w-4" />
                  Procurar...
                </Button>
              </div>
              <p class="text-xs text-muted-foreground">
                Por segurança, o caminho deve ser um diretório exclusivo para
                backups (não utilize a raiz C:\ ou pastas de sistema).
              </p>
            </div>

            <!-- Toggle de Backup Automático Diário -->
            <div
              class="flex items-center justify-between rounded-lg border border-border p-3"
            >
              <div class="space-y-0.5">
                <span class="text-sm font-medium text-foreground block">
                  Backup Automático Diário
                </span>
                <span class="text-xs text-muted-foreground block">
                  Executa automaticamente uma cópia diária em segundo plano
                  respeitando a política de retenção.
                </span>
              </div>
              <label
                class="relative inline-flex items-center cursor-pointer select-none"
              >
                <input
                  type="checkbox"
                  bind:checked={autoBackupEnabled}
                  disabled={!canManage || isSavingSettings}
                  class="sr-only peer"
                />
                <div
                  class="w-11 h-6 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-background after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"
                ></div>
              </label>
            </div>

            <!-- Políticas de Retenção -->
            <div class="space-y-3 pt-2">
              <span class="text-sm font-medium text-foreground block">
                Política de Retenção e Expiração Automática
              </span>
              <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
                <div class="space-y-1">
                  <label
                    for="ret-days"
                    class="text-xs font-medium text-muted-foreground block"
                  >
                    Dias (Diários)
                  </label>
                  <Input
                    id="ret-days"
                    type="number"
                    min="1"
                    max="365"
                    bind:value={retentionDaily}
                    disabled={!canManage || isSavingSettings}
                  />
                  <span class="text-[10px] text-muted-foreground block">
                    Mantém 1 por dia nos últimos X dias.
                  </span>
                </div>
                <div class="space-y-1">
                  <label
                    for="ret-weeks"
                    class="text-xs font-medium text-muted-foreground block"
                  >
                    Semanas
                  </label>
                  <Input
                    id="ret-weeks"
                    type="number"
                    min="1"
                    max="52"
                    bind:value={retentionWeekly}
                    disabled={!canManage || isSavingSettings}
                  />
                  <span class="text-[10px] text-muted-foreground block">
                    Mantém 1 por semana nas últimas X semanas.
                  </span>
                </div>
                <div class="space-y-1">
                  <label
                    for="ret-months"
                    class="text-xs font-medium text-muted-foreground block"
                  >
                    Meses
                  </label>
                  <Input
                    id="ret-months"
                    type="number"
                    min="1"
                    max="120"
                    bind:value={retentionMonthly}
                    disabled={!canManage || isSavingSettings}
                  />
                  <span class="text-[10px] text-muted-foreground block">
                    Mantém 1 por mês nos últimos X meses.
                  </span>
                </div>
              </div>
            </div>
          </CardContent>
          <CardFooter class="flex justify-end border-t border-border pt-4">
            <Button
              onclick={handleSaveSettings}
              disabled={!canManage || isSavingSettings}
            >
              {#if isSavingSettings}
                Salvando...
              {:else}
                Salvar Configurações
              {/if}
            </Button>
          </CardFooter>
        </Card>
      </div>

      <!-- Coluna da Direita: Status e Ação Imediata -->
      <div class="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle class="text-lg">Status Operacional</CardTitle>
            <CardDescription>
              Resumo da última rotina de backup executada.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4 text-sm">
            <div
              class="flex items-center justify-between border-b border-border pb-2"
            >
              <span class="text-muted-foreground"
                >Último backup bem-sucedido:</span
              >
              <span class="font-medium text-foreground">
                {formatDate(
                  status?.settings.last_success_at ??
                    status?.settings.last_backup_at ??
                    null,
                )}
              </span>
            </div>
            {#if status?.settings.last_attempt_at && status.settings.last_attempt_at !== (status.settings.last_success_at ?? status.settings.last_backup_at)}
              <div
                class="flex items-center justify-between border-b border-border pb-2 text-xs"
              >
                <span class="text-muted-foreground">Última tentativa:</span>
                <span class="font-medium text-foreground">
                  {formatDate(status.settings.last_attempt_at)}
                </span>
              </div>
            {/if}
            <div
              class="flex items-center justify-between border-b border-border pb-2"
            >
              <span class="text-muted-foreground">Status do último:</span>
              <span>
                {#if status?.settings.last_backup_status === "success"}
                  <Badge
                    variant="default"
                    class="bg-primary/20 text-primary border-primary/30"
                  >
                    Sucesso
                  </Badge>
                {:else if status?.settings.last_backup_status === "failed"}
                  <Badge variant="destructive">Falha</Badge>
                {:else}
                  <Badge variant="secondary">Nenhum</Badge>
                {/if}
              </span>
            </div>
            {#if status?.settings.last_backup_error}
              <div
                class="rounded bg-destructive/10 p-2 text-xs text-destructive"
              >
                <span class="font-semibold block">Erro registrado:</span>
                {status.settings.last_backup_error}
              </div>
            {/if}
          </CardContent>
          <CardFooter class="flex flex-col gap-2 border-t border-border pt-4">
            <Button
              class="w-full flex items-center justify-center gap-2"
              onclick={handleRunBackupNow}
              disabled={!canManage ||
                isRunningBackup ||
                !destinationFolder.trim()}
            >
              {#if isRunningBackup}
                <RefreshCw class="h-4 w-4 animate-spin" />
                Criando backup transacional...
              {:else}
                <Play class="h-4 w-4" />
                Fazer Backup Agora
              {/if}
            </Button>
            {#if !destinationFolder.trim()}
              <span class="text-[11px] text-muted-foreground text-center">
                Configure uma pasta de destino para habilitar.
              </span>
            {/if}
          </CardFooter>
        </Card>
      </div>
    </div>

    <!-- Seção Inferior: Histórico de Backups na Pasta -->
    <Card>
      <CardHeader class="flex flex-row items-center justify-between">
        <div>
          <CardTitle class="text-lg flex items-center gap-2">
            <FileArchive class="h-5 w-5 text-primary" />
            Backups Encontrados na Pasta
          </CardTitle>
          <CardDescription>
            Histórico de arquivos .zip válidos presentes no diretório
            configurado.
          </CardDescription>
        </div>
        <Button
          variant="outline"
          size="sm"
          onclick={refreshBackupList}
          disabled={isRefreshingList || !destinationFolder.trim()}
          class="flex items-center gap-2"
        >
          <RefreshCw
            class={`h-4 w-4 ${isRefreshingList ? "animate-spin" : ""}`}
          />
          Atualizar Lista
        </Button>
      </CardHeader>
      <CardContent>
        {#if !destinationFolder.trim()}
          <div class="text-center py-8 text-sm text-muted-foreground">
            Selecione uma pasta de armazenamento acima para visualizar o
            histórico de backups.
          </div>
        {:else if backups.length === 0}
          <div class="text-center py-8 text-sm text-muted-foreground space-y-1">
            <Archive class="h-8 w-8 mx-auto text-muted-foreground/50 mb-2" />
            <p class="font-medium text-foreground">
              Nenhum backup encontrado na pasta.
            </p>
            <p class="text-xs">
              Clique em "Fazer Backup Agora" para criar a primeira cópia segura.
            </p>
          </div>
        {:else}
          <div class="overflow-x-auto">
            <table class="w-full text-left text-sm">
              <thead
                class="border-b border-border text-xs text-muted-foreground uppercase"
              >
                <tr>
                  <th class="py-3 px-4">Arquivo</th>
                  <th class="py-3 px-4">Tamanho</th>
                  <th class="py-3 px-4">Data de Criação</th>
                  <th class="py-3 px-4">Versão App / Schema</th>
                  <th class="py-3 px-4">Compatibilidade</th>
                  <th class="py-3 px-4">Integridade</th>
                  <th class="py-3 px-4 text-right">Ação</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-border">
                {#each backups as backup (backup.file_name)}
                  <tr class="hover:bg-muted/50 transition-colors">
                    <td
                      class="py-3 px-4 font-mono text-xs text-foreground font-medium"
                    >
                      <div class="flex items-center gap-1.5 flex-wrap">
                        <span>{backup.file_name}</span>
                        {#if backup.skipped_media_files && backup.skipped_media_files > 0}
                          <Badge
                            variant="secondary"
                            class="text-[10px] px-1.5 py-0 bg-amber-500/10 text-amber-500 border border-amber-500/20"
                            title="{backup.skipped_media_files} arquivo(s) espúrios/não-canônicos ignorados"
                          >
                            {backup.skipped_media_files} ignorados
                          </Badge>
                        {/if}
                      </div>
                    </td>
                    <td class="py-3 px-4 text-xs text-muted-foreground">
                      {formatBytes(backup.byte_size)}
                    </td>
                    <td class="py-3 px-4 text-xs text-muted-foreground">
                      {formatDate(backup.created_at)}
                    </td>
                    <td class="py-3 px-4 text-xs text-muted-foreground">
                      v{backup.app_version} (schema #{backup.last_applied_migration})
                    </td>
                    <td class="py-3 px-4 text-xs">
                      {#if backup.is_compatible}
                        <Badge
                          variant="outline"
                          class="text-primary border-primary/30 flex items-center gap-1 w-fit"
                        >
                          <CheckCircle2 class="h-3 w-3" />
                          Compatível
                        </Badge>
                      {:else}
                        <Badge
                          variant="destructive"
                          class="flex items-center gap-1 w-fit"
                          title="Versão do banco incompatível ou superior à aplicação atual"
                        >
                          <XCircle class="h-3 w-3" />
                          Incompatível
                        </Badge>
                      {/if}
                    </td>
                    <td class="py-3 px-4 text-xs">
                      {#if verifyingFiles[backup.file_name]}
                        <Badge
                          variant="outline"
                          class="flex items-center gap-1 w-fit text-muted-foreground animate-pulse"
                        >
                          <RefreshCw class="h-3 w-3 animate-spin" />
                          Verificando...
                        </Badge>
                      {:else if verifiedFiles[backup.file_name] === true || backup.verified}
                        <Badge
                          variant="outline"
                          class="text-primary border-primary/30 flex items-center gap-1 w-fit"
                          title="Integridade física e SHA-256 conferidos"
                        >
                          <CheckCircle2 class="h-3 w-3 text-primary" />
                          Íntegro
                        </Badge>
                      {:else if verifiedFiles[backup.file_name] === false}
                        <Badge
                          variant="destructive"
                          class="flex items-center gap-1 w-fit"
                          title="Falha na integridade física do arquivo"
                        >
                          <AlertTriangle class="h-3 w-3" />
                          Inválido
                        </Badge>
                      {:else}
                        <Button
                          variant="ghost"
                          size="sm"
                          class="h-7 px-2 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"
                          onclick={() => handleVerifyBackup(backup)}
                          disabled={!canManage || isSchedulingRestore}
                        >
                          <ShieldCheck class="h-3.5 w-3.5 text-primary" />
                          Verificar
                        </Button>
                      {/if}
                    </td>
                    <td class="py-3 px-4 text-right">
                      <Button
                        variant="destructive"
                        size="sm"
                        onclick={() => openRestoreModal(backup)}
                        disabled={!canManage ||
                          !backup.is_compatible ||
                          isSchedulingRestore}
                      >
                        Restaurar...
                      </Button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </CardContent>
    </Card>
  {/if}
</div>

<!-- Modal Bloqueante de Confirmação de Restauração -->
{#if restoreModalOpen && backupToRestore}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200"
    role="dialog"
    aria-modal="true"
    aria-labelledby="restore-modal-title"
  >
    <div
      class="w-full max-w-lg rounded-xl border border-destructive/40 bg-card p-6 shadow-2xl space-y-5 animate-in zoom-in-95 duration-200"
    >
      <div class="flex items-start gap-4">
        <div class="rounded-full bg-destructive/10 p-3 text-destructive">
          <ShieldAlert class="h-6 w-6" />
        </div>
        <div class="space-y-1">
          <h2
            id="restore-modal-title"
            class="text-lg font-bold text-foreground"
          >
            Confirmar Restauração a Frio
          </h2>
          <p class="text-xs text-muted-foreground">
            Operação de substituição irreversível dos dados locais ativos.
          </p>
        </div>
      </div>

      <div
        class="rounded-lg border border-destructive/20 bg-destructive/5 p-4 text-xs text-destructive space-y-2"
      >
        <p class="font-semibold">AVISO CRÍTICO:</p>
        <p class="leading-relaxed">
          A restauração substituirá <strong>integralmente</strong> todos os dados
          cadastrados no aplicativo (banco SQLite e arquivos de mídia) pelos dados
          contidos neste arquivo de backup.
        </p>
        <p class="leading-relaxed">
          Para garantir a integridade absoluta dos dados sem conexões ativas
          concorrentes, a operação é executada <strong
            >a frio na próxima inicialização</strong
          > do aplicativo. Ao confirmar, o arquivo será agendado e o sistema solicitará
          o encerramento do app.
        </p>
      </div>

      <div
        class="bg-muted rounded-lg p-3 text-xs space-y-1 text-muted-foreground"
      >
        <div>
          <span class="font-medium text-foreground">Arquivo:</span>
          {backupToRestore.file_name}
        </div>
        <div>
          <span class="font-medium text-foreground">Data do backup:</span>
          {formatDate(backupToRestore.created_at)}
        </div>
        <div>
          <span class="font-medium text-foreground">Tamanho:</span>
          {formatBytes(backupToRestore.byte_size)}
        </div>
        <div>
          <span class="font-medium text-foreground">Versão:</span>
          v{backupToRestore.app_version} (schema #{backupToRestore.last_applied_migration})
        </div>
      </div>

      <div
        class="rounded-lg border border-warning/30 bg-warning/10 p-3 text-xs text-warning space-y-1"
      >
        <div class="font-semibold flex items-center gap-1.5">
          <AlertTriangle class="h-4 w-4 shrink-0" />
          Atenção com as credenciais restauradas
        </div>
        <p class="text-muted-foreground">
          A restauração substituirá todos os cadastros do banco de dados,
          incluindo usuários e senhas. Ao concluir a restauração, suas
          credenciais de acesso voltarão a ser as que existiam no momento em que
          o backup foi gerado.
        </p>
      </div>

      <div class="space-y-2">
        <label
          for="restore-password-input"
          class="text-xs font-medium text-foreground block"
        >
          Confirme sua senha de acesso atual:
        </label>
        <Input
          id="restore-password-input"
          type="password"
          placeholder="Digite sua senha de acesso atual"
          bind:value={restorePasswordInput}
        />
      </div>

      <div class="space-y-2">
        <label
          for="restore-confirm-input"
          class="text-xs font-medium text-foreground block"
        >
          Para prosseguir, digite exatamente <span
            class="font-bold text-destructive">RESTAURAR</span
          > no campo abaixo:
        </label>
        <Input
          id="restore-confirm-input"
          type="text"
          placeholder="RESTAURAR"
          bind:value={restoreConfirmationInput}
          class="font-mono text-center tracking-widest uppercase font-bold"
        />
      </div>

      <div class="flex justify-end gap-3 pt-2">
        <Button
          variant="outline"
          onclick={closeRestoreModal}
          disabled={isSchedulingRestore}
        >
          Cancelar
        </Button>
        <Button
          variant="destructive"
          onclick={handleConfirmRestore}
          disabled={!isRestoreConfirmed || isSchedulingRestore}
          class="flex items-center gap-2"
        >
          {#if isSchedulingRestore}
            Agendando...
          {:else}
            Confirmar e Agendar Restauração
          {/if}
        </Button>
      </div>
    </div>
  </div>
{/if}
