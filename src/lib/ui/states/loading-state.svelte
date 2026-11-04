<script lang="ts">
  import { Loader2 } from 'lucide-svelte';
  import { _ } from '$lib/i18n';
  import { cn } from '../utils';

  interface Props {
    message?: string;
    class?: string;
  }

  let { message, class: className = '' }: Props = $props();
</script>

<div class={cn('flex min-h-[220px] flex-col items-center justify-center space-y-3 p-8 text-center', className)}>
  <Loader2 class="h-8 w-8 animate-spin text-primary" />
  <p class="text-sm font-medium text-muted-foreground animate-pulse">
    {message ?? $_('common.loading', { default: 'Carregando informações...' })}
  </p>
</div>
