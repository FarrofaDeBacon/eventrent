<script lang="ts">
  import { untrack } from "svelte";
  import { Plus, Trash2, CheckCircle2, AlertCircle } from "lucide-svelte";
  import type {
    CreateCustomerInput,
    DocumentType,
    PersonType,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import {
    BRAZILIAN_UFS,
    maskCep,
    maskCnpj,
    maskCpf,
    maskPhone,
    normalizeDocument,
    validateCnpj,
    validateCpf,
    validateForeignDocument,
  } from "$lib/utils/document";
  import { customerFormSchema } from "$lib/utils/customer-schema";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/ui/card";
  import { Input } from "$lib/ui/input";

  interface Props {
    initialValues?: CreateCustomerInput;
    readonly?: boolean;
    submitting?: boolean;
    submitError?: string | null;
    onSubmit?: (values: CreateCustomerInput) => Promise<void> | void;
    onCancel?: () => void;
  }

  let {
    initialValues,
    readonly = false,
    submitting = false,
    submitError = null,
    onSubmit,
    onCancel,
  }: Props = $props();

  interface ContactFormItem {
    id?: string | null;
    name: string;
    email: string;
    phone: string;
    role: string;
    is_primary: boolean;
  }

  interface AddressFormItem {
    id?: string | null;
    label: string;
    street: string;
    number: string;
    complement: string;
    neighborhood: string;
    city: string;
    state: string;
    postal_code: string;
    country: string;
    is_primary: boolean;
  }

  interface CustomerFormState {
    person_type: PersonType;
    name: string;
    trade_name: string;
    document_type: DocumentType;
    document_number: string;
    state_registration: string;
    municipal_registration: string;
    notes: string;
    contacts: ContactFormItem[];
    addresses: AddressFormItem[];
  }

  function cloneInitial(src?: CreateCustomerInput): CustomerFormState {
    if (!src) {
      return {
        person_type: "legal",
        name: "",
        trade_name: "",
        document_type: "cnpj",
        document_number: "",
        state_registration: "",
        municipal_registration: "",
        notes: "",
        contacts: [
          {
            name: "",
            email: "",
            phone: "",
            role: "",
            is_primary: true,
          },
        ],
        addresses: [
          {
            label: "Principal",
            street: "",
            number: "",
            complement: "",
            neighborhood: "",
            city: "",
            state: "SP",
            postal_code: "",
            country: "BR",
            is_primary: true,
          },
        ],
      };
    }
    const docType: DocumentType =
      src.document_type ??
      (src.document_number
        ? src.person_type === "individual"
          ? "cpf"
          : "cnpj"
        : "none");
    const rawDoc = src.document_number ?? "";
    return {
      person_type: src.person_type,
      name: src.name,
      trade_name: src.trade_name ?? "",
      document_type: docType,
      document_number:
        docType === "cpf"
          ? maskCpf(rawDoc)
          : docType === "cnpj"
            ? maskCnpj(rawDoc)
            : rawDoc,
      state_registration: src.state_registration ?? "",
      municipal_registration: src.municipal_registration ?? "",
      notes: src.notes ?? "",
      contacts: src.contacts.map((c) => ({
        id: c.id ?? null,
        name: c.name,
        email: c.email ?? "",
        phone: c.phone ? maskPhone(c.phone) : "",
        role: c.role ?? "",
        is_primary: c.is_primary,
      })),
      addresses: src.addresses.map((a) => ({
        id: a.id ?? null,
        label: a.label,
        street: a.street,
        number: a.number,
        complement: a.complement ?? "",
        neighborhood: a.neighborhood,
        city: a.city,
        state: a.state,
        postal_code: maskCep(a.postal_code),
        country: a.country ?? "BR",
        is_primary: a.is_primary,
      })),
    };
  }

  let form = $state<CustomerFormState>(
    untrack(() => cloneInitial(initialValues)),
  );
  let fieldErrors = $state<Record<string, string>>({});

  let cleanDoc = $derived(normalizeDocument(form.document_number));
  let isDocComplete = $derived(
    form.document_type === "cpf"
      ? cleanDoc.length === 11
      : form.document_type === "cnpj"
        ? cleanDoc.length === 14
        : form.document_type === "foreign"
          ? cleanDoc.length >= 5
          : false,
  );
  let isDocValid = $derived(
    form.document_type === "cpf"
      ? validateCpf(cleanDoc)
      : form.document_type === "cnpj"
        ? validateCnpj(cleanDoc)
        : form.document_type === "foreign"
          ? validateForeignDocument(cleanDoc)
          : true,
  );

  function setPersonType(type: PersonType) {
    if (readonly || form.person_type === type) return;
    form.person_type = type;
    if (form.document_type === "cpf" && type === "legal") {
      form.document_type = "cnpj";
      form.document_number = maskCnpj(form.document_number);
    } else if (form.document_type === "cnpj" && type === "individual") {
      form.document_type = "cpf";
      form.document_number = maskCpf(form.document_number);
    }
    if (type === "individual") {
      form.trade_name = "";
      form.state_registration = "";
      form.municipal_registration = "";
    }
  }

  function handleDocumentTypeChange(event: Event) {
    const nextType = (event.currentTarget as HTMLSelectElement)
      .value as DocumentType;
    form.document_type = nextType;
    if (nextType === "none") {
      form.document_number = "";
    } else if (nextType === "cpf") {
      form.document_number = maskCpf(form.document_number);
    } else if (nextType === "cnpj") {
      form.document_number = maskCnpj(form.document_number);
    } else {
      form.document_number = normalizeDocument(form.document_number);
    }
  }

  function handleDocumentInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const masked =
      form.document_type === "cpf"
        ? maskCpf(input.value)
        : form.document_type === "cnpj"
          ? maskCnpj(input.value)
          : normalizeDocument(input.value).slice(0, 30);
    form.document_number = masked;
    input.value = masked;
  }

  function handlePhoneInput(index: number, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const masked = maskPhone(input.value);
    form.contacts[index].phone = masked;
    input.value = masked;
  }

  function handleCepInput(index: number, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const masked = maskCep(input.value);
    form.addresses[index].postal_code = masked;
    input.value = masked;
  }

  function addContact() {
    const hasPrimary = form.contacts.some((c) => c.is_primary);
    const nextContact: ContactFormItem = {
      name: "",
      email: "",
      phone: "",
      role: "",
      is_primary: !hasPrimary,
    };
    form.contacts = [...form.contacts, nextContact];
  }

  function removeContact(index: number) {
    const removedWasPrimary = form.contacts[index]?.is_primary;
    const next = form.contacts.filter((_, i) => i !== index);
    if (
      removedWasPrimary &&
      next.length > 0 &&
      !next.some((c) => c.is_primary)
    ) {
      next[0].is_primary = true;
    }
    form.contacts = next;
  }

  function setPrimaryContact(index: number) {
    form.contacts = form.contacts.map((c, i) => ({
      ...c,
      is_primary: i === index,
    }));
  }

  function addAddress() {
    const hasPrimary = form.addresses.some((a) => a.is_primary);
    const nextAddress: AddressFormItem = {
      label: hasPrimary ? "Entrega" : "Principal",
      street: "",
      number: "",
      complement: "",
      neighborhood: "",
      city: "",
      state: "SP",
      postal_code: "",
      country: "BR",
      is_primary: !hasPrimary,
    };
    form.addresses = [...form.addresses, nextAddress];
  }

  function removeAddress(index: number) {
    const removedWasPrimary = form.addresses[index]?.is_primary;
    const next = form.addresses.filter((_, i) => i !== index);
    if (
      removedWasPrimary &&
      next.length > 0 &&
      !next.some((a) => a.is_primary)
    ) {
      next[0].is_primary = true;
    }
    form.addresses = next;
  }

  function setPrimaryAddress(index: number) {
    form.addresses = form.addresses.map((a, i) => ({
      ...a,
      is_primary: i === index,
    }));
  }

  async function handleFormSubmit(event: SubmitEvent) {
    event.preventDefault();
    if (readonly) return;

    const parsed = customerFormSchema.safeParse(form);
    if (!parsed.success) {
      const nextErrors: Record<string, string> = {};
      for (const issue of parsed.error.issues) {
        const key = issue.path.join(".");
        if (!nextErrors[key]) {
          nextErrors[key] = issue.message;
        }
      }
      fieldErrors = nextErrors;
      return;
    }

    fieldErrors = {};
    const payload: CreateCustomerInput = {
      person_type: parsed.data.person_type,
      name: parsed.data.name,
      trade_name:
        parsed.data.person_type === "legal" && parsed.data.trade_name
          ? parsed.data.trade_name
          : null,
      document_type: parsed.data.document_type,
      document_number:
        parsed.data.document_type === "none"
          ? null
          : normalizeDocument(parsed.data.document_number ?? ""),
      state_registration:
        parsed.data.person_type === "legal" && parsed.data.state_registration
          ? parsed.data.state_registration
          : null,
      municipal_registration:
        parsed.data.person_type === "legal" &&
        parsed.data.municipal_registration
          ? parsed.data.municipal_registration
          : null,
      notes: parsed.data.notes || null,
      contacts: parsed.data.contacts.map((c) => ({
        id: c.id ?? null,
        name: c.name,
        email: c.email || null,
        phone: c.phone || null,
        role: c.role || null,
        is_primary: c.is_primary,
      })),
      addresses: parsed.data.addresses.map((a) => ({
        id: a.id ?? null,
        label: a.label,
        street: a.street,
        number: a.number,
        complement: a.complement || null,
        neighborhood: a.neighborhood,
        city: a.city,
        state: a.state,
        postal_code: a.postal_code,
        country: a.country || "BR",
        is_primary: a.is_primary,
      })),
    };

    await onSubmit?.(payload);
  }
