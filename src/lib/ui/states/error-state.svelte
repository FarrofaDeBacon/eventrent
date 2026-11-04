<script lang="ts">
  import { AlertTriangle } from 'lucide-svelte';
  import { Button } from '../button';
  import { cn } from '../utils';

  interface Props {
    title?: string;
    message?: string;
    onRetry?: () => void;
    class?: string;
  }

  let {
    title = 'Ocorreu um erro',
    message = 'Não foi possível carregar os dados. Verifique a conexão local e tente novamente.',
    onRetry,
    class: className = ''
  }: Props = $props();
</script>

<div class={cn('flex min-h-[260px] flex-col items-center justify-center rounded-[var(--radius)] border border-destructive/20 bg-destructive/5 p-8 text-center', className)}>
  <div class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10 text-destructive">
    <AlertTriangle class="h-6 w-6" />
  </div>
  <h3 class="mt-4 text-base font-semibold text-destructive">{title}</h3>
  <p class="mt-2 mb-4 max-w-sm text-sm text-muted-foreground">{message}</p>
  {#if onRetry}
    <Button variant="outline" size="sm" onclick={onRetry}>
      Tentar Novamente
    </Button>
  {/if}
</div>
