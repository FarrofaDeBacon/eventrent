<script lang="ts">
  import type { Snippet } from 'svelte';
  import { FolderSearch } from 'lucide-svelte';
  import { _ } from '$lib/i18n';
  import { cn } from '../utils';

  interface Props {
    title?: string;
    description?: string;
    class?: string;
    action?: Snippet;
  }

  let { title, description, class: className = '', action }: Props = $props();
</script>

<div class={cn('flex min-h-[280px] flex-col items-center justify-center rounded-[var(--radius)] border border-dashed border-border p-8 text-center', className)}>
  <div class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-muted text-muted-foreground">
    <FolderSearch class="h-6 w-6" />
  </div>
  <h3 class="mt-4 text-base font-semibold text-foreground">
    {title ?? $_('common.emptyTitle', { default: 'Nenhum item encontrado' })}
  </h3>
  <p class="mt-2 mb-4 max-w-sm text-sm text-muted-foreground">
    {description ?? $_('common.emptyMessage', { default: 'Não há dados para exibir no momento.' })}
  </p>
  {#if action}
    {@render action()}
  {/if}
</div>
