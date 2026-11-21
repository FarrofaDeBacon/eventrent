<script lang="ts">
  import { Button } from "$lib/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
  } from "$lib/ui/card";
  import { Input } from "$lib/ui/input";
  import { toast } from "svelte-sonner";
  import { _ } from "$lib/i18n";
  import {
    Copy,
    Download,
    KeyRound,
    Printer,
    ShieldAlert,
  } from "lucide-svelte";

  interface Props {
    open: boolean;
    recoveryKey: string;
    title?: string;
    description?: string;
    onclose: () => void;
  }

  let {
    open,
    recoveryKey,
    title = $_("auth.recoveryModalTitle"),
    description = $_("auth.recoveryModalDesc"),
    onclose,
  }: Props = $props();

  let typedConfirmation = $state("");
  let isConfirmed = $derived(typedConfirmation.trim() === "Guardei a chave");

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(recoveryKey);
      toast.success($_("setup.copiedToast"));
    } catch {
      toast.info(recoveryKey);
    }
  }

  function handleDownloadTxt() {
    const text = [
      "===========================================================",
      "EVENTRENT - NOVA CHAVE DE RECUPERAÇÃO DE CONTA",
      "===========================================================",
      "",
      `Data/Hora: ${new Date().toLocaleString("pt-BR")}`,
      "",
      "CHAVE DE RECUPERAÇÃO:",
      recoveryKey,
      "",
      "ATENÇÃO:",
      "- Esta chave é a ÚNICA forma de recuperar o acesso à sua conta caso você perca a senha.",
      "- A chave anterior foi REVOGADA imediatamente e não possui mais validade.",
      "- Guarde este arquivo em local seguro, impresso em cofre ou gerenciador de senhas.",
      "- Nunca compartilhe esta chave com terceiros ou operadores do sistema.",
      "",
      "===========================================================",
    ].join("\n");

    const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `eventrent-chave-recuperacao-${new Date().toISOString().slice(0, 10)}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success($_("auth.downloadTxtSuccessToast"));
  }

  function handlePrint() {
    window.print();
  }

  function handleComplete() {
    if (!isConfirmed) return;
    typedConfirmation = "";
    onclose();
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm p-4 overflow-y-auto"
    role="dialog"
    aria-modal="true"
    aria-labelledby="recovery-modal-title"
  >
    <Card
      class="w-full max-w-lg border-primary/40 shadow-2xl animate-in fade-in-0 zoom-in-95 duration-200"
    >
      <CardHeader class="space-y-2 pb-4">
        <div class="flex items-center gap-3">
          <div
            class="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10 text-primary shrink-0"
          >
            <KeyRound class="h-5 w-5" />
          </div>
          <div>
            <CardTitle id="recovery-modal-title" class="text-xl">
              {title}
            </CardTitle>
            <CardDescription class="text-xs mt-1">
              {description}
            </CardDescription>
          </div>
        </div>
      </CardHeader>

      <CardContent class="space-y-5">
        <div
          class="rounded-[var(--radius)] border border-warning/40 bg-warning/10 p-3.5 flex items-start gap-3"
        >
          <ShieldAlert class="h-5 w-5 text-warning shrink-0 mt-0.5" />
          <div class="text-xs text-foreground/90 space-y-1">
            <p class="font-semibold text-warning-foreground">
              {$_("auth.recoveryModalWarningHeading")}
            </p>
            <p>
              {$_("auth.recoveryModalWarningBody")}
            </p>
          </div>
        </div>

        <div class="space-y-2">
          <span
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
          >
            {$_("setup.recoveryKeyLabel")}
          </span>
          <div
            class="flex flex-col sm:flex-row items-center justify-between gap-3 rounded-[var(--radius)] border border-primary/40 bg-primary/5 p-4"
          >
            <code
              data-testid="new-recovery-key-display"
              class="font-mono text-lg font-bold tracking-wider text-foreground select-all break-all"
            >
              {recoveryKey}
            </code>
          </div>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            onclick={handleCopy}
            class="w-full"
            data-testid="copy-recovery-key-btn"
          >
            <Copy class="h-4 w-4 mr-1.5" />
            {$_("setup.copyKeyBtn")}
          </Button>

          <Button
            type="button"
            variant="secondary"
            size="sm"
            onclick={handleDownloadTxt}
            class="w-full"
            data-testid="download-recovery-key-btn"
          >
            <Download class="h-4 w-4 mr-1.5" />
            {$_("auth.downloadTxtBtn")}
          </Button>

          <Button
            type="button"
            variant="secondary"
            size="sm"
            onclick={handlePrint}
            class="w-full"
            data-testid="print-recovery-key-btn"
          >
            <Printer class="h-4 w-4 mr-1.5" />
            {$_("auth.printBtn")}
          </Button>
        </div>

        <div
          class="space-y-2 rounded-[var(--radius)] border border-border bg-muted/40 p-4"
        >
          <label
            for="confirm-phrase-input"
            class="block text-xs font-medium text-foreground"
          >
            {$_("auth.typeConfirmationInstruction")}
            <span class="font-bold text-primary select-all"
              >"Guardei a chave"</span
            >
          </label>
          <Input
            id="confirm-phrase-input"
            data-testid="confirm-recovery-input"
            type="text"
            bind:value={typedConfirmation}
            placeholder={$_("auth.typeConfirmationPlaceholder")}
            autocomplete="off"
            class="text-sm"
          />
        </div>
      </CardContent>

      <CardFooter class="flex justify-end pt-2">
        <Button
          type="button"
          disabled={!isConfirmed}
          onclick={handleComplete}
          data-testid="finish-recovery-modal-btn"
        >
          {$_("auth.continueToSystemBtn")}
        </Button>
      </CardFooter>
    </Card>
  </div>
{/if}
