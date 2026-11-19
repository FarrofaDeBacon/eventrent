<script lang="ts">
  import { ArrowLeft, FolderTree, Plus, Sparkles, Trash2 } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    appSession,
    getCatalogRepository,
    hasPermission,
    PERMISSIONS,
    type AttributeDataType,
    type EquipmentCategoryTreeItem,
    type InheritedAttributeDefinition,
  } from "$lib/data";
  import { Button } from "$lib/ui/button";
  import { Input } from "$lib/ui/input";

  const repo = getCatalogRepository();
  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.CATALOG_MANAGE),
  );

  let tree = $state<EquipmentCategoryTreeItem[]>([]);
  let loading = $state(true);
  let loadingSuggested = $state(false);

  let selectedCategoryId = $state<string | null>(null);
  let effectiveAttrs = $state<InheritedAttributeDefinition[]>([]);

  // New / Edit category form
  let showCategoryForm = $state(false);
  let catParentId = $state("");
  let catName = $state("");
  let catDescription = $state("");
  let catSortOrder = $state("0");
  let savingCat = $state(false);

  // New attribute definition form
  let attrKey = $state("");
  let attrLabel = $state("");
  let attrType = $state<AttributeDataType>("text");
  let attrUnit = $state("");
  let attrChoicesRaw = $state("");
  let attrRequired = $state(false);
  let savingAttr = $state(false);

  let selectedItem = $derived(
    tree.find((item) => item.category.id === selectedCategoryId) ?? null,
  );

  async function refreshTree(selectId?: string) {
    loading = true;
    try {
      const data = await repo.listCategoriesTree(true);
      tree = data;
      const target =
        selectId ??
        (selectedCategoryId &&
        data.some((i) => i.category.id === selectedCategoryId)
          ? selectedCategoryId
          : (data[0]?.category.id ?? null));
      selectedCategoryId = target;
      if (target) {
        effectiveAttrs = await repo.getCategoryEffectiveAttributes(target);
      } else {
        effectiveAttrs = [];
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void refreshTree();
  });

  async function selectCategory(id: string) {
    selectedCategoryId = id;
    try {
      effectiveAttrs = await repo.getCategoryEffectiveAttributes(id);
    } catch {
      effectiveAttrs = [];
    }
  }

  async function handleLoadSuggested() {
    if (!canManage) return;
    loadingSuggested = true;
    try {
      const res = await repo.loadSuggestedCategories();
      toast.success(
        `Sugestões carregadas: ${res.categories_created} categorias e ${res.attributes_created} atributos criados.`,
      );
      await refreshTree();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      loadingSuggested = false;
    }
  }

  async function handleCreateCategory(e: SubmitEvent) {
    e.preventDefault();
    if (!canManage) return;
    savingCat = true;
    try {
      const created = await repo.createCategory({
        parent_id: catParentId || null,
        name: catName.trim(),
        description: catDescription.trim() || null,
        sort_order: Number.parseInt(catSortOrder, 10) || 0,
        active: true,
      });
      toast.success(`Categoria '${created.name}' criada.`);
      catName = "";
      catDescription = "";
      catSortOrder = "0";
      showCategoryForm = false;
      await refreshTree(created.id);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      savingCat = false;
    }
  }

  async function handleDeleteCategory(id: string) {
    if (!canManage) return;
    try {
      await repo.deleteCategory(id);
      toast.success("Categoria excluída.");
      if (selectedCategoryId === id) selectedCategoryId = null;
      await refreshTree();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleAddAttribute(e: SubmitEvent) {
    e.preventDefault();
    if (!canManage || !selectedCategoryId) return;
    savingAttr = true;
    try {
      const choices =
        attrType === "select"
          ? attrChoicesRaw
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean)
          : [];
      await repo.createAttributeDefinition({
        category_id: selectedCategoryId,
        field_key: attrKey.trim(),
        label: attrLabel.trim(),
        data_type: attrType,
        unit: attrUnit.trim() || null,
        choices,
        is_required: attrRequired,
        sort_order: (selectedItem?.own_attributes.length ?? 0) + 1,
        active: true,
      });
      toast.success("Campo técnico adicionado.");
      attrKey = "";
      attrLabel = "";
      attrType = "text";
      attrUnit = "";
      attrChoicesRaw = "";
      attrRequired = false;
      await refreshTree(selectedCategoryId);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      savingAttr = false;
    }
  }

  async function handleDeleteAttribute(defId: string) {
    if (!canManage || !selectedCategoryId) return;
    try {
      await repo.deleteAttributeDefinition(defId);
      toast.success("Campo técnico removido.");
      await refreshTree(selectedCategoryId);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }
</script>

<div class="mx-auto max-w-6xl space-y-6">
  <div class="flex flex-wrap items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <Button
        variant="ghost"
        size="icon"
        href="/catalog"
        aria-label="Voltar para catálogo"
      >
        <ArrowLeft class="h-4 w-4" />
      </Button>
      <div>
        <h1 class="text-2xl font-semibold tracking-tight text-foreground">
          Categorias e Campos Técnicos
        </h1>
        <p class="text-sm text-muted-foreground">
          Árvore de até 4 níveis com herança automática de atributos técnicos
          para os modelos.
        </p>
      </div>
    </div>

    {#if canManage}
      <div class="flex items-center gap-2">
        <Button
          variant="outline"
          disabled={loadingSuggested}
          onclick={handleLoadSuggested}
        >
          <Sparkles class="mr-2 h-4 w-4" />
          {loadingSuggested ? "Carregando..." : "Carregar categorias sugeridas"}
        </Button>
        <Button onclick={() => (showCategoryForm = !showCategoryForm)}>
          <Plus class="mr-2 h-4 w-4" />
          Nova categoria
        </Button>
      </div>
    {/if}
  </div>

  {#if showCategoryForm && canManage}
    <form
      onsubmit={handleCreateCategory}
      class="rounded-xl border border-border bg-card p-5 shadow-xs space-y-4"
    >
      <h2 class="text-base font-semibold text-foreground">Nova Categoria</h2>
      <div class="grid gap-4 md:grid-cols-3">
        <div class="space-y-1.5">
          <label class="text-sm font-medium text-foreground" for="cat-name"
            >Nome *</label
          >
          <Input
            id="cat-name"
            bind:value={catName}
            placeholder="Ex.: Mesas de som"
            required
          />
        </div>
        <div class="space-y-1.5">
          <label class="text-sm font-medium text-foreground" for="cat-parent"
            >Categoria pai (opcional)</label
          >
          <select
            id="cat-parent"
            bind:value={catParentId}
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <option value="">Raiz (Nível 1)</option>
            {#each tree.filter((i) => i.depth < 4) as item (item.category.id)}
              <option value={item.category.id}>
                {"\u00A0".repeat((item.depth - 1) * 2)}{item.category.name} (Nível
                {item.depth})
              </option>
            {/each}
          </select>
        </div>
        <div class="space-y-1.5">
          <label class="text-sm font-medium text-foreground" for="cat-sort"
            >Ordem</label
          >
          <Input id="cat-sort" type="number" bind:value={catSortOrder} />
        </div>
        <div class="space-y-1.5 md:col-span-3">
          <label class="text-sm font-medium text-foreground" for="cat-desc"
            >Descrição</label
          >
          <Input
            id="cat-desc"
            bind:value={catDescription}
            placeholder="Descrição opcional"
          />
        </div>
      </div>
      <div class="flex justify-end gap-2">
        <Button
          type="button"
          variant="outline"
          onclick={() => (showCategoryForm = false)}
        >
          Cancelar
        </Button>
        <Button type="submit" disabled={savingCat}>
          {savingCat ? "Salvando..." : "Criar categoria"}
        </Button>
      </div>
    </form>
  {/if}

  <div class="grid gap-6 lg:grid-cols-12">
    <div
      class="rounded-xl border border-border bg-card p-4 shadow-xs lg:col-span-5"
    >
      <div
        class="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground"
      >
        <FolderTree class="h-4 w-4 text-primary" />
        <span>Árvore de Categorias</span>
      </div>

      {#if loading}
        <p class="py-6 text-center text-sm text-muted-foreground">
          Carregando...
        </p>
      {:else if tree.length === 0}
        <p class="py-6 text-center text-sm text-muted-foreground">
          Nenhuma categoria cadastrada. Use "Carregar categorias sugeridas" ou
          crie manualmente.
        </p>
      {:else}
        <div class="space-y-1">
          {#each tree as item (item.category.id)}
            {@const isSelected = selectedCategoryId === item.category.id}
            <div
              class={`flex items-center justify-between rounded-lg px-3 py-2 text-sm transition-colors ${
                isSelected
                  ? "bg-primary/10 text-primary font-medium"
                  : "hover:bg-muted/50"
              }`}
              style={`padding-left: ${(item.depth - 1) * 18 + 12}px`}
            >
              <button
                type="button"
                class="flex flex-1 items-center gap-2 text-left"
                onclick={() => selectCategory(item.category.id)}
              >
                <span>{item.depth > 1 ? "↳" : "•"}</span>
                <span>{item.category.name}</span>
                <span class="text-xs text-muted-foreground">
                  ({item.own_attributes.length} campos, {item.model_count} mod.)
                </span>
              </button>
              {#if canManage}
                <button
                  type="button"
                  title="Excluir categoria"
                  aria-label={`Excluir ${item.category.name}`}
                  onclick={() => handleDeleteCategory(item.category.id)}
                  class="rounded p-1 text-muted-foreground hover:text-destructive"
                >
                  <Trash2 class="h-3.5 w-3.5" />
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div
      class="rounded-xl border border-border bg-card p-5 shadow-xs space-y-5 lg:col-span-7"
    >
      {#if !selectedItem}
        <p class="py-10 text-center text-sm text-muted-foreground">
          Selecione uma categoria ao lado para visualizar e gerenciar seus
          campos técnicos.
        </p>
      {:else}
        <div>
          <h2 class="text-base font-semibold text-foreground">
            Atributos de "{selectedItem.category.name}" (Nível {selectedItem.depth})
          </h2>
          <p class="text-xs text-muted-foreground">
            Inclui campos próprios desta categoria e campos herdados dos
            ancestrais.
          </p>
        </div>

        <div class="space-y-2">
          {#if effectiveAttrs.length === 0}
            <p class="text-sm text-muted-foreground">
              Nenhum campo técnico configurado para esta categoria ou seus
              ancestrais.
            </p>
          {:else}
            {#each effectiveAttrs as item (item.definition.id)}
              {@const def = item.definition}
              <div
                class="flex items-center justify-between rounded-lg border border-border px-3.5 py-2.5 text-sm"
              >
                <div>
                  <div
                    class="flex items-center gap-2 font-medium text-foreground"
                  >
                    <span>{def.label}</span>
                    <code
                      class="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground"
                    >
                      {def.field_key}
                    </code>
                    {#if item.is_inherited}
                      <span
                        class="rounded bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground"
                      >
                        herdado de {item.source_category_name}
                      </span>
                    {/if}
                  </div>
                  <div class="mt-0.5 text-xs text-muted-foreground">
                    Tipo: {def.data_type}
                    {#if def.unit}
                      • Unidade: {def.unit}
                    {/if}
                    {#if def.choices.length > 0}
                      • Opções: {def.choices.join(", ")}
                    {/if}
                    {#if def.is_required}
                      • Obrigatório
                    {/if}
                  </div>
                </div>
                {#if canManage && !item.is_inherited}
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    aria-label={`Remover ${def.label}`}
                    onclick={() => handleDeleteAttribute(def.id)}
                  >
                    <Trash2 class="h-4 w-4 text-destructive" />
                  </Button>
                {/if}
              </div>
            {/each}
          {/if}
        </div>

        {#if canManage}
          <form
            onsubmit={handleAddAttribute}
            class="border-t border-border pt-4 space-y-4"
          >
            <h3 class="text-sm font-semibold text-foreground">
              Adicionar campo técnico em "{selectedItem.category.name}"
            </h3>
            <div class="grid gap-3 sm:grid-cols-2">
              <div class="space-y-1">
                <label
                  class="text-sm font-medium text-foreground"
                  for="attr-label">Rótulo *</label
                >
                <Input
                  id="attr-label"
                  bind:value={attrLabel}
                  placeholder="Ex.: Canais"
                  required
                />
              </div>
              <div class="space-y-1">
                <label
                  class="text-sm font-medium text-foreground"
                  for="attr-key">Chave técnica (snake_case) *</label
                >
                <Input
                  id="attr-key"
                  bind:value={attrKey}
                  placeholder="Ex.: canais"
                  required
                />
              </div>
              <div class="space-y-1">
                <label
                  class="text-sm font-medium text-foreground"
                  for="attr-type">Tipo *</label
                >
                <select
                  id="attr-type"
                  bind:value={attrType}
                  class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <option value="text">Texto</option>
                  <option value="number">Número</option>
                  <option value="boolean">Sim/Não (Booleano)</option>
                  <option value="select">Lista de opções</option>
                </select>
              </div>
              <div class="space-y-1">
                <label
                  class="text-sm font-medium text-foreground"
                  for="attr-unit">Unidade (opcional)</label
                >
                <Input
                  id="attr-unit"
                  bind:value={attrUnit}
                  placeholder="Ex.: ch, W, mm"
                />
              </div>
              {#if attrType === "select"}
                <div class="space-y-1 sm:col-span-2">
                  <label
                    class="text-sm font-medium text-foreground"
                    for="attr-choices">Opções (separadas por vírgula) *</label
                  >
                  <Input
                    id="attr-choices"
                    bind:value={attrChoicesRaw}
                    placeholder="Ex.: Analógica, Dante, AES50"
                    required
                  />
                </div>
              {/if}
            </div>
            <div class="flex items-center justify-between">
              <label class="flex items-center gap-2 text-sm text-foreground">
                <input
                  type="checkbox"
                  bind:checked={attrRequired}
                  class="h-4 w-4 rounded border-input"
                />
                <span>Campo obrigatório no cadastro de modelos</span>
              </label>
              <Button type="submit" disabled={savingAttr}>
                <Plus class="mr-2 h-4 w-4" />
                {savingAttr ? "Adicionando..." : "Adicionar campo"}
              </Button>
            </div>
          </form>
        {/if}
      {/if}
    </div>
  </div>
</div>
