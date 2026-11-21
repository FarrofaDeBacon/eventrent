<script lang="ts">
  import { Speaker } from "lucide-svelte";
  import { getCatalogRepository } from "$lib/data";
  import { mediaBlobUrlCache } from "$lib/utils/media-lru-cache";

  interface Props {
    blobId: string | null | undefined;
    mimeType?: string | null | undefined;
    alt: string;
  }

  let { blobId, mimeType = "image/webp", alt }: Props = $props();

  const repo = getCatalogRepository();
  let blobUrl = $state<string | null>(null);

  $effect(() => {
    const currentBlobId = blobId;
    const currentMime = mimeType || "image/webp";
    if (!currentBlobId) {
      blobUrl = null;
      return;
    }

    const cached = mediaBlobUrlCache.get(currentBlobId);
    if (cached) {
      blobUrl = cached;
      return;
    }

    let cancelled = false;
    void mediaBlobUrlCache
      .getOrLoad(currentBlobId, currentMime, () =>
        repo.getMediaBlobBytes(currentBlobId),
      )
      .then((url) => {
        if (!cancelled) {
          blobUrl = url;
        }
      })
      .catch(() => {
        if (!cancelled) {
          blobUrl = null;
        }
      });

    return () => {
      cancelled = true;
    };
  });
</script>

<div
  class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-muted"
  data-testid="catalog-list-thumb"
>
  {#if blobUrl}
    <img
      src={blobUrl}
      {alt}
      loading="lazy"
      class="h-full w-full object-contain"
    />
  {:else}
    <Speaker class="h-5 w-5 text-muted-foreground/60" aria-hidden="true" />
  {/if}
</div>
