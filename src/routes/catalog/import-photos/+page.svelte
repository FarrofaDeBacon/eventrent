<script lang="ts">
  import {
    AlertTriangle,
    ArrowLeft,
    CheckCircle2,
    ImagePlus,
    Trash2,
    Upload,
    XCircle,
  } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import { getCatalogRepository, type MediaOrigin } from "$lib/data";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import {
    extractFilenameStem,
    matchFilenameToCatalogModels,
    type BatchMatchableModel,
    type BatchMatchField,
    type BatchMatchStatus,
  } from "$lib/utils/batch-photo-matcher";
  import {
    formatMediaByteSize,
    resizeImageForModelUpload,
  } from "$lib/utils/image-resize";

  interface BatchRow {
    id: string;
    file: File;
    filename: string;
    initialStatus: BatchMatchStatus;
    matchedBy: BatchMatchField;
    selectedModelId: string;
    candidates: BatchMatchableModel[];
    manuallyCorrected: boolean;
    processState: "pending" | "processing" | "success" | "error";
    processMessage: string | null;
  }

  const repo = getCatalogRepository();

  let allModels = $state<BatchMatchableModel[]>([]);
  let loadingModels = $state(true);
  let rows = $state<BatchRow[]>([]);
  let dragOver = $state(false);
  let runningBatch = $state(false);
  let batchOrigin = $state<MediaOrigin>("manufacturer");
  let batchCredit = $state("");

  const MATCH_FIELD_LABELS: Record<Exclude<BatchMatchField, null>, string> = {
    sku: "SKU",
    model_code: "Código do modelo",
    ean: "EAN",
  };

  async function loadAllCatalogModels() {
    loadingModels = true;
    try {
      const collected: BatchMatchableModel[] = [];
      let cursor: string | null = null;
      do {
        const page = await repo.listEquipmentModels({
          limit: 100,
          cursor,
          active: null,
        });
        for (const item of page.items) {
          collected.push({
            id: item.model.id,
            name: item.model.name,
            sku: item.model.sku,
            model_code: item.model.model_code,
            ean: item.model.ean,
            brand_name: item.brand_name,
            category_name: item.category_name,
          });
        }
        cursor = page.next_cursor;
      } while (cursor);
      allModels = collected;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      loadingModels = false;
    }
  }

  $effect(() => {
    void loadAllCatalogModels();
  });

  function addFilesToBatch(fileList: FileList | File[]) {
    const incoming = Array.from(fileList);
    if (incoming.length === 0) return;

    const newRows: BatchRow[] = incoming.map((file) => {
      const match = matchFilenameToCatalogModels(file.name, allModels);
      return {
        id: crypto.randomUUID(),
        file,
        filename: file.name,
        initialStatus: match.status,
        matchedBy: match.matchedBy,
        selectedModelId: match.matchedModel?.id ?? "",
        candidates: match.candidates,
        manuallyCorrected: false,
        processState: "pending",
        processMessage: null,
      };
    });

    rows = [...rows, ...newRows];
  }

  function handleFileInput(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      addFilesToBatch(input.files);
      input.value = "";
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    if (runningBatch) return;
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      addFilesToBatch(e.dataTransfer.files);
    }
  }

  function handleManualModelChange(rowId: string, modelId: string) {
    rows = rows.map((r) =>
      r.id === rowId
        ? {
            ...r,
            selectedModelId: modelId,
            manuallyCorrected: true,
            processState: "pending",
            processMessage: null,
          }
        : r,
    );
  }

  function discardRow(rowId: string) {
    rows = rows.filter((r) => r.id !== rowId);
  }

  function effectiveStatus(row: BatchRow): BatchMatchStatus {
    if (row.selectedModelId) {
      return "matched";
    }
    return row.initialStatus === "ambiguous" ? "ambiguous" : "unmatched";
  }

  let readyCount = $derived(
    rows.filter((r) => r.selectedModelId && r.processState !== "success")
      .length,
  );
  let unresolvedCount = $derived(rows.filter((r) => !r.selectedModelId).length);
  let successCount = $derived(
    rows.filter((r) => r.processState === "success").length,
  );
  let errorCount = $derived(
    rows.filter((r) => r.processState === "error").length,
  );

  async function confirmBatchImport() {
    if (runningBatch || rows.length === 0) return;
    runningBatch = true;

    for (let i = 0; i < rows.length; i++) {
      const current = rows[i];
      if (current.processState === "success") continue;

      if (!current.selectedModelId) {
        rows[i] = {
          ...current,
          processState: "error",
          processMessage:
            "Nenhum modelo selecionado para este arquivo. Selecione manualmente ou descarte.",
        };
        continue;
      }

      rows[i] = {
        ...current,
        processState: "processing",
        processMessage: "Redimensionando e enviando...",
      };

      try {
        const resized = await resizeImageForModelUpload(current.file);
        const stem = extractFilenameStem(current.filename) || "Foto";
        const ext = resized.displayMimeType === "image/webp" ? "webp" : "jpg";

        await repo.attachModelPhoto({
          model_id: current.selectedModelId,
          title: stem.slice(0, 150),
          origin: batchOrigin,
          credit_note: batchCredit.trim() || null,
          original_filename: `${stem}.${ext}`,
          display_mime_type: resized.displayMimeType,
          display_bytes: resized.displayBytes,
          thumb_mime_type: resized.thumbMimeType,
          thumb_bytes: resized.thumbBytes,
        });

        rows[i] = {
          ...rows[i],
          processState: "success",
          processMessage: `Sucesso (${formatMediaByteSize(resized.displayBytes.byteLength)}, ${resized.displayWidth}×${resized.displayHeight} px)`,
        };
      } catch (err) {
        rows[i] = {
          ...rows[i],
          processState: "error",
          processMessage: err instanceof Error ? err.message : String(err),
        };
      }
    }

    runningBatch = false;
    toast.success(
      `Processamento do lote concluído: ${rows.filter((r) => r.processState === "success").length} sucesso(s), ${rows.filter((r) => r.processState === "error").length} erro(s).`,
    );
  }
