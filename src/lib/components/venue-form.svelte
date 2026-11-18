<script lang="ts">
  import { untrack } from "svelte";
  import {
    MapPin,
    Layers,
    Users,
    Plus,
    Trash2,
    AlertTriangle,
    ChevronDown,
    ChevronUp,
    Save,
    Star,
  } from "lucide-svelte";
  import {
    getVenueRepository,
    type CreateVenueInput,
    type VenueSpaceInput,
    type VenueContactInput,
    type VenueWithDetails,
  } from "$lib/data";
  import PhoneInput from "$lib/components/phone-input.svelte";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";
  import { Badge } from "$lib/ui/badge";
  import { Card, CardContent } from "$lib/ui/card";

  interface Props {
    initialVenue?: VenueWithDetails | null;
    readOnly?: boolean;
    isSaving?: boolean;
    onSubmit: (payload: CreateVenueInput) => Promise<void>;
    onCancel: () => void;
  }

  let {
    initialVenue = null,
    readOnly = false,
    isSaving = false,
    onSubmit,
    onCancel,
  }: Props = $props();

  type TabId = "dados" | "espacos" | "contatos";
  let activeTab = $state<TabId>("dados");

  const seed = untrack(() => initialVenue);

  let name = $state(seed?.name ?? "");
  let street = $state(seed?.street ?? "");
  let number = $state(seed?.number ?? "");
  let complement = $state(seed?.complement ?? "");
  let neighborhood = $state(seed?.neighborhood ?? "");
  let city = $state(seed?.city ?? "");
  let stateUf = $state(seed?.state ?? "");
  let postalCode = $state(seed?.postal_code ?? "");
  let country = $state(seed?.country ?? "BR");
  let accessHours = $state(seed?.access_hours ?? "");
  let parkingNotes = $state(seed?.parking_notes ?? "");
  let loadingDockNotes = $state(seed?.loading_dock_notes ?? "");
  let notes = $state(seed?.notes ?? "");

  interface EditableSpace {
    localKey: string;
    id: string | null;
    name: string;
    capacity_people: string;
    area_sqm: string;
    ceiling_height_m: string;
    internal_transfer_minutes: string;
    power_available: string;
    loading_access: string;
    rigging_points: string;
    acoustics_notes: string;
    notes: string;
    active: boolean;
    techExpanded: boolean;
  }

  interface EditableContact {
    localKey: string;
    id: string | null;
    name: string;
    role: string;
    phone: string;
    email: string;
    is_primary: boolean;
  }

  let nextKey = 1;
  function makeKey(prefix: string): string {
    return `${prefix}-${nextKey++}`;
  }

  let spaces = $state<EditableSpace[]>(
    (seed?.spaces ?? []).map((sp) => ({
      localKey: makeKey("sp"),
      id: sp.id,
      name: sp.name,
      capacity_people:
        sp.capacity_people !== null && sp.capacity_people !== undefined
          ? String(sp.capacity_people)
          : "",
      area_sqm:
        sp.area_sqm !== null && sp.area_sqm !== undefined
          ? String(sp.area_sqm)
          : "",
      ceiling_height_m:
        sp.ceiling_height_m !== null && sp.ceiling_height_m !== undefined
          ? String(sp.ceiling_height_m)
          : "",
      internal_transfer_minutes: String(sp.internal_transfer_minutes ?? 0),
      power_available: sp.power_available ?? "",
      loading_access: sp.loading_access ?? "",
      rigging_points: sp.rigging_points ?? "",
      acoustics_notes: sp.acoustics_notes ?? "",
      notes: sp.notes ?? "",
      active: sp.active,
      techExpanded: false,
    })),
  );

  let contacts = $state<EditableContact[]>(
    (seed?.contacts ?? []).map((ct) => ({
      localKey: makeKey("ct"),
      id: ct.id,
      name: ct.name,
      role: ct.role ?? "",
      phone: ct.phone ?? "",
      email: ct.email ?? "",
      is_primary: ct.is_primary,
    })),
  );

  let duplicateWarning = $state(false);
  let formError = $state<string | null>(null);

  async function checkDuplicate() {
    if (readOnly) return;
    const trimmed = name.trim();
    if (!trimmed) {
      duplicateWarning = false;
      return;
    }
    try {
      duplicateWarning = await getVenueRepository().checkDuplicateNameCity(
        trimmed,
        city.trim() || null,
        initialVenue?.id ?? null,
      );
    } catch {
      duplicateWarning = false;
    }
  }

  function addSpace() {
    spaces = [
      ...spaces,
      {
        localKey: makeKey("sp"),
        id: null,
        name: "",
        capacity_people: "",
        area_sqm: "",
        ceiling_height_m: "",
        internal_transfer_minutes: "0",
        power_available: "",
        loading_access: "",
        rigging_points: "",
        acoustics_notes: "",
        notes: "",
        active: true,
        techExpanded: false,
      },
    ];
  }

  function removeSpace(localKey: string) {
    spaces = spaces.filter((s) => s.localKey !== localKey);
  }

  function toggleTech(localKey: string) {
    spaces = spaces.map((s) =>
      s.localKey === localKey ? { ...s, techExpanded: !s.techExpanded } : s,
    );
  }

  function addContact() {
    const isFirst = contacts.length === 0;
    contacts = [
      ...contacts,
      {
        localKey: makeKey("ct"),
        id: null,
        name: "",
        role: "",
        phone: "",
        email: "",
        is_primary: isFirst,
      },
    ];
  }

  function removeContact(localKey: string) {
    const remaining = contacts.filter((c) => c.localKey !== localKey);
    if (remaining.length > 0 && !remaining.some((c) => c.is_primary)) {
      remaining[0] = { ...remaining[0], is_primary: true };
    }
    contacts = remaining;
  }

  function setPrimaryContact(localKey: string) {
    contacts = contacts.map((c) => ({
      ...c,
      is_primary: c.localKey === localKey,
    }));
  }

  function parseOptionalInt(raw: string): number | null {
    const t = raw.trim();
    if (!t) return null;
    const n = Number.parseInt(t, 10);
    return Number.isNaN(n) ? null : n;
  }

  function parseOptionalFloat(raw: string): number | null {
    const t = raw.trim().replace(",", ".");
    if (!t) return null;
    const n = Number.parseFloat(t);
    return Number.isNaN(n) ? null : n;
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (readOnly || isSaving) return;

    formError = null;
    const trimmedName = name.trim();
    if (!trimmedName) {
      activeTab = "dados";
      formError = "O nome do local é obrigatório.";
      return;
    }

    const spaceInputs: VenueSpaceInput[] = [];
    for (let i = 0; i < spaces.length; i++) {
      const sp = spaces[i];
      if (!sp.name.trim()) {
        activeTab = "espacos";
        formError = `Informe o nome do espaço #${i + 1}.`;
        return;
      }
      const transfer = parseOptionalInt(sp.internal_transfer_minutes) ?? 0;
      if (transfer < 0 || transfer > 240) {
        activeTab = "espacos";
        formError = `O deslocamento interno de "${sp.name}" deve estar entre 0 e 240 minutos.`;
        return;
      }
      spaceInputs.push({
        id: sp.id,
        name: sp.name.trim(),
        capacity_people: parseOptionalInt(sp.capacity_people),
        area_sqm: parseOptionalFloat(sp.area_sqm),
        ceiling_height_m: parseOptionalFloat(sp.ceiling_height_m),
        power_available: sp.power_available.trim() || null,
        loading_access: sp.loading_access.trim() || null,
        rigging_points: sp.rigging_points.trim() || null,
        acoustics_notes: sp.acoustics_notes.trim() || null,
        internal_transfer_minutes: transfer,
        notes: sp.notes.trim() || null,
        active: sp.active,
        sort_order: i,
      });
    }

    const contactInputs: VenueContactInput[] = [];
    for (let i = 0; i < contacts.length; i++) {
      const ct = contacts[i];
      if (!ct.name.trim()) {
        activeTab = "contatos";
        formError = `Informe o nome do contato #${i + 1}.`;
        return;
      }
      if (!ct.phone.trim() && !ct.email.trim()) {
        activeTab = "contatos";
        formError = `O contato "${ct.name}" precisa de pelo menos um telefone ou e-mail.`;
        return;
      }
      contactInputs.push({
        id: ct.id,
        name: ct.name.trim(),
        role: ct.role.trim() || null,
        phone: ct.phone.trim() || null,
        email: ct.email.trim() || null,
        is_primary: ct.is_primary,
      });
    }

    await onSubmit({
      name: trimmedName,
      street: street.trim() || null,
      number: number.trim() || null,
      complement: complement.trim() || null,
      neighborhood: neighborhood.trim() || null,
      city: city.trim() || null,
      state: stateUf.trim() || null,
      postal_code: postalCode.trim() || null,
      country: country.trim() || "BR",
      access_hours: accessHours.trim() || null,
      parking_notes: parkingNotes.trim() || null,
      loading_dock_notes: loadingDockNotes.trim() || null,
      notes: notes.trim() || null,
      spaces: spaceInputs,
      contacts: contactInputs,
    });
  }
