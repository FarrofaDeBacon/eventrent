<script lang="ts">
  import { page } from "$app/state";
</script>

<div
  class="flex min-h-screen w-full flex-col items-center justify-center bg-background p-6 font-mono text-foreground"
>
  <div
    class="w-full max-w-2xl rounded-xl border border-destructive/40 bg-card p-6 shadow-lg"
  >
    <h1 class="mb-2 text-2xl font-bold text-destructive">
      Erro {page.status} no SvelteKit
    </h1>
    <p class="mb-1 text-sm font-semibold">
      <strong>Mensagem:</strong>
      {page.error?.message ?? "Erro interno desconhecido"}
    </p>
    <p class="mb-4 text-xs text-muted-foreground">
      <strong>URL:</strong>
      {page.url.href}
    </p>

    {#if page.error && typeof page.error === "object" && "stack" in page.error && page.error.stack}
      <pre
        class="max-h-80 overflow-auto rounded bg-muted p-4 text-xs leading-relaxed text-foreground whitespace-pre-wrap">{page
          .error.stack}</pre>
    {/if}
  </div>
</div>
