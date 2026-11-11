<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  interface Props {
    items: T[];
    itemHeight?: number;
    height?: number;
    overscan?: number;
    getKey?: (item: T, index: number) => string | number;
    onEndReached?: () => void;
    endReachedThreshold?: number;
    children: Snippet<[T, number]>;
  }

  let {
    items,
    itemHeight = 72,
    height = 560,
    overscan = 5,
    getKey = (_item: T, index: number) => index,
    onEndReached,
    endReachedThreshold = 200,
    children,
  }: Props = $props();

  let scrollTop = $state(0);
  let viewportEl: HTMLDivElement | undefined = $state();
  let measuredHeight = $state(0);
  let lastNotifiedLength = -1;

  let viewportHeight = $derived(measuredHeight > 0 ? measuredHeight : height);
  let totalHeight = $derived(items.length * itemHeight);
  let startIndex = $derived(
    Math.max(0, Math.floor(scrollTop / itemHeight) - overscan),
  );
  let visibleCount = $derived(
    Math.ceil(viewportHeight / itemHeight) + overscan * 2,
  );
  let endIndex = $derived(Math.min(items.length, startIndex + visibleCount));
  let visibleSlice = $derived(items.slice(startIndex, endIndex));
  let offsetTop = $derived(startIndex * itemHeight);

  function handleScroll(event: Event) {
    const target = event.currentTarget as HTMLDivElement;
    scrollTop = target.scrollTop;
    if (target.clientHeight > 0 && target.clientHeight !== measuredHeight) {
      measuredHeight = target.clientHeight;
    }

    if (onEndReached && items.length > 0) {
      const distanceFromBottom =
        target.scrollHeight - (target.scrollTop + target.clientHeight);
      if (
        distanceFromBottom <= endReachedThreshold &&
        lastNotifiedLength !== items.length
      ) {
        lastNotifiedLength = items.length;
        onEndReached();
      }
    }
  }

  $effect(() => {
    if (!viewportEl) return;
    if (viewportEl.clientHeight > 0) {
      measuredHeight = viewportEl.clientHeight;
    }
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.height > 0) {
          measuredHeight = entry.contentRect.height;
        }
      }
    });
    observer.observe(viewportEl);
    return () => observer.disconnect();
  });
</script>

<div
  bind:this={viewportEl}
  onscroll={handleScroll}
  style={`height: ${height}px; overflow-y: auto; position: relative;`}
  class="w-full rounded-[var(--radius)] border border-border bg-card"
  data-testid="virtual-list-viewport"
>
  <div style={`height: ${totalHeight}px; position: relative; width: 100%;`}>
    <div
      style={`position: absolute; top: 0; left: 0; right: 0; transform: translateY(${offsetTop}px);`}
      data-testid="virtual-list-window"
    >
      {#each visibleSlice as item, localIdx (getKey(item, startIndex + localIdx))}
        {@const globalIdx = startIndex + localIdx}
        <div
          style={`height: ${itemHeight}px;`}
          class="border-b border-border last:border-b-0"
          data-virtual-index={globalIdx}
        >
          {@render children(item, globalIdx)}
        </div>
      {/each}
    </div>
  </div>
</div>