</script>

<form onsubmit={handleSubmit} class="space-y-6">
  {#if formError}
    <div
      role="alert"
      class="flex items-center gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
    >
      <AlertTriangle class="size-4 shrink-0" />
      <span>{formError}</span>
    </div>
  {/if}

  {#if duplicateWarning}
    <div
      role="status"
      class="flex items-center gap-3 rounded-lg border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-foreground"
    >
      <AlertTriangle class="size-5 shrink-0 text-warning" />
      <span>
        Já existe um local com este nome e cidade cadastrado. Verifique se não é
        duplicidade — caso seja outra unidade, você pode salvar normalmente.
      </span>
    </div>
  {/if}

  <!-- Abas -->
  <div
    role="tablist"
    aria-label="Seções do cadastro de local"
    class="flex flex-wrap gap-2 border-b border-border pb-2"
  >
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "dados"}
      onclick={() => (activeTab = "dados")}
      class={`flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
        activeTab === "dados"
          ? "bg-primary text-primary-foreground"
          : "text-muted-foreground hover:bg-muted hover:text-foreground"
      }`}
    >
      <MapPin class="size-4" />
      Dados do Local
    </button>

    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "espacos"}
      onclick={() => (activeTab = "espacos")}
      class={`flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
        activeTab === "espacos"
          ? "bg-primary text-primary-foreground"
          : "text-muted-foreground hover:bg-muted hover:text-foreground"
      }`}
    >
      <Layers class="size-4" />
      Espaços
      <Badge variant="secondary" class="ml-1">{spaces.length}</Badge>
    </button>

    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "contatos"}
      onclick={() => (activeTab = "contatos")}
      class={`flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
        activeTab === "contatos"
          ? "bg-primary text-primary-foreground"
          : "text-muted-foreground hover:bg-muted hover:text-foreground"
      }`}
    >
      <Users class="size-4" />
      Contatos
      <Badge variant="secondary" class="ml-1">{contacts.length}</Badge>
    </button>
  </div>

  {#if activeTab === "dados"}
    <Card>
      <CardContent class="space-y-5 pt-6">
        <div class="grid gap-4 sm:grid-cols-2">
          <div class="space-y-1.5 sm:col-span-2">
            <label for="venue-name" class="text-sm font-medium">
              Nome do local <span class="text-destructive">*</span>
            </label>
            <Input
              id="venue-name"
              placeholder="Ex.: Riocentro Centro de Convenções"
              disabled={readOnly}
              bind:value={name}
              onblur={checkDuplicate}
            />
          </div>

          <div class="space-y-1.5">
            <label for="venue-country" class="text-sm font-medium">País</label>
            <select
              id="venue-country"
              disabled={readOnly}
              bind:value={country}
              class="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
            >
              <option value="BR">Brasil (BR)</option>
              <option value="US">Estados Unidos (US)</option>
              <option value="AR">Argentina (AR)</option>
              <option value="PT">Portugal (PT)</option>
              <option value="UY">Uruguai (UY)</option>
              <option value="CL">Chile (CL)</option>
            </select>
          </div>

          <div class="space-y-1.5">
            <label for="venue-postal" class="text-sm font-medium">CEP</label>
            <Input
              id="venue-postal"
              placeholder="22783-115"
              disabled={readOnly}
              bind:value={postalCode}
            />
          </div>

          <div class="space-y-1.5 sm:col-span-2">
            <label for="venue-street" class="text-sm font-medium"
              >Logradouro</label
            >
            <Input
              id="venue-street"
              placeholder="Av. Salvador Allende"
              disabled={readOnly}
              bind:value={street}
            />
          </div>

          <div class="space-y-1.5">
            <label for="venue-number" class="text-sm font-medium">Número</label>
            <Input
              id="venue-number"
              placeholder="6555"
              disabled={readOnly}
              bind:value={number}
            />
          </div>

          <div class="space-y-1.5">
            <label for="venue-complement" class="text-sm font-medium"
              >Complemento</label
            >
            <Input
              id="venue-complement"
              disabled={readOnly}
              bind:value={complement}
            />
          </div>

          <div class="space-y-1.5">
            <label for="venue-neighborhood" class="text-sm font-medium"
              >Bairro</label
            >
            <Input
              id="venue-neighborhood"
              placeholder="Barra da Tijuca"
              disabled={readOnly}
              bind:value={neighborhood}
            />
          </div>

          <div class="grid grid-cols-3 gap-3">
            <div class="col-span-2 space-y-1.5">
              <label for="venue-city" class="text-sm font-medium">Cidade</label>
              <Input
                id="venue-city"
                placeholder="Rio de Janeiro"
                disabled={readOnly}
                bind:value={city}
                onblur={checkDuplicate}
              />
            </div>
            <div class="space-y-1.5">
              <label for="venue-state" class="text-sm font-medium">UF</label>
              <Input
                id="venue-state"
                placeholder="RJ"
                maxlength={2}
                disabled={readOnly}
                bind:value={stateUf}
              />
            </div>
          </div>

          <div class="space-y-1.5 sm:col-span-2">
            <label for="venue-access-hours" class="text-sm font-medium">
              Horários de acesso / montagem
            </label>
            <Input
              id="venue-access-hours"
              placeholder="Ex.: 07:00 às 22:00 (docas mediante agendamento)"
              disabled={readOnly}
              bind:value={accessHours}
            />
          </div>

          <div class="space-y-1.5">
            <label for="venue-parking" class="text-sm font-medium">
              Estacionamento e manobra
            </label>
            <textarea
              id="venue-parking"
              rows={3}
              disabled={readOnly}
              bind:value={parkingNotes}
              class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
            ></textarea>
          </div>

          <div class="space-y-1.5">
            <label for="venue-dock" class="text-sm font-medium">
              Docas / Carga e descarga
            </label>
            <textarea
              id="venue-dock"
              rows={3}
              disabled={readOnly}
              bind:value={loadingDockNotes}
              class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
            ></textarea>
          </div>

          <div class="space-y-1.5 sm:col-span-2">
            <label for="venue-notes" class="text-sm font-medium">
              Observações gerais
            </label>
            <textarea
              id="venue-notes"
              rows={3}
              disabled={readOnly}
              bind:value={notes}
              class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
            ></textarea>
          </div>
        </div>
      </CardContent>
    </Card>
  {:else if activeTab === "espacos"}
    <div class="space-y-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <p class="text-sm text-muted-foreground">
          Cadastre pavilhões, auditórios, salas ou áreas internas deste local.
        </p>
        {#if !readOnly}
          <Button type="button" variant="outline" onclick={addSpace}>
            <Plus class="mr-2 size-4" />
            Adicionar espaço
          </Button>
        {/if}
      </div>

      {#if spaces.length === 0}
        <Card>
          <CardContent class="py-8 text-center text-sm text-muted-foreground">
            Nenhum espaço cadastrado ainda.
          </CardContent>
        </Card>
      {:else}
        {#each spaces as sp, idx (sp.localKey)}
          <Card>
            <CardContent class="space-y-4 pt-5">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-semibold">
                    Espaço #{idx + 1}
                  </span>
                  {#if sp.id}
                    <Badge variant="outline" class="font-mono text-xs">
                      ID estável
                    </Badge>
                  {/if}
                </div>
                <div class="flex items-center gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onclick={() => toggleTech(sp.localKey)}
                  >
                    {#if sp.techExpanded}
                      <ChevronUp class="mr-1 size-4" />
                      Ocultar ficha técnica
                    {:else}
                      <ChevronDown class="mr-1 size-4" />
                      Ficha técnica
                    {/if}
                  </Button>
                  {#if !readOnly}
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onclick={() => removeSpace(sp.localKey)}
                      aria-label={`Remover espaço ${sp.name || idx + 1}`}
                      class="text-destructive hover:text-destructive"
                    >
                      <Trash2 class="size-4" />
                    </Button>
                  {/if}
                </div>
              </div>

              <div class="grid gap-4 sm:grid-cols-5">
                <div class="space-y-1.5 sm:col-span-2">
                  <label
                    for={`sp-name-${sp.localKey}`}
                    class="text-sm font-medium"
                  >
                    Nome do espaço <span class="text-destructive">*</span>
                  </label>
                  <Input
                    id={`sp-name-${sp.localKey}`}
                    placeholder="Ex.: Pavilhão 2"
                    disabled={readOnly}
                    bind:value={sp.name}
                  />
                </div>

                <div class="space-y-1.5">
                  <label
                    for={`sp-cap-${sp.localKey}`}
                    class="text-sm font-medium"
                  >
                    Capacidade (pessoas)
                  </label>
                  <Input
                    id={`sp-cap-${sp.localKey}`}
                    type="number"
                    min={0}
                    disabled={readOnly}
                    bind:value={sp.capacity_people}
                  />
                </div>

                <div class="space-y-1.5">
                  <label
                    for={`sp-area-${sp.localKey}`}
                    class="text-sm font-medium"
                  >
                    Área (m²)
                  </label>
                  <Input
                    id={`sp-area-${sp.localKey}`}
                    type="number"
                    min={0}
                    step="0.1"
                    disabled={readOnly}
                    bind:value={sp.area_sqm}
                  />
                </div>

                <div class="space-y-1.5">
                  <label
                    for={`sp-transfer-${sp.localKey}`}
                    class="text-sm font-medium"
                  >
                    Desloc. interno (min)
                  </label>
                  <Input
                    id={`sp-transfer-${sp.localKey}`}
                    type="number"
                    min={0}
                    max={240}
                    disabled={readOnly}
                    bind:value={sp.internal_transfer_minutes}
                  />
                </div>
              </div>

              {#if sp.techExpanded}
                <div
                  class="grid gap-4 border-t border-border pt-4 sm:grid-cols-2"
                >
                  <div class="space-y-1.5">
                    <label
                      for={`sp-height-${sp.localKey}`}
                      class="text-sm font-medium"
                    >
                      Pé-direito (m)
                    </label>
                    <Input
                      id={`sp-height-${sp.localKey}`}
                      type="number"
                      min={0}
                      step="0.1"
                      disabled={readOnly}
                      bind:value={sp.ceiling_height_m}
                    />
                  </div>

                  <div class="space-y-1.5">
                    <label
                      for={`sp-power-${sp.localKey}`}
                      class="text-sm font-medium"
                    >
                      Energia disponível
                    </label>
                    <Input
                      id={`sp-power-${sp.localKey}`}
                      placeholder="Ex.: Trifásico 380V / 250 kVA"
                      disabled={readOnly}
                      bind:value={sp.power_available}
                    />
                  </div>

                  <div class="space-y-1.5">
                    <label
                      for={`sp-loading-${sp.localKey}`}
                      class="text-sm font-medium"
                    >
                      Acesso de carga
                    </label>
                    <Input
                      id={`sp-loading-${sp.localKey}`}
                      placeholder="Ex.: Rampa direta p/ caminhão truck"
                      disabled={readOnly}
                      bind:value={sp.loading_access}
                    />
                  </div>

                  <div class="space-y-1.5">
                    <label
                      for={`sp-rigging-${sp.localKey}`}
                      class="text-sm font-medium"
                    >
                      Pontos de rigging
                    </label>
                    <Input
                      id={`sp-rigging-${sp.localKey}`}
                      placeholder="Ex.: 500 kg por ponto na treliça"
                      disabled={readOnly}
                      bind:value={sp.rigging_points}
                    />
                  </div>

                  <div class="space-y-1.5 sm:col-span-2">
                    <label
                      for={`sp-acoustics-${sp.localKey}`}
                      class="text-sm font-medium"
                    >
                      Acústica e observações técnicas
                    </label>
                    <textarea
                      id={`sp-acoustics-${sp.localKey}`}
                      rows={2}
                      disabled={readOnly}
                      bind:value={sp.acoustics_notes}
                      class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
                    ></textarea>
                  </div>
                </div>
              {/if}
            </CardContent>
          </Card>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="space-y-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <p class="text-sm text-muted-foreground">
          Contatos operacionais e comerciais do local (portaria, gerência,
          docas).
        </p>
        {#if !readOnly}
          <Button type="button" variant="outline" onclick={addContact}>
            <Plus class="mr-2 size-4" />
            Adicionar contato
          </Button>
        {/if}
      </div>

      {#if contacts.length === 0}
        <Card>
          <CardContent class="py-8 text-center text-sm text-muted-foreground">
            Nenhum contato cadastrado ainda.
          </CardContent>
        </Card>
      {:else}
        {#each contacts as ct, idx (ct.localKey)}
          <Card>
            <CardContent class="space-y-4 pt-5">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-semibold">
                    Contato #{idx + 1}
                  </span>
                  {#if ct.is_primary}
                    <Badge variant="default">Principal</Badge>
                  {/if}
                </div>
                {#if !readOnly}
                  <div class="flex items-center gap-2">
                    {#if !ct.is_primary}
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onclick={() => setPrimaryContact(ct.localKey)}
                      >
                        <Star class="mr-1 size-4" />
                        Definir principal
                      </Button>
                    {/if}
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onclick={() => removeContact(ct.localKey)}
                      aria-label={`Remover contato ${ct.name || idx + 1}`}
                      class="text-destructive hover:text-destructive"
                    >
                      <Trash2 class="size-4" />
                    </Button>
                  </div>
                {/if}
              </div>

              <div class="grid gap-4 sm:grid-cols-2">
                <div class="space-y-1.5">
                  <label
                    for={`ct-name-${ct.localKey}`}
                    class="text-sm font-medium"
                  >
                    Nome <span class="text-destructive">*</span>
                  </label>
                  <Input
                    id={`ct-name-${ct.localKey}`}
                    disabled={readOnly}
                    bind:value={ct.name}
                  />
                </div>

                <div class="space-y-1.5">
                  <label
                    for={`ct-role-${ct.localKey}`}
                    class="text-sm font-medium"
                  >
                    Cargo / Função
                  </label>
                  <Input
                    id={`ct-role-${ct.localKey}`}
                    placeholder="Ex.: Coordenador de Operações"
                    disabled={readOnly}
                    bind:value={ct.role}
                  />
                </div>

                <div class="space-y-1.5">
                  <label
                    for={`ct-phone-${ct.localKey}`}
                    class="text-sm font-medium"
                  >
                    Telefone
                  </label>
                  <PhoneInput
                    id={`ct-phone-${ct.localKey}`}
                    disabled={readOnly}
                    bind:value={ct.phone}
                  />
                </div>

                <div class="space-y-1.5">
                  <label
                    for={`ct-email-${ct.localKey}`}
                    class="text-sm font-medium"
                  >
                    E-mail
                  </label>
                  <Input
                    id={`ct-email-${ct.localKey}`}
                    type="email"
                    disabled={readOnly}
                    bind:value={ct.email}
                  />
                </div>
              </div>
            </CardContent>
          </Card>
        {/each}
      {/if}
    </div>
  {/if}

  <div class="flex items-center justify-end gap-3 border-t border-border pt-4">
    <Button type="button" variant="outline" onclick={onCancel}>
      {readOnly ? "Voltar" : "Cancelar"}
    </Button>
    {#if !readOnly}
      <Button type="submit" disabled={isSaving}>
        <Save class="mr-2 size-4" />
        {isSaving ? "Salvando..." : "Salvar local"}
      </Button>
    {/if}
  </div>
</form>