</script>

<form onsubmit={handleFormSubmit} class="space-y-6" novalidate>
  {#if submitError}
    <div
      role="alert"
      class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm font-medium text-destructive"
    >
      {submitError}
    </div>
  {/if}

  <!-- Seção 1: Dados Básicos -->
  <Card>
    <CardHeader>
      <CardTitle>{$_("customers.basicSectionTitle")}</CardTitle>
      <CardDescription>{$_("customers.basicSectionDesc")}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="space-y-2">
        <span class="text-sm font-medium text-foreground block">
          {$_("customers.personTypeLabel")}
        </span>
        <div
          class="inline-flex rounded-[var(--radius)] border border-border p-1 bg-muted/40"
        >
          <button
            type="button"
            disabled={readonly}
            onclick={() => setPersonType("legal")}
            data-testid="person-type-legal"
            class="px-3 py-1.5 text-sm font-medium rounded-sm transition-colors cursor-pointer {form.person_type ===
            'legal'
              ? 'bg-primary text-primary-foreground shadow-xs'
              : 'text-muted-foreground hover:text-foreground'}"
          >
            {$_("customers.personTypeLegalFull")}
          </button>
          <button
            type="button"
            disabled={readonly}
            onclick={() => setPersonType("individual")}
            data-testid="person-type-individual"
            class="px-3 py-1.5 text-sm font-medium rounded-sm transition-colors cursor-pointer {form.person_type ===
            'individual'
              ? 'bg-primary text-primary-foreground shadow-xs'
              : 'text-muted-foreground hover:text-foreground'}"
          >
            {$_("customers.personTypeIndividualFull")}
          </button>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="space-y-2">
          <label
            for="customer-name"
            class="text-sm font-medium text-foreground"
          >
            {form.person_type === "legal"
              ? $_("customers.nameLabelLegal")
              : $_("customers.nameLabelIndividual")}
          </label>
          <Input
            id="customer-name"
            bind:value={form.name}
            disabled={readonly}
            placeholder={form.person_type === "legal"
              ? $_("customers.namePlaceholderLegal")
              : $_("customers.namePlaceholderIndividual")}
            required
          />
          {#if fieldErrors["name"]}
            <p class="text-xs text-destructive">{fieldErrors["name"]}</p>
          {/if}
        </div>

        <div class="space-y-2">
          <div class="flex items-center justify-between gap-2">
            <label
              for="customer-document"
              class="text-sm font-medium text-foreground"
            >
              {form.document_type === "cpf"
                ? $_("customers.documentLabelCpf")
                : form.document_type === "cnpj"
                  ? $_("customers.documentLabelCnpj")
                  : form.document_type === "foreign"
                    ? "Documento Estrangeiro *"
                    : "Documento (Opcional / Pré-cadastro)"}
            </label>
            {#if isDocComplete}
              {#if isDocValid}
                <span
                  class="inline-flex items-center gap-1 text-xs text-success"
                >
                  <CheckCircle2 class="h-3.5 w-3.5" />
                  {$_("customers.documentValid")}
                </span>
              {:else}
                <span
                  class="inline-flex items-center gap-1 text-xs text-destructive"
                >
                  <AlertCircle class="h-3.5 w-3.5" />
                  {$_("customers.documentInvalid")}
                </span>
              {/if}
            {/if}
          </div>
          <div class="flex gap-2">
            <select
              aria-label="Tipo de documento"
              value={form.document_type}
              onchange={handleDocumentTypeChange}
              disabled={readonly}
              class="h-10 rounded-[var(--radius)] border border-input bg-background px-2.5 text-xs text-foreground"
            >
              {#if form.person_type === "individual"}
                <option value="cpf">CPF</option>
              {:else}
                <option value="cnpj">CNPJ</option>
              {/if}
              <option value="foreign">Estrangeiro</option>
              <option value="none">Sem documento</option>
            </select>
            <Input
              id="customer-document"
              value={form.document_number}
              oninput={handleDocumentInput}
              disabled={readonly || form.document_type === "none"}
              placeholder={form.document_type === "cnpj"
                ? $_("customers.documentPlaceholderCnpj")
                : form.document_type === "cpf"
                  ? $_("customers.documentPlaceholderCpf")
                  : form.document_type === "foreign"
                    ? "Ex.: PASS123456"
                    : "Pré-cadastro sem documento"}
            />
          </div>
          {#if fieldErrors["document_number"]}
            <p class="text-xs text-destructive">
              {fieldErrors["document_number"]}
            </p>
          {/if}
        </div>

        {#if form.person_type === "legal"}
          <div class="space-y-2">
            <label
              for="customer-trade-name"
              class="text-sm font-medium text-foreground"
            >
              {$_("customers.tradeNameLabel")}
            </label>
            <Input
              id="customer-trade-name"
              bind:value={form.trade_name}
              disabled={readonly}
              placeholder={$_("customers.tradeNamePlaceholder")}
            />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div class="space-y-2">
              <label
                for="customer-ie"
                class="text-sm font-medium text-foreground"
              >
                {$_("customers.stateRegistrationLabel")}
              </label>
              <Input
                id="customer-ie"
                bind:value={form.state_registration}
                disabled={readonly}
              />
            </div>
            <div class="space-y-2">
              <label
                for="customer-im"
                class="text-sm font-medium text-foreground"
              >
                {$_("customers.municipalRegistrationLabel")}
              </label>
              <Input
                id="customer-im"
                bind:value={form.municipal_registration}
                disabled={readonly}
              />
            </div>
          </div>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="customer-notes" class="text-sm font-medium text-foreground">
          {$_("customers.notesLabel")}
        </label>
        <textarea
          id="customer-notes"
          rows={3}
          bind:value={form.notes}
          disabled={readonly}
          placeholder={$_("customers.notesPlaceholder")}
          class="flex w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        ></textarea>
      </div>
    </CardContent>
  </Card>

  <!-- Seção 2: Contatos -->
  <Card>
    <CardHeader class="flex flex-row items-center justify-between gap-4">
      <div>
        <CardTitle>{$_("customers.contactsSectionTitle")}</CardTitle>
        <CardDescription>{$_("customers.contactsSectionDesc")}</CardDescription>
      </div>
      {#if !readonly}
        <Button
          type="button"
          variant="outline"
          size="sm"
          onclick={addContact}
          data-testid="add-contact-btn"
        >
          <Plus class="h-4 w-4" />
          {$_("customers.addContactBtn")}
        </Button>
      {/if}
    </CardHeader>
    <CardContent class="space-y-4">
      {#if fieldErrors["contacts"]}
        <p class="text-xs text-destructive">{fieldErrors["contacts"]}</p>
      {/if}

      {#if form.contacts.length === 0}
        <p class="text-sm text-muted-foreground">
          {$_("customers.noContacts")}
        </p>
      {:else}
        {#each form.contacts as contact, idx (idx)}
          <div
            class="rounded-[var(--radius)] border border-border p-4 space-y-3 bg-muted/20"
            data-testid={`contact-row-${idx}`}
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <span
                  class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                >
                  Contato #{idx + 1}
                </span>
                {#if contact.is_primary}
                  <Badge variant="default">{$_("customers.primaryLabel")}</Badge
                  >
                {/if}
              </div>

              {#if !readonly}
                <div class="flex items-center gap-2">
                  {#if !contact.is_primary}
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onclick={() => setPrimaryContact(idx)}
                    >
                      {$_("customers.primaryLabel")}
                    </Button>
                  {/if}
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onclick={() => removeContact(idx)}
                    title={$_("customers.removeBtn")}
                  >
                    <Trash2 class="h-4 w-4 text-destructive" />
                  </Button>
                </div>
              {/if}
            </div>

            <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
              <div class="space-y-1">
                <label
                  for={`contact-name-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.contactNameLabel")}
                </label>
                <Input
                  id={`contact-name-${idx}`}
                  bind:value={contact.name}
                  disabled={readonly}
                  required
                />
                {#if fieldErrors[`contacts.${idx}.name`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`contacts.${idx}.name`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1">
                <label
                  for={`contact-email-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.contactEmailLabel")}
                </label>
                <Input
                  id={`contact-email-${idx}`}
                  type="email"
                  bind:value={contact.email}
                  disabled={readonly}
                />
                {#if fieldErrors[`contacts.${idx}.email`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`contacts.${idx}.email`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1">
                <label
                  for={`contact-phone-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.contactPhoneLabel")}
                </label>
                <Input
                  id={`contact-phone-${idx}`}
                  value={contact.phone}
                  oninput={(e) => handlePhoneInput(idx, e)}
                  disabled={readonly}
                  placeholder="(11) 99999-9999"
                />
                {#if fieldErrors[`contacts.${idx}.phone`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`contacts.${idx}.phone`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1">
                <label
                  for={`contact-role-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.contactRoleLabel")}
                </label>
                <Input
                  id={`contact-role-${idx}`}
                  bind:value={contact.role}
                  disabled={readonly}
                />
              </div>
            </div>
          </div>
        {/each}
      {/if}
    </CardContent>
  </Card>

  <!-- Seção 3: Endereços -->
  <Card>
    <CardHeader class="flex flex-row items-center justify-between gap-4">
      <div>
        <CardTitle>{$_("customers.addressesSectionTitle")}</CardTitle>
        <CardDescription>{$_("customers.addressesSectionDesc")}</CardDescription
        >
      </div>
      {#if !readonly}
        <Button
          type="button"
          variant="outline"
          size="sm"
          onclick={addAddress}
          data-testid="add-address-btn"
        >
          <Plus class="h-4 w-4" />
          {$_("customers.addAddressBtn")}
        </Button>
      {/if}
    </CardHeader>
    <CardContent class="space-y-4">
      {#if fieldErrors["addresses"]}
        <p class="text-xs text-destructive">{fieldErrors["addresses"]}</p>
      {/if}

      {#if form.addresses.length === 0}
        <p class="text-sm text-muted-foreground">
          {$_("customers.noAddresses")}
        </p>
      {:else}
        {#each form.addresses as address, idx (idx)}
          <div
            class="rounded-[var(--radius)] border border-border p-4 space-y-3 bg-muted/20"
            data-testid={`address-row-${idx}`}
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <span
                  class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                >
                  Endereço #{idx + 1}
                </span>
                {#if address.is_primary}
                  <Badge variant="default">{$_("customers.primaryLabel")}</Badge
                  >
                {/if}
              </div>

              {#if !readonly}
                <div class="flex items-center gap-2">
                  {#if !address.is_primary}
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onclick={() => setPrimaryAddress(idx)}
                    >
                      {$_("customers.primaryLabel")}
                    </Button>
                  {/if}
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onclick={() => removeAddress(idx)}
                    title={$_("customers.removeBtn")}
                  >
                    <Trash2 class="h-4 w-4 text-destructive" />
                  </Button>
                </div>
              {/if}
            </div>

            <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
              <div class="space-y-1">
                <label
                  for={`address-label-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.addressLabelField")}
                </label>
                <Input
                  id={`address-label-${idx}`}
                  bind:value={address.label}
                  disabled={readonly}
                  placeholder={$_("customers.addressLabelPlaceholder")}
                  required
                />
                {#if fieldErrors[`addresses.${idx}.label`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`addresses.${idx}.label`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1">
                <label
                  for={`address-cep-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.addressCepLabel")}
                </label>
                <Input
                  id={`address-cep-${idx}`}
                  value={address.postal_code}
                  oninput={(e) => handleCepInput(idx, e)}
                  disabled={readonly}
                  placeholder="00000-000"
                  required
                />
                {#if fieldErrors[`addresses.${idx}.postal_code`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`addresses.${idx}.postal_code`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1 md:col-span-2">
                <label
                  for={`address-street-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.addressStreetLabel")}
                </label>
                <Input
                  id={`address-street-${idx}`}
                  bind:value={address.street}
                  disabled={readonly}
                  required
                />
                {#if fieldErrors[`addresses.${idx}.street`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`addresses.${idx}.street`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1">
                <label
                  for={`address-number-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.addressNumberLabel")}
                </label>
                <Input
                  id={`address-number-${idx}`}
                  bind:value={address.number}
                  disabled={readonly}
                  required
                />
                {#if fieldErrors[`addresses.${idx}.number`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`addresses.${idx}.number`]}
                  </p>
                {/if}
              </div>

              <div class="space-y-1">
                <label
                  for={`address-complement-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.addressComplementLabel")}
                </label>
                <Input
                  id={`address-complement-${idx}`}
                  bind:value={address.complement}
                  disabled={readonly}
                />
              </div>

              <div class="space-y-1">
                <label
                  for={`address-neighborhood-${idx}`}
                  class="text-xs font-medium text-foreground"
                >
                  {$_("customers.addressNeighborhoodLabel")}
                </label>
                <Input
                  id={`address-neighborhood-${idx}`}
                  bind:value={address.neighborhood}
                  disabled={readonly}
                  required
                />
                {#if fieldErrors[`addresses.${idx}.neighborhood`]}
                  <p class="text-xs text-destructive">
                    {fieldErrors[`addresses.${idx}.neighborhood`]}
                  </p>
                {/if}
              </div>

              <div class="grid grid-cols-3 gap-2">
                <div class="space-y-1 col-span-2">
                  <label
                    for={`address-city-${idx}`}
                    class="text-xs font-medium text-foreground"
                  >
                    {$_("customers.addressCityLabel")}
                  </label>
                  <Input
                    id={`address-city-${idx}`}
                    bind:value={address.city}
                    disabled={readonly}
                    required
                  />
                  {#if fieldErrors[`addresses.${idx}.city`]}
                    <p class="text-xs text-destructive">
                      {fieldErrors[`addresses.${idx}.city`]}
                    </p>
                  {/if}
                </div>

                <div class="space-y-1">
                  <label
                    for={`address-state-${idx}`}
                    class="text-xs font-medium text-foreground"
                  >
                    {$_("customers.addressStateLabel")}
                  </label>
                  <select
                    id={`address-state-${idx}`}
                    bind:value={address.state}
                    disabled={readonly}
                    class="flex h-10 w-full rounded-[var(--radius)] border border-input bg-background px-2 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
                  >
                    {#each BRAZILIAN_UFS as uf (uf)}
                      <option value={uf}>{uf}</option>
                    {/each}
                  </select>
                </div>
              </div>
            </div>
          </div>
        {/each}
      {/if}
    </CardContent>
  </Card>

  {#if !readonly}
    <div class="flex items-center justify-end gap-3 pt-2">
      {#if onCancel}
        <Button type="button" variant="ghost" onclick={onCancel}>
          {$_("customers.cancelBtn")}
        </Button>
      {/if}
      <Button
        type="submit"
        disabled={submitting}
        data-testid="save-customer-btn"
      >
        {submitting ? $_("customers.saving") : $_("customers.saveBtn")}
      </Button>
    </div>
  {/if}
</form>
