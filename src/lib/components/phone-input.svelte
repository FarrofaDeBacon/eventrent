<script lang="ts">
  import { untrack } from "svelte";
  import {
    composePhoneE164,
    digitsOnly,
    maskPhone,
    PHONE_DDI_OPTIONS,
    splitPhoneDdi,
  } from "$lib/utils/document";
  import { Input } from "$lib/ui/input";

  interface Props {
    id: string;
    value?: string;
    disabled?: boolean;
    placeholder?: string;
  }

  let {
    id,
    value = $bindable(""),
    disabled = false,
    placeholder = "(11) 99999-9999",
  }: Props = $props();

  let initial = untrack(() => splitPhoneDdi(value));
  let ddi = $state(initial.ddi);
  let localText = $state(initial.local);

  $effect(() => {
    const incoming = value ?? "";
    const currentComposed = composePhoneE164(ddi, localText);
    if (incoming !== currentComposed) {
      const parsed = splitPhoneDdi(incoming);
      ddi = parsed.ddi;
      localText = parsed.local;
    }
  });

  function handleDdiChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    ddi = select.value;
    const digits = digitsOnly(localText);
    localText = ddi === "+55" ? maskPhone(digits) : digits;
    value = composePhoneE164(ddi, localText);
  }

  function handleInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const raw = input.value;
    if (raw.trim().startsWith("+")) {
      const parsed = splitPhoneDdi(raw);
      ddi = parsed.ddi;
      localText = parsed.local;
      input.value = parsed.local;
      value = composePhoneE164(ddi, parsed.local);
      return;
    }
    const nextLocal =
      ddi === "+55" ? maskPhone(raw) : digitsOnly(raw).slice(0, 15);
    localText = nextLocal;
    input.value = nextLocal;
    value = composePhoneE164(ddi, nextLocal);
  }
</script>

<div class="flex gap-2">
  <select
    aria-label="DDI do telefone"
    data-testid={`${id}-ddi`}
    value={ddi}
    onchange={handleDdiChange}
    {disabled}
    class="h-10 rounded-[var(--radius)] border border-input bg-background px-2 text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
  >
    {#each PHONE_DDI_OPTIONS as option (option.code)}
      <option value={option.code}>{option.label}</option>
    {/each}
  </select>
  <Input
    {id}
    value={localText}
    oninput={handleInput}
    {disabled}
    placeholder={ddi === "+55" ? placeholder : "Número internacional"}
    class="flex-1"
  />
</div>