</script>

<div class="mx-auto max-w-6xl space-y-6">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <Button
        variant="ghost"
        size="icon"
        href="/catalog"
        aria-label="Voltar para catálogo"
      >
        <ArrowLeft class="h-4 w-4" />
      </Button>
      <div>
        <h1 class="text-2xl font-semibold tracking-tight text-foreground">
          Importação em Lote de Fotos
        </h1>
        <p class="text-sm text-muted-foreground">
          O nome de cada arquivo (sem extensão) é casado automaticamente na
          ordem: 1º SKU, 2º Código do Modelo e 3º EAN.
        </p>
      </div>
    </div>
  </div>

  <div
    class="grid gap-4 rounded-xl border border-border bg-card p-5 shadow-xs sm:grid-cols-2"
  >
    <div>
      <label
        for="batch-origin-select"
        class="mb-1.5 block text-xs font-medium text-foreground"
      >
        Origem das fotos do lote
      </label>
      <select
        id="batch-origin-select"
        bind:value={batchOrigin}
        class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground"
      >
        <option value="manufacturer">Fabricante</option>
        <option value="own">Própria</option>
        <option value="website">Site</option>
        <option value="other">Outra</option>
      </select>
    </div>
    <div>
      <label
        for="batch-credit-input"
        class="mb-1.5 block text-xs font-medium text-foreground"
      >
        Crédito / Fonte do lote (opcional)
      </label>
      <Input
        id="batch-credit-input"
        bind:value={batchCredit}
        maxlength={300}
        placeholder="Ex.: Catálogo oficial do fabricante"
      />
    </div>
  </div>

  <div
    role="region"
    aria-label="Soltar arquivos para importação em lote"
    data-testid="batch-photo-dropzone"
    ondragover={(e) => {
      e.preventDefault();
      dragOver = true;
    }}
    ondragleave={() => {
      dragOver = false;
    }}
    ondrop={handleDrop}
    class="flex flex-col items-center justify-center rounded-xl border-2 border-dashed p-8 text-center transition-colors {dragOver
      ? 'border-primary bg-primary/10'
      : 'border-border bg-card'}"
  >
    <ImagePlus class="mb-2 h-9 w-9 text-muted-foreground" />
    <p class="text-sm font-medium text-foreground">
      Arraste vários arquivos de imagem aqui ou clique para selecionar
    </p>
    <p class="mt-1 text-xs text-muted-foreground">
      Ex.: <code class="rounded bg-muted px-1.5 py-0.5">PA-CDJ-3000.png</code>
      (SKU),
      <code class="rounded bg-muted px-1.5 py-0.5">CDJ-3000.jpg</code> (Código)
      ou
      <code class="rounded bg-muted px-1.5 py-0.5">7891234567895.webp</code> (EAN)
    </p>
    <div class="mt-4">
      <label
        class="inline-flex cursor-pointer items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
      >
        <Upload class="mr-2 h-4 w-4" />
        Selecionar arquivos
        <input
          type="file"
          accept="image/webp,image/jpeg,image/png"
          multiple
          class="sr-only"
          data-testid="batch-photo-input"
          disabled={loadingModels || runningBatch}
          onchange={handleFileInput}
        />
      </label>
    </div>
  </div>

  {#if rows.length > 0}
    <div
      class="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-border bg-card p-4 shadow-xs"
    >
      <div class="flex flex-wrap items-center gap-4 text-sm">
        <span class="font-medium text-foreground">
          {rows.length} arquivo{rows.length === 1 ? "" : "s"} na pré-visualização
        </span>
        <span class="text-muted-foreground">
          Prontos: <strong class="text-foreground">{readyCount}</strong>
        </span>
        {#if unresolvedCount > 0}
          <span class="text-destructive">
            Pendentes de correção/descarte: <strong>{unresolvedCount}</strong>
          </span>
        {/if}
        {#if successCount > 0 || errorCount > 0}
          <span data-testid="batch-summary-counts" class="text-xs font-medium">
            Concluídos: {successCount} sucesso(s) • {errorCount} erro(s)
          </span>
        {/if}
      </div>

      <div class="flex items-center gap-2">
        <Button
          variant="outline"
          disabled={runningBatch}
          onclick={() => {
            rows = [];
          }}
        >
          Limpar lista
        </Button>
        <Button
          data-testid="confirm-batch-import-btn"
          disabled={runningBatch || rows.length === 0}
          onclick={confirmBatchImport}
        >
          {runningBatch ? "Processando lote..." : "Confirmar importação"}
        </Button>
      </div>
    </div>

    <div
      class="overflow-x-auto rounded-xl border border-border bg-card shadow-xs"
    >
      <table class="w-full text-left text-sm" data-testid="batch-import-table">
        <thead
          class="border-b border-border bg-muted/50 text-xs uppercase text-muted-foreground"
        >
          <tr>
            <th class="px-4 py-3">Arquivo</th>
            <th class="px-4 py-3">Modelo encontrado</th>
            <th class="px-4 py-3">Situação</th>
            <th class="px-4 py-3">Resultado</th>
            <th class="px-4 py-3 text-right">Ações</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border">
          {#each rows as row (row.id)}
            {@const status = effectiveStatus(row)}
            <tr data-testid="batch-import-row">
              <td class="px-4 py-3">
                <div class="font-medium text-foreground">{row.filename}</div>
                <div class="text-xs text-muted-foreground">
                  {formatMediaByteSize(row.file.size)}
                </div>
              </td>

              <td class="px-4 py-3">
                <select
                  aria-label="Selecionar modelo para {row.filename}"
                  data-testid="batch-row-model-select"
                  value={row.selectedModelId}
                  disabled={runningBatch || row.processState === "success"}
                  onchange={(e) =>
                    handleManualModelChange(
                      row.id,
                      (e.currentTarget as HTMLSelectElement).value,
                    )}
                  class="flex h-9 w-full max-w-md rounded-md border border-input bg-background px-2.5 py-1 text-xs text-foreground"
                >
                  <option value="">— Selecione um modelo —</option>
                  {#each allModels as m (m.id)}
                    <option value={m.id}>
                      {m.name}
                      {m.sku ? ` [SKU: ${m.sku}]` : ""}
                      {m.model_code ? ` [Cód: ${m.model_code}]` : ""}
                      {m.ean ? ` [EAN: ${m.ean}]` : ""}
                    </option>
                  {/each}
                </select>
                {#if row.initialStatus === "ambiguous" && row.candidates.length > 1}
                  <div class="mt-1 text-[11px] text-muted-foreground">
                    Candidatos ambíguos: {row.candidates
                      .map((c) => c.name)
                      .join(", ")}
                  </div>
                {/if}
              </td>

              <td class="px-4 py-3" data-testid="batch-row-status">
                {#if status === "matched"}
                  <span
                    class="inline-flex items-center gap-1 rounded-md bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary"
                  >
                    <CheckCircle2 class="h-3.5 w-3.5" />
                    Casado
                    {#if row.manuallyCorrected}
                      (manual)
                    {:else if row.matchedBy}
                      (via {MATCH_FIELD_LABELS[row.matchedBy]})
                    {/if}
                  </span>
                {:else if status === "ambiguous"}
                  <span
                    class="inline-flex items-center gap-1 rounded-md bg-muted px-2 py-0.5 text-xs font-medium text-foreground"
                  >
                    <AlertTriangle class="h-3.5 w-3.5" />
                    Ambíguo
                  </span>
                {:else}
                  <span
                    class="inline-flex items-center gap-1 rounded-md bg-destructive/10 px-2 py-0.5 text-xs font-medium text-destructive"
                  >
                    <XCircle class="h-3.5 w-3.5" />
                    Sem correspondência
                  </span>
                {/if}
              </td>

              <td class="px-4 py-3 text-xs" data-testid="batch-row-result">
                {#if row.processState === "pending"}
                  <span class="text-muted-foreground"
                    >Aguardando confirmação</span
                  >
                {:else if row.processState === "processing"}
                  <span class="text-primary">{row.processMessage}</span>
                {:else if row.processState === "success"}
                  <span class="font-medium text-primary">
                    {row.processMessage}
                  </span>
                {:else}
                  <span class="font-medium text-destructive">
                    Erro: {row.processMessage}
                  </span>
                {/if}
              </td>

              <td class="px-4 py-3 text-right">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={runningBatch}
                  aria-label="Descartar arquivo {row.filename}"
                  onclick={() => discardRow(row.id)}
                >
                  <Trash2 class="mr-1 h-3.5 w-3.5 text-destructive" />
                  Descartar
                </Button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
