<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import { cn } from '../utils';

  interface Props extends HTMLInputAttributes {
    value?: string | number;
  }

  let {
    value = $bindable(''),
    class: className = '',
    type = 'text',
    ...restProps
  }: Props = $props();
</script>

<input
  {type}
  bind:value
  class={cn(
    'flex h-9 w-full rounded-[var(--radius)] border border-input bg-card px-3 py-1 text-sm shadow-xs transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-50 text-foreground',
    className
  )}
  {...restProps}
/>

