<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';
  import type { Snippet } from 'svelte';
  import { cn } from '../utils';

  interface Props extends HTMLAttributes<HTMLHeadingElement> {
    children?: Snippet;
  }

  let { class: className = '', children, ...restProps }: Props = $props();
</script>

<h3 class={cn('text-lg font-semibold leading-none tracking-tight text-foreground', className)} {...restProps}>
  {#if children}
    {@render children()}
  {/if}
</h3>
