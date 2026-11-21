<script lang="ts">
  import {
    AlertCircle,
    ArrowDown,
    ArrowUp,
    ClipboardPaste,
    Download,
    ExternalLink,
    FileText,
    ImagePlus,
    Pencil,
    Star,
    Trash2,
    Upload,
  } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    getCatalogRepository,
    type EquipmentModelMediaView,
    type MediaDocType,
    type MediaOrigin,
  } from "$lib/data";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import {
    formatMediaByteSize,
    resizeImageForModelUpload,
  } from "$lib/utils/image-resize";
  import { mediaBlobUrlCache } from "$lib/utils/media-lru-cache";

  interface Props {
    modelId: string;
    modelName: string;
    mode: "photos" | "documents";
    readonly?: boolean;
  }

  let { modelId, modelName, mode, readonly = false }: Props = $props();

  const repo = getCatalogRepository();

  const ORIGIN_LABELS: Record<MediaOrigin, string> = {
    manufacturer: "Fabricante",
    own: "Própria",
    website: "Site",
    other: "Outra",
  };

  const DOC_TYPE_LABELS: Record<MediaDocType, string> = {
    manual: "Manual",
    datasheet: "Datasheet",
    certificate: "Certificado",
    other: "Outro",
  };

  let items = $state<EquipmentModelMediaView[]>([]);
  let loading = $state(true);
  let uploading = $state(false);
  let dragOver = $state(false);

  // Cache local de Blob URLs por blob_id para renderização reativa
  let blobUrls = $state<Record<string, string>>({});
  // Rastreia falhas ou arquivos ausentes no disco para exibir mensagem clara
  let failedBlobIds = $state<Record<string, string>>({});

  // Resumo do último redimensionamento de foto
  let lastResizeSummary = $state<string | null>(null);

  // Campos padrão para envio de nova foto
  let defaultPhotoOrigin = $state<MediaOrigin>("manufacturer");
  let defaultPhotoCredit = $state("");

  // Campos para envio de novo documento PDF
  let docFile = $state<File | null>(null);
  let docTitle = $state("");
  let docType = $state<MediaDocType>("manual");
  let docOrigin = $state<MediaOrigin>("manufacturer");
  let docVersion = $state("");
  let docLanguage = $state("pt-BR");
  let docCredit = $state("");

  // Estado de edição de metadados
  let editingId = $state<string | null>(null);
  let editTitle = $state("");
  let editOrigin = $state<MediaOrigin>("manufacturer");
  let editCredit = $state("");
  let editDocType = $state<MediaDocType>("manual");
  let editVersion = $state("");
  let editLanguage = $state("");

  // Estado de confirmação de remoção
  let confirmingDeleteId = $state<string | null>(null);

  let photos = $derived(
    items
      .filter((i) => i.kind === "image")
      .sort(
        (a, b) =>
          Number(b.is_primary) - Number(a.is_primary) ||
          a.sort_order - b.sort_order,
      ),
  );

  let documents = $derived(
    items
      .filter((i) => i.kind === "document")
      .sort((a, b) => a.sort_order - b.sort_order),
  );

  async function ensureBlobUrl(blobId: string, mimeType: string) {
    if (blobUrls[blobId]) return;
    const cached = mediaBlobUrlCache.get(blobId);
    if (cached) {
      blobUrls = { ...blobUrls, [blobId]: cached };
      return;
    }
    try {
      const url = await mediaBlobUrlCache.getOrLoad(blobId, mimeType, () =>
        repo.getMediaBlobBytes(blobId),
      );
      blobUrls = { ...blobUrls, [blobId]: url };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      failedBlobIds = {
        ...failedBlobIds,
        [blobId]: msg.includes("não encontrado")
          ? "Arquivo não encontrado no disco"
          : "Falha ao carregar imagem",
      };
    }
  }

  async function loadMedia() {
    if (!modelId) return;
    loading = true;
    try {
      items = await repo.listModelMedia(modelId);
      for (const item of items) {
        if (item.kind === "image") {
          void ensureBlobUrl(item.blob_id, item.mime_type);
          if (item.thumb_blob_id && item.thumb_mime_type) {
            void ensureBlobUrl(item.thumb_blob_id, item.thumb_mime_type);
          }
        }
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void modelId;
    void loadMedia();
  });

  function deriveTitleFromFilename(filename: string): string {
    const base = filename.replace(/\.[^.]+$/, "").trim();
    return base ? base.slice(0, 150) : modelName.slice(0, 150);
  }

  async function processAndUploadPhotoFiles(fileList: File[] | FileList) {
    if (readonly || uploading) return;
    const files = Array.from(fileList).filter((f) =>
      ["image/webp", "image/jpeg", "image/png"].includes(f.type),
    );
    if (files.length === 0) {
      toast.error("Selecione imagens nos formatos WebP, JPEG ou PNG.");
      return;
    }

    uploading = true;
    try {
      for (const file of files) {
        const resized = await resizeImageForModelUpload(file);
        const title = deriveTitleFromFilename(file.name || "foto.webp");
        const ext = resized.displayMimeType === "image/webp" ? "webp" : "jpg";
        const cleanBase =
          (file.name || "foto").replace(/\.[^.]+$/, "") || "foto";

        const created = await repo.attachModelPhoto({
          model_id: modelId,
          title,
          origin: defaultPhotoOrigin,
          credit_note: defaultPhotoCredit.trim() || null,
          original_filename: `${cleanBase}.${ext}`,
          display_mime_type: resized.displayMimeType,
          display_bytes: resized.displayBytes,
          thumb_mime_type: resized.thumbMimeType,
          thumb_bytes: resized.thumbBytes,
        });

        // Pré-popula o cache LRU com os bytes já em memória
        const displayUrl = mediaBlobUrlCache.put(
          created.blob_id,
          resized.displayBytes,
          resized.displayMimeType,
        );
        blobUrls = { ...blobUrls, [created.blob_id]: displayUrl };
        if (created.thumb_blob_id) {
          const thumbUrl = mediaBlobUrlCache.put(
            created.thumb_blob_id,
            resized.thumbBytes,
            resized.thumbMimeType,
          );
          blobUrls = { ...blobUrls, [created.thumb_blob_id]: thumbUrl };
        }

        lastResizeSummary = `${created.original_filename}: exibição ${formatMediaByteSize(resized.displayBytes.byteLength)} (${resized.displayWidth}×${resized.displayHeight} px) • miniatura ${formatMediaByteSize(resized.thumbBytes.byteLength)} (${resized.thumbWidth}×${resized.thumbHeight} px)`;
      }
      await loadMedia();
      toast.success(
        files.length === 1
          ? "Foto redimensionada e anexada com sucesso!"
          : `${files.length} fotos anexadas com sucesso!`,
      );
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      uploading = false;
    }
  }

  function handlePhotoInputChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      void processAndUploadPhotoFiles(input.files);
      input.value = "";
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    if (readonly || mode !== "photos") return;
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      void processAndUploadPhotoFiles(e.dataTransfer.files);
    }
  }

  function handlePaste(e: ClipboardEvent) {
    if (readonly || mode !== "photos") return;
    const target = e.target as HTMLElement | null;
    if (
      target &&
      (target.tagName === "INPUT" || target.tagName === "TEXTAREA") &&
      !target.hasAttribute("data-paste-zone")
    ) {
      return;
    }

    const pastedFiles: File[] = [];
    if (e.clipboardData?.files && e.clipboardData.files.length > 0) {
      for (const f of Array.from(e.clipboardData.files)) {
        if (f.type.startsWith("image/")) {
          pastedFiles.push(f);
        }
      }
    } else if (e.clipboardData?.items) {
      for (const item of Array.from(e.clipboardData.items)) {
        if (item.kind === "file" && item.type.startsWith("image/")) {
          const f = item.getAsFile();
          if (f) pastedFiles.push(f);
        }
      }
    }

    if (pastedFiles.length > 0) {
      e.preventDefault();
      void processAndUploadPhotoFiles(pastedFiles);
    }
  }

  function handleDocFileChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const f = input.files?.[0] ?? null;
    docFile = f;
    if (f && !docTitle.trim()) {
      docTitle = deriveTitleFromFilename(f.name);
    }
  }

  async function handleAttachDocument(e: SubmitEvent) {
    e.preventDefault();
    if (readonly || uploading || !docFile) return;

    uploading = true;
    try {
      const buf = await docFile.arrayBuffer();
      const bytes = new Uint8Array(buf);
      await repo.attachModelDocument({
        model_id: modelId,
        title: docTitle.trim() || deriveTitleFromFilename(docFile.name),
        doc_type: docType,
        origin: docOrigin,
        credit_note: docCredit.trim() || null,
        language: docLanguage.trim() || null,
        version_note: docVersion.trim() || null,
        original_filename: docFile.name || "documento.pdf",
        mime_type: "application/pdf",
        bytes,
      });
      docFile = null;
      docTitle = "";
      docVersion = "";
      docCredit = "";
      await loadMedia();
      toast.success("Documento PDF anexado com sucesso!");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      uploading = false;
    }
  }

  function startEdit(item: EquipmentModelMediaView) {
    editingId = item.id;
    editTitle = item.title;
    editOrigin = item.origin;
    editCredit = item.credit_note ?? "";
    editDocType = item.doc_type ?? "manual";
    editVersion = item.version_note ?? "";
    editLanguage = item.language ?? "";
  }

  async function saveEdit(item: EquipmentModelMediaView) {
    if (readonly) return;
    try {
      await repo.updateModelMediaMetadata({
        media_link_id: item.id,
        title: editTitle.trim(),
        doc_type: item.kind === "document" ? editDocType : null,
        origin: editOrigin,
        credit_note: editCredit.trim() || null,
        language: item.kind === "document" ? editLanguage.trim() || null : null,
        version_note:
          item.kind === "document" ? editVersion.trim() || null : null,
        expected_updated_at: item.updated_at,
      });
      editingId = null;
      await loadMedia();
      toast.success("Metadados atualizados.");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleSetPrimary(photoId: string) {
    if (readonly) return;
    try {
      await repo.setPrimaryModelPhoto(photoId);
      await loadMedia();
      toast.success("Foto principal definida.");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleMovePhoto(index: number, direction: -1 | 1) {
    if (readonly) return;
    const targetIdx = index + direction;
    if (targetIdx < 0 || targetIdx >= photos.length) return;
    const currentOrder = photos.map((p) => p.id);
    const [moved] = currentOrder.splice(index, 1);
    currentOrder.splice(targetIdx, 0, moved);

    try {
      await repo.reorderModelMedia({
        model_id: modelId,
        kind: "image",
        ordered_link_ids: currentOrder,
      });
      await loadMedia();
      toast.success("Ordem das fotos atualizada.");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleConfirmRemove(item: EquipmentModelMediaView) {
    if (readonly) return;
    try {
      await repo.removeModelMedia(item.id);
      confirmingDeleteId = null;
      await loadMedia();
      toast.success(
        item.kind === "image" ? "Foto removida." : "Documento removido.",
      );
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleOpenDocument(doc: EquipmentModelMediaView) {
    try {
      const tempPath = await repo.openModelDocumentExternal(doc.id);
      toast.success(`Documento aberto no leitor do sistema (${tempPath}).`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleSaveDocumentAs(doc: EquipmentModelMediaView) {
    try {
      const savedPath = await repo.saveModelDocumentAs(doc.id);
      if (savedPath) {
        toast.success(`Documento salvo em: ${savedPath}`);
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }
</script>

<svelte:window onpaste={handlePaste} />

{#if mode === "photos"}
  <div class="space-y-6" data-testid="model-photos-section">
    <div
      class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-card p-4 shadow-xs"
    >
      <div>
        <h2 class="text-base font-semibold text-foreground">
          Galeria de Fotos do Modelo
        </h2>
        <p class="text-xs text-muted-foreground">
          {photos.length} de 12 fotos utilizadas • Redimensionamento automático no
          navegador (exibição até 1280 px, miniatura 256 px, WebP/JPEG sem EXIF)
        </p>
      </div>
      {#if lastResizeSummary}
        <div
          class="rounded-md bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary"
          data-testid="photo-final-size-summary"
        >
          Tamanho final: {lastResizeSummary}
        </div>
      {/if}
    </div>

    {#if !readonly}
      <div
        class="space-y-4 rounded-xl border border-border bg-card p-5 shadow-xs"
      >
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <label
              for="photo-origin-select"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Origem padrão para novos envios
            </label>
            <select
              id="photo-origin-select"
              bind:value={defaultPhotoOrigin}
              class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <option value="manufacturer">Fabricante</option>
              <option value="own">Própria</option>
              <option value="website">Site</option>
              <option value="other">Outra</option>
            </select>
          </div>
          <div>
            <label
              for="photo-credit-input"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Crédito / Fonte (opcional, até 300 caracteres)
            </label>
            <Input
              id="photo-credit-input"
              bind:value={defaultPhotoCredit}
              maxlength={300}
              placeholder="Ex.: Divulgação oficial / Foto acervo próprio"
            />
          </div>
        </div>

        <div
          role="region"
          aria-label="Área de envio e colagem de fotos"
          data-testid="photo-drop-paste-zone"
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
            : 'border-border bg-muted/40'}"
        >
          <ImagePlus class="mb-2 h-8 w-8 text-muted-foreground" />
          <p class="text-sm font-medium text-foreground">
            Arraste e solte fotos aqui, cole da área de transferência (Ctrl+V)
            ou selecione arquivos
          </p>
          <p class="mt-1 text-xs text-muted-foreground">
            Formatos aceitos: WebP, JPEG ou PNG (máximo de 12 fotos por modelo)
          </p>
          <div class="mt-4 flex flex-wrap items-center justify-center gap-3">
            <label
              class="inline-flex cursor-pointer items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
            >
              <Upload class="mr-2 h-4 w-4" />
              Selecionar fotos
              <input
                type="file"
                accept="image/webp,image/jpeg,image/png"
                multiple
                class="sr-only"
                data-testid="photo-file-input"
                disabled={uploading || photos.length >= 12}
                onchange={handlePhotoInputChange}
              />
            </label>
            <span
              class="inline-flex items-center gap-1.5 rounded-md border border-border bg-background px-3 py-2 text-xs text-muted-foreground"
            >
              <ClipboardPaste class="h-3.5 w-3.5" />
              Pressione Ctrl+V para colar imagem
            </span>
          </div>
        </div>
      </div>
    {/if}

    {#if loading}
      <div
        class="rounded-xl border border-border bg-card p-8 text-center text-sm text-muted-foreground"
      >
        Carregando fotos...
      </div>
    {:else if photos.length === 0}
      <div
        class="rounded-xl border border-dashed border-border bg-card p-10 text-center text-sm text-muted-foreground"
      >
        Nenhuma foto cadastrada para este modelo.
      </div>
    {:else}
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {#each photos as photo, idx (photo.id)}
          {@const imgUrl =
            blobUrls[photo.blob_id] ??
            (photo.thumb_blob_id ? blobUrls[photo.thumb_blob_id] : undefined)}
          <div
            class="flex flex-col overflow-hidden rounded-xl border border-border bg-card shadow-xs"
            data-testid="photo-gallery-card"
          >
            <div
              class="relative flex h-48 w-full items-center justify-center bg-muted p-2"
            >
              {#if imgUrl}
                <img
                  src={imgUrl}
                  alt={modelName}
                  class="max-h-full max-w-full object-contain"
                />
              {:else if failedBlobIds[photo.blob_id] || (photo.thumb_blob_id && failedBlobIds[photo.thumb_blob_id])}
                <div
                  class="flex flex-col items-center justify-center p-3 text-center text-xs text-destructive"
                  data-testid="media-file-missing"
                >
                  <AlertCircle class="mb-1 h-5 w-5" />
                  <span class="font-medium">
                    {failedBlobIds[photo.blob_id] ??
                      (photo.thumb_blob_id
                        ? failedBlobIds[photo.thumb_blob_id]
                        : "Arquivo não encontrado no disco")}
                  </span>
                </div>
              {:else}
                <span class="text-xs text-muted-foreground">Carregando...</span>
              {/if}

              {#if photo.is_primary}
                <span
                  class="absolute top-2 left-2 inline-flex items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground shadow-xs"
                  data-testid="primary-photo-badge"
                >
                  <Star class="h-3 w-3" />
                  Principal
                </span>
              {/if}
            </div>

            <div class="flex flex-1 flex-col justify-between space-y-3 p-4">
              {#if editingId === photo.id && !readonly}
                <div class="space-y-2">
                  <Input
                    bind:value={editTitle}
                    maxlength={150}
                    aria-label="Título da foto"
                    placeholder="Título da foto"
                  />
                  <select
                    bind:value={editOrigin}
                    aria-label="Origem da foto"
                    class="flex h-9 w-full rounded-md border border-input bg-background px-2 py-1 text-xs text-foreground"
                  >
                    <option value="manufacturer">Fabricante</option>
                    <option value="own">Própria</option>
                    <option value="website">Site</option>
                    <option value="other">Outra</option>
                  </select>
                  <Input
                    bind:value={editCredit}
                    maxlength={300}
                    aria-label="Crédito da foto"
                    placeholder="Crédito / fonte"
                  />
                  <div class="flex items-center gap-2 pt-1">
                    <Button size="sm" onclick={() => saveEdit(photo)}>
                      Salvar
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      onclick={() => {
                        editingId = null;
                      }}
                    >
                      Cancelar
                    </Button>
                  </div>
                </div>
              {:else}
                <div>
                  <div class="font-medium text-foreground">{photo.title}</div>
                  <div class="mt-1 space-y-0.5 text-xs text-muted-foreground">
                    <div>
                      Origem: <span class="text-foreground"
                        >{ORIGIN_LABELS[photo.origin]}</span
                      >
                    </div>
                    {#if photo.credit_note}
                      <div>Crédito: {photo.credit_note}</div>
                    {/if}
                    <div data-testid="photo-card-size">
                      Tamanho: {formatMediaByteSize(photo.byte_size)}
                      {#if photo.width && photo.height}
                        • {photo.width}×{photo.height} px
                      {/if}
                    </div>
                  </div>
                </div>
              {/if}

              {#if !readonly}
                <div
                  class="flex flex-wrap items-center justify-between gap-1.5 border-t border-border pt-3"
                >
                  <div class="flex items-center gap-1">
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={idx === 0}
                      aria-label="Mover foto para trás"
                      onclick={() => handleMovePhoto(idx, -1)}
                    >
                      <ArrowUp class="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={idx === photos.length - 1}
                      aria-label="Mover foto para frente"
                      onclick={() => handleMovePhoto(idx, 1)}
                    >
                      <ArrowDown class="h-3.5 w-3.5" />
                    </Button>
                    {#if !photo.is_primary}
                      <Button
                        size="sm"
                        variant="outline"
                        onclick={() => handleSetPrimary(photo.id)}
                      >
                        <Star class="mr-1 h-3.5 w-3.5" />
                        Definir principal
                      </Button>
                    {/if}
                  </div>

                  <div class="flex items-center gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="Editar metadados da foto"
                      onclick={() => startEdit(photo)}
                    >
                      <Pencil class="h-3.5 w-3.5" />
                    </Button>
                    {#if confirmingDeleteId === photo.id}
                      <Button
                        size="sm"
                        variant="destructive"
                        onclick={() => handleConfirmRemove(photo)}
                      >
                        Confirmar
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onclick={() => {
                          confirmingDeleteId = null;
                        }}
                      >
                        Não
                      </Button>
                    {:else}
                      <Button
                        size="sm"
                        variant="ghost"
                        aria-label="Remover foto"
                        onclick={() => {
                          confirmingDeleteId = photo.id;
                        }}
                      >
                        <Trash2 class="h-3.5 w-3.5 text-destructive" />
                      </Button>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{:else}
  <div class="space-y-6" data-testid="model-documents-section">
    <div
      class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-card p-4 shadow-xs"
    >
      <div>
        <h2 class="text-base font-semibold text-foreground">
          Documentos Técnicos (PDF)
        </h2>
        <p class="text-xs text-muted-foreground">
          {documents.length} de 20 documentos utilizados • PDFs nunca são interpretados
          dentro do aplicativo; a leitura ocorre no visualizador do sistema
        </p>
      </div>
    </div>

    {#if !readonly}
      <form
        onsubmit={handleAttachDocument}
        class="space-y-4 rounded-xl border border-border bg-card p-5 shadow-xs"
      >
        <h3 class="text-sm font-semibold text-foreground">
          Anexar Documento PDF (até 25 MB)
        </h3>
        <div class="grid gap-4 sm:grid-cols-3">
          <div>
            <label
              for="doc-file-input"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Arquivo PDF *
            </label>
            <input
              id="doc-file-input"
              type="file"
              accept="application/pdf,.pdf"
              required
              data-testid="document-file-input"
              onchange={handleDocFileChange}
              class="block w-full text-xs text-foreground file:mr-3 file:rounded-md file:border-0 file:bg-primary file:px-3 file:py-2 file:text-xs file:font-medium file:text-primary-foreground"
            />
          </div>
          <div>
            <label
              for="doc-title-input"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Título *
            </label>
            <Input
              id="doc-title-input"
              bind:value={docTitle}
              maxlength={150}
              required
              placeholder="Ex.: Manual de Operação CDJ-3000"
            />
          </div>
          <div>
            <label
              for="doc-type-select"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Tipo *
            </label>
            <select
              id="doc-type-select"
              bind:value={docType}
              class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground"
            >
              <option value="manual">Manual</option>
              <option value="datasheet">Datasheet</option>
              <option value="certificate">Certificado</option>
              <option value="other">Outro</option>
            </select>
          </div>
          <div>
            <label
              for="doc-version-input"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Versão (opcional)
            </label>
            <Input
              id="doc-version-input"
              bind:value={docVersion}
              maxlength={300}
              placeholder="Ex.: Rev 2.1 (2025)"
            />
          </div>
          <div>
            <label
              for="doc-origin-select"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Origem
            </label>
            <select
              id="doc-origin-select"
              bind:value={docOrigin}
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
              for="doc-credit-input"
              class="mb-1.5 block text-xs font-medium text-foreground"
            >
              Crédito / Observação
            </label>
            <Input
              id="doc-credit-input"
              bind:value={docCredit}
              maxlength={300}
              placeholder="Ex.: Suporte Oficial Pioneer DJ"
            />
          </div>
        </div>
        <div class="flex justify-end">
          <Button
            type="submit"
            disabled={uploading || !docFile || documents.length >= 20}
          >
            <FileText class="mr-2 h-4 w-4" />
            Anexar PDF
          </Button>
        </div>
      </form>
    {/if}

    {#if loading}
      <div
        class="rounded-xl border border-border bg-card p-8 text-center text-sm text-muted-foreground"
      >
        Carregando documentos...
      </div>
    {:else if documents.length === 0}
      <div
        class="rounded-xl border border-dashed border-border bg-card p-10 text-center text-sm text-muted-foreground"
      >
        Nenhum documento PDF anexado a este modelo.
      </div>
    {:else}
      <div
        class="overflow-x-auto rounded-xl border border-border bg-card shadow-xs"
      >
        <table class="w-full text-left text-sm">
          <thead
            class="border-b border-border bg-muted/50 text-xs uppercase text-muted-foreground"
          >
            <tr>
              <th class="px-4 py-3">Título</th>
              <th class="px-4 py-3">Tipo</th>
              <th class="px-4 py-3">Versão</th>
              <th class="px-4 py-3">Origem</th>
              <th class="px-4 py-3">Tamanho</th>
              <th class="px-4 py-3 text-right">Ações</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            {#each documents as doc (doc.id)}
              <tr data-testid="document-table-row">
                <td class="px-4 py-3 font-medium text-foreground">
                  {#if editingId === doc.id && !readonly}
                    <Input
                      bind:value={editTitle}
                      maxlength={150}
                      aria-label="Editar título do documento"
                    />
                  {:else}
                    <div>{doc.title}</div>
                    <div class="text-xs text-muted-foreground">
                      {doc.original_filename}
                    </div>
                  {/if}
                </td>
                <td class="px-4 py-3 text-muted-foreground">
                  {#if editingId === doc.id && !readonly}
                    <select
                      bind:value={editDocType}
                      class="rounded border border-input bg-background px-2 py-1 text-xs text-foreground"
                    >
                      <option value="manual">Manual</option>
                      <option value="datasheet">Datasheet</option>
                      <option value="certificate">Certificado</option>
                      <option value="other">Outro</option>
                    </select>
                  {:else}
                    {doc.doc_type ? DOC_TYPE_LABELS[doc.doc_type] : "Outro"}
                  {/if}
                </td>
                <td class="px-4 py-3 text-muted-foreground">
                  {#if editingId === doc.id && !readonly}
                    <Input
                      bind:value={editVersion}
                      maxlength={300}
                      placeholder="Versão"
                    />
                  {:else}
                    {doc.version_note || "—"}
                  {/if}
                </td>
                <td class="px-4 py-3 text-muted-foreground">
                  {#if editingId === doc.id && !readonly}
                    <select
                      bind:value={editOrigin}
                      class="rounded border border-input bg-background px-2 py-1 text-xs text-foreground"
                    >
                      <option value="manufacturer">Fabricante</option>
                      <option value="own">Própria</option>
                      <option value="website">Site</option>
                      <option value="other">Outra</option>
                    </select>
                  {:else}
                    {ORIGIN_LABELS[doc.origin]}
                  {/if}
                </td>
                <td class="px-4 py-3 text-muted-foreground">
                  {formatMediaByteSize(doc.byte_size)}
                </td>
                <td class="px-4 py-3 text-right">
                  <div
                    class="inline-flex flex-wrap items-center justify-end gap-1.5"
                  >
                    {#if editingId === doc.id && !readonly}
                      <Button size="sm" onclick={() => saveEdit(doc)}>
                        Salvar
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onclick={() => {
                          editingId = null;
                        }}
                      >
                        Cancelar
                      </Button>
                    {:else}
                      <Button
                        size="sm"
                        variant="outline"
                        onclick={() => handleOpenDocument(doc)}
                      >
                        <ExternalLink class="mr-1 h-3.5 w-3.5" />
                        Abrir
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onclick={() => handleSaveDocumentAs(doc)}
                      >
                        <Download class="mr-1 h-3.5 w-3.5" />
                        Salvar como...
                      </Button>
                      {#if !readonly}
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label="Editar documento"
                          onclick={() => startEdit(doc)}
                        >
                          <Pencil class="h-3.5 w-3.5" />
                        </Button>
                        {#if confirmingDeleteId === doc.id}
                          <Button
                            size="sm"
                            variant="destructive"
                            onclick={() => handleConfirmRemove(doc)}
                          >
                            Confirmar
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            onclick={() => {
                              confirmingDeleteId = null;
                            }}
                          >
                            Não
                          </Button>
                        {:else}
                          <Button
                            size="sm"
                            variant="ghost"
                            aria-label="Remover documento"
                            onclick={() => {
                              confirmingDeleteId = doc.id;
                            }}
                          >
                            <Trash2 class="h-3.5 w-3.5 text-destructive" />
                          </Button>
                        {/if}
                      {/if}
                    {/if}
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
{/if}
