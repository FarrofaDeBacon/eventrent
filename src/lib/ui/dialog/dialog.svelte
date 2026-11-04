<script lang="ts">
  import { Dialog as BitsDialog } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { X } from 'lucide-svelte';

  interface Props {
    open?: boolean;
    title?: string;
    description?: string;
    children?: Snippet;
    trigger?: Snippet;
    onOpenChange?: (open: boolean) => void;
  }

  let {
    open = $bindable(false),
    title,
    description,
    children,
    trigger,
    onOpenChange
  }: Props = $props();
</script>

<BitsDialog.Root bind:open {onOpenChange}>
  {#if trigger}
    <BitsDialog.Trigger>
      {@render trigger()}
    </BitsDialog.Trigger>
  {/if}

  <BitsDialog.Portal>
    <BitsDialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs transition-opacity animate-in fade-in-0" />
    <BitsDialog.Content class="fixed left-1/2 top-1/2 z-50 grid w-full max-w-lg -translate-x-1/2 -translate-y-1/2 gap-4 border border-border bg-card p-6 shadow-lg duration-200 rounded-[var(--radius)] sm:max-w-[425px]">
      {#if title}
        <div class="flex flex-col space-y-1.5 text-center sm:text-left">
          <BitsDialog.Title class="text-lg font-semibold leading-none tracking-tight text-foreground">
            {title}
          </BitsDialog.Title>
          {#if description}
            <BitsDialog.Description class="text-sm text-muted-foreground">
              {description}
            </BitsDialog.Description>
          {/if}
        </div>
      {/if}

      {#if children}
        {@render children()}
      {/if}

      <BitsDialog.Close class="absolute right-4 top-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 cursor-pointer">
        <X class="h-4 w-4" />
        <span class="sr-only">Fechar</span>
      </BitsDialog.Close>
    </BitsDialog.Content>
  </BitsDialog.Portal>
</BitsDialog.Root>
