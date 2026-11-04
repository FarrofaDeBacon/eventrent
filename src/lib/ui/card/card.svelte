<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';
  import type { Snippet } from 'svelte';
  import { cn } from '../utils';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children?: Snippet;
  }

  let { class: className = '', children, ...restProps }: Props = $props();
</script>

<div
  class={cn('rounded-[var(--radius)] border border-border bg-card text-card-foreground shadow-xs', className)}
  {...restProps}
>
  {#if children}
    {@render children()}
  {/if}
</div>

