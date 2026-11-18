<script lang="ts">
  import { Plus, Save, X, Lock } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    getCatalogRepository,
    type AttributeValueInput,
    type Brand,
    type ControlMode,
    type CreateEquipmentModelInput,
    type EquipmentCategoryTreeItem,
    type EquipmentModelWithDetails,
    type InheritedAttributeDefinition,
    type VoltageOption,
  } from "$lib/data";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";

  interface Props {
    initial?: EquipmentModelWithDetails | null;
    readonly?: boolean;
    submitting?: boolean;
    onSubmit: (payload: CreateEquipmentModelInput) => Promise<void>;
    onCancel: () => void;
  }

  let {
    initial = null,
    readonly = false,
    submitting = false,
    onSubmit,
    onCancel,
  }: Props = $props();

  const repo = getCatalogRepository();

  let categories = $state<EquipmentCategoryTreeItem[]>([]);
  let brands = $state<Brand[]>([]);
  let effectiveAttributes = $state<InheritedAttributeDefinition[]>([]);
  let loadingMeta = $state(true);

  let categoryId = $state("");
  let brandId = $state("");
  let name = $state("");
  let modelCode = $state("");
  let sku = $state("");
  let ean = $state("");
  let ncm = $state("");
  let description = $state("");
  let controlMode = $state<ControlMode>("serialized");

  let weightKg = $state("");
  let lengthCm = $state("");
  let widthCm = $state("");
  let heightCm = $state("");
  let powerWatts = $state("");
  let voltage = $state<"" | VoltageOption>("");
  let amperage = $state("");
  let requiresCase = $state(false);
  let unitsPerCase = $state("");

  let dailyRateReais = $state("");
  let replacementReais = $state("");
  let costPriceReais = $state("");
  let active = $state(true);

  // Dynamic custom attribute values keyed by definition_id
  let attrTextValues = $state<Record<string, string>>({});
  let attrNumberValues = $state<Record<string, string>>({});
  let attrBoolValues = $state<Record<string, boolean>>({});

  // Inline brand creation
  let showNewBrand = $state(false);
  let newBrandName = $state("");
  let creatingBrand = $state(false);

  function centsToReaisStr(cents: number | null): string {
    if (cents === null || cents === undefined) return "";
    return (cents / 100).toFixed(2);
  }

  function reaisStrToCents(raw: string): number | null {
    const trimmed = raw.trim().replace(",", ".");
    if (!trimmed) return null;
    const num = Number(trimmed);
    if (!Number.isFinite(num) || num < 0) return null;
    return Math.round(num * 100);
  }

  function parseOptionalNumber(raw: string): number | null {
    const trimmed = raw.trim().replace(",", ".");
    if (!trimmed) return null;
    const num = Number(trimmed);
    return Number.isFinite(num) ? num : null;
  }

  function parseOptionalInt(raw: string): number | null {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    const num = Number.parseInt(trimmed, 10);
    return Number.isFinite(num) ? num : null;
  }

  $effect(() => {
    if (initial) {
      const m = initial.model;
      categoryId = m.category_id;
      brandId = m.brand_id ?? "";
      name = m.name;
      modelCode = m.model_code ?? "";
      sku = m.sku ?? "";
      ean = m.ean ?? "";
      ncm = m.ncm ?? "";
      description = m.description ?? "";
      controlMode = m.control_mode;
      weightKg = m.weight_kg !== null ? String(m.weight_kg) : "";
      lengthCm = m.length_cm !== null ? String(m.length_cm) : "";
      widthCm = m.width_cm !== null ? String(m.width_cm) : "";
      heightCm = m.height_cm !== null ? String(m.height_cm) : "";
      powerWatts = m.power_watts !== null ? String(m.power_watts) : "";
      voltage = m.voltage ?? "";
      amperage = m.amperage !== null ? String(m.amperage) : "";
      requiresCase = m.requires_case;
      unitsPerCase = m.units_per_case !== null ? String(m.units_per_case) : "";
      dailyRateReais = centsToReaisStr(m.daily_rate_cents);
      replacementReais = centsToReaisStr(m.replacement_value_cents);
      costPriceReais = centsToReaisStr(m.cost_price_cents);
      active = m.active;

      const texts: Record<string, string> = {};
      const nums: Record<string, string> = {};
      const bools: Record<string, boolean> = {};
      for (const val of initial.attribute_values) {
        if (val.value_text !== null) texts[val.definition_id] = val.value_text;
        if (val.value_number !== null)
          nums[val.definition_id] = String(val.value_number);
        if (val.value_bool !== null) bools[val.definition_id] = val.value_bool;
      }
      attrTextValues = texts;
      attrNumberValues = nums;
      attrBoolValues = bools;
      effectiveAttributes = initial.effective_attributes;
    }
  });

  async function loadMetadata() {
    loadingMeta = true;
    try {
      const [cats, brs] = await Promise.all([
        repo.listCategoriesTree(false),
        repo.listBrands(false),
      ]);
      categories = cats;
      brands = brs;
      if (!categoryId && cats.length > 0) {
        categoryId = cats[0].category.id;
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      loadingMeta = false;
    }
  }

  $effect(() => {
    void loadMetadata();
  });

  $effect(() => {
    const cid = categoryId;
    if (!cid) {
      effectiveAttributes = [];
      return;
    }
    void repo
      .getCategoryEffectiveAttributes(cid)
      .then((attrs) => {
        effectiveAttributes = attrs;
      })
      .catch(() => {
        effectiveAttributes = [];
      });
  });

  async function handleCreateInlineBrand() {
    const trimmed = newBrandName.trim();
    if (!trimmed) {
      toast.error("Informe o nome da marca.");
      return;
    }
    creatingBrand = true;
    try {
      const created = await repo.createBrand({ name: trimmed, active: true });
      brands = [...brands, created].sort((a, b) =>
        a.name_normalized.localeCompare(b.name_normalized),
      );
      brandId = created.id;
      newBrandName = "";
      showNewBrand = false;
      toast.success(`Marca '${created.name}' criada.`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      creatingBrand = false;
    }
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (readonly) return;

    if (!categoryId) {
      toast.error("Selecione uma categoria para o modelo.");
      return;
    }

    const attribute_values: AttributeValueInput[] = [];
    for (const item of effectiveAttributes) {
      const def = item.definition;
      if (def.data_type === "text" || def.data_type === "select") {
        const val = (attrTextValues[def.id] ?? "").trim();
        if (val) {
          attribute_values.push({
            definition_id: def.id,
            value_text: val,
            value_number: null,
            value_bool: null,
          });
        }
      } else if (def.data_type === "number") {
        const raw = (attrNumberValues[def.id] ?? "").trim().replace(",", ".");
        if (raw) {
          const num = Number(raw);
          if (!Number.isFinite(num)) {
            toast.error(`Valor numérico inválido para '${def.label}'.`);
            return;
          }
          attribute_values.push({
            definition_id: def.id,
            value_text: null,
            value_number: num,
            value_bool: null,
          });
        }
      } else if (def.data_type === "boolean") {
        if (def.id in attrBoolValues) {
          attribute_values.push({
            definition_id: def.id,
            value_text: null,
            value_number: null,
            value_bool: Boolean(attrBoolValues[def.id]),
          });
        }
      }
    }

    await onSubmit({
      category_id: categoryId,
      brand_id: brandId || null,
      name: name.trim(),
      model_code: modelCode.trim() || null,
      sku: sku.trim() || null,
      ean: ean.trim() || null,
      ncm: ncm.trim() || null,
      description: description.trim() || null,
      control_mode: controlMode,
      weight_kg: parseOptionalNumber(weightKg),
      length_cm: parseOptionalNumber(lengthCm),
      width_cm: parseOptionalNumber(widthCm),
      height_cm: parseOptionalNumber(heightCm),
      power_watts: parseOptionalInt(powerWatts),
      voltage: voltage === "" ? null : voltage,
      amperage: parseOptionalNumber(amperage),
      requires_case: requiresCase,
      units_per_case: parseOptionalInt(unitsPerCase),
      daily_rate_cents: reaisStrToCents(dailyRateReais),
      replacement_value_cents: reaisStrToCents(replacementReais),
      cost_price_cents: reaisStrToCents(costPriceReais),
      active,
      attribute_values,
    });
  }
</script>

<form onsubmit={handleSubmit} class="space-y-6">
  {#if readonly}
    <div
      class="flex items-center gap-2 rounded-lg border border-border bg-muted/40 px-4 py-2.5 text-sm text-muted-foreground"
    >
      <Lock class="h-4 w-4 shrink-0" />
      <span
        >Modo somente leitura: seu perfil possui apenas permissão de consulta ao
        catálogo.</span
      >
    </div>
  {/if}

  <div class="rounded-xl border border-border bg-card p-6 shadow-xs space-y-5">
    <h2 class="text-base font-semibold text-foreground">
      Identificação e Classificação
    </h2>

    <div class="grid gap-4 md:grid-cols-2">
      <div class="space-y-1.5 md:col-span-2">
        <label class="text-sm font-medium text-foreground" for="model-name"
          >Nome do modelo *</label
        >
        <Input
          id="model-name"
          bind:value={name}
          placeholder="Ex.: Mesa de Som Digital X32"
          required
          disabled={readonly}
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-category"
          >Categoria *</label
        >
        <select
          id="model-category"
          bind:value={categoryId}
          disabled={readonly || loadingMeta}
          required
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        >
          <option value="" disabled>Selecione uma categoria...</option>
          {#each categories as item (item.category.id)}
            <option value={item.category.id}>
              {"\u00A0".repeat((item.depth - 1) * 3)}{item.depth > 1
                ? "↳ "
                : ""}{item.category.name}
            </option>
          {/each}
        </select>
      </div>

      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <label class="text-sm font-medium text-foreground" for="model-brand"
            >Marca</label
          >
          {#if !readonly}
            <button
              type="button"
              class="inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
              onclick={() => (showNewBrand = !showNewBrand)}
            >
              <Plus class="h-3 w-3" />
              Nova marca
            </button>
          {/if}
        </div>
        <select
          id="model-brand"
          bind:value={brandId}
          disabled={readonly || loadingMeta}
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        >
          <option value="">Sem marca / Genérico</option>
          {#each brands as brand (brand.id)}
            <option value={brand.id}>{brand.name}</option>
          {/each}
        </select>

        {#if showNewBrand && !readonly}
          <div class="mt-2 flex items-center gap-2">
            <Input
              bind:value={newBrandName}
              placeholder="Nome da nova marca"
              aria-label="Nome da nova marca"
            />
            <Button
              type="button"
              size="sm"
              disabled={creatingBrand}
              onclick={handleCreateInlineBrand}
            >
              {creatingBrand ? "..." : "Adicionar"}
            </Button>
          </div>
        {/if}
      </div>

      <div class="space-y-1.5">
        <label
          class="text-sm font-medium text-foreground"
          for="model-control-mode">Modo de controle de estoque *</label
        >
        <select
          id="model-control-mode"
          bind:value={controlMode}
          disabled={readonly}
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        >
          <option value="serialized"
            >Serializado (por nº de série / patrimônio)</option
          >
          <option value="quantity">Quantidade (por saldo agrupado)</option>
        </select>
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-code"
          >Código do modelo</label
        >
        <Input
          id="model-code"
          bind:value={modelCode}
          placeholder="Ex.: X32-COMPACT"
          disabled={readonly}
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-sku"
          >SKU interno</label
        >
        <Input
          id="model-sku"
          bind:value={sku}
          placeholder="Ex.: AUD-MES-001"
          disabled={readonly}
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-ean"
          >EAN / Código de barras</label
        >
        <Input
          id="model-ean"
          bind:value={ean}
          placeholder="8, 12, 13 ou 14 dígitos"
          disabled={readonly}
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-ncm"
          >NCM (8 dígitos)</label
        >
        <Input
          id="model-ncm"
          bind:value={ncm}
          placeholder="Ex.: 85198190"
          disabled={readonly}
        />
      </div>

      <div class="space-y-1.5 md:col-span-2">
        <label
          class="text-sm font-medium text-foreground"
          for="model-description">Descrição técnica</label
        >
        <textarea
          id="model-description"
          bind:value={description}
          rows="3"
          disabled={readonly}
          placeholder="Observações técnicas, acessórios inclusos, compatibilidade..."
          class="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        ></textarea>
      </div>
    </div>
  </div>

  {#if effectiveAttributes.length > 0}
    <div
      class="rounded-xl border border-border bg-card p-6 shadow-xs space-y-4"
    >
      <div>
        <h2 class="text-base font-semibold text-foreground">
          Especificações da Categoria
        </h2>
        <p class="text-xs text-muted-foreground">
          Campos técnicos próprios e herdados da árvore de categorias.
        </p>
      </div>

      <div class="grid gap-4 md:grid-cols-2">
        {#each effectiveAttributes as item (item.definition.id)}
          {@const def = item.definition}
          <div class="space-y-1.5">
            <div class="flex items-center justify-between gap-2">
              <label
                class="text-sm font-medium text-foreground"
                for={`attr-${def.id}`}
              >
                {def.label}
                {#if def.unit}
                  <span class="text-muted-foreground font-normal"
                    >({def.unit})</span
                  >
                {/if}
                {#if def.is_required}
                  <span class="text-destructive">*</span>
                {/if}
              </label>
              {#if item.is_inherited}
                <span
                  class="rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
                >
                  herdado de {item.source_category_name}
                </span>
              {/if}
            </div>

            {#if def.data_type === "text"}
              <Input
                id={`attr-${def.id}`}
                value={attrTextValues[def.id] ?? ""}
                oninput={(ev) => {
                  attrTextValues = {
                    ...attrTextValues,
                    [def.id]: (ev.currentTarget as HTMLInputElement).value,
                  };
                }}
                required={def.is_required}
                disabled={readonly}
              />
            {:else if def.data_type === "number"}
              <Input
                id={`attr-${def.id}`}
                type="number"
                step="any"
                value={attrNumberValues[def.id] ?? ""}
                oninput={(ev) => {
                  attrNumberValues = {
                    ...attrNumberValues,
                    [def.id]: (ev.currentTarget as HTMLInputElement).value,
                  };
                }}
                required={def.is_required}
                disabled={readonly}
              />
            {:else if def.data_type === "select"}
              <select
                id={`attr-${def.id}`}
                value={attrTextValues[def.id] ?? ""}
                onchange={(ev) => {
                  attrTextValues = {
                    ...attrTextValues,
                    [def.id]: (ev.currentTarget as HTMLSelectElement).value,
                  };
                }}
                required={def.is_required}
                disabled={readonly}
                class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
              >
                <option value="">Selecione...</option>
                {#each def.choices as opt (opt)}
                  <option value={opt}>{opt}</option>
                {/each}
              </select>
            {:else if def.data_type === "boolean"}
              <label
                class="flex h-10 items-center gap-2 text-sm text-foreground"
              >
                <input
                  id={`attr-${def.id}`}
                  type="checkbox"
                  checked={Boolean(attrBoolValues[def.id])}
                  onchange={(ev) => {
                    attrBoolValues = {
                      ...attrBoolValues,
                      [def.id]: (ev.currentTarget as HTMLInputElement).checked,
                    };
                  }}
                  disabled={readonly}
                  class="h-4 w-4 rounded border-input"
                />
                <span>Sim</span>
              </label>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="rounded-xl border border-border bg-card p-6 shadow-xs space-y-5">
    <h2 class="text-base font-semibold text-foreground">
      Dimensões, Energia e Acondicionamento
    </h2>

    <div class="grid gap-4 sm:grid-cols-2 md:grid-cols-4">
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-weight"
          >Peso (kg)</label
        >
        <Input
          id="model-weight"
          type="number"
          step="any"
          min="0.001"
          bind:value={weightKg}
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-length"
          >Comprimento (cm)</label
        >
        <Input
          id="model-length"
          type="number"
          step="any"
          min="0.1"
          bind:value={lengthCm}
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-width"
          >Largura (cm)</label
        >
        <Input
          id="model-width"
          type="number"
          step="any"
          min="0.1"
          bind:value={widthCm}
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-height"
          >Altura (cm)</label
        >
        <Input
          id="model-height"
          type="number"
          step="any"
          min="0.1"
          bind:value={heightCm}
          disabled={readonly}
        />
      </div>

      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-watts"
          >Potência (W)</label
        >
        <Input
          id="model-watts"
          type="number"
          min="0"
          step="1"
          bind:value={powerWatts}
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-voltage"
          >Tensão elétrica</label
        >
        <select
          id="model-voltage"
          bind:value={voltage}
          disabled={readonly}
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        >
          <option value="">Não aplicável</option>
          <option value="110">110V</option>
          <option value="220">220V</option>
          <option value="bivolt">Bivolt</option>
        </select>
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-amperage"
          >Corrente (A)</label
        >
        <Input
          id="model-amperage"
          type="number"
          step="any"
          min="0.1"
          bind:value={amperage}
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label
          class="text-sm font-medium text-foreground"
          for="model-units-case">Unidades por case</label
        >
        <Input
          id="model-units-case"
          type="number"
          min="1"
          step="1"
          bind:value={unitsPerCase}
          disabled={readonly}
        />
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-6 pt-1">
      <label class="flex items-center gap-2 text-sm text-foreground">
        <input
          type="checkbox"
          bind:checked={requiresCase}
          disabled={readonly}
          class="h-4 w-4 rounded border-input"
        />
        <span>Requer case / flight case no transporte</span>
      </label>

      <label class="flex items-center gap-2 text-sm text-foreground">
        <input
          type="checkbox"
          bind:checked={active}
          disabled={readonly}
          class="h-4 w-4 rounded border-input"
        />
        <span>Modelo ativo no catálogo</span>
      </label>
    </div>
  </div>

  <div class="rounded-xl border border-border bg-card p-6 shadow-xs space-y-5">
    <h2 class="text-base font-semibold text-foreground">
      Precificação Base (R$)
    </h2>

    <div class="grid gap-4 md:grid-cols-3">
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-daily"
          >Diária padrão (R$)</label
        >
        <Input
          id="model-daily"
          type="number"
          step="0.01"
          min="0"
          bind:value={dailyRateReais}
          placeholder="0.00"
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label
          class="text-sm font-medium text-foreground"
          for="model-replacement">Valor de reposição (R$)</label
        >
        <Input
          id="model-replacement"
          type="number"
          step="0.01"
          min="0"
          bind:value={replacementReais}
          placeholder="0.00"
          disabled={readonly}
        />
      </div>
      <div class="space-y-1.5">
        <label class="text-sm font-medium text-foreground" for="model-cost"
          >Custo de aquisição (R$)</label
        >
        <Input
          id="model-cost"
          type="number"
          step="0.01"
          min="0"
          bind:value={costPriceReais}
          placeholder="0.00"
          disabled={readonly}
        />
      </div>
    </div>
  </div>

  <div class="flex items-center justify-end gap-3">
    <Button type="button" variant="outline" onclick={onCancel}>
      <X class="mr-2 h-4 w-4" />
      {readonly ? "Voltar" : "Cancelar"}
    </Button>
    {#if !readonly}
      <Button type="submit" disabled={submitting}>
        <Save class="mr-2 h-4 w-4" />
        {submitting ? "Salvando..." : "Salvar modelo"}
      </Button>
    {/if}
  </div>
</form>
