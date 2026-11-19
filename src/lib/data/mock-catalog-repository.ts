import type {
  AttachModelDocumentInput,
  AttachModelPhotoInput,
  AttributeDefinition,
  Brand,
  CatalogListFilter,
  CatalogListResult,
  CatalogRepository,
  CreateAttributeDefinitionInput,
  CreateBrandInput,
  CreateCategoryInput,
  CreateEquipmentModelInput,
  EquipmentCategory,
  EquipmentCategoryTreeItem,
  EquipmentModel,
  EquipmentModelAttributeValue,
  EquipmentModelListItem,
  EquipmentModelMediaView,
  EquipmentModelWithDetails,
  InheritedAttributeDefinition,
  MediaGarbageCollectionResult,
  ReorderModelMediaInput,
  SuggestedCategoriesResult,
  UpdateAttributeDefinitionInput,
  UpdateBrandInput,
  UpdateCategoryInput,
  UpdateEquipmentModelInput,
  UpdateModelMediaInput,
} from "./catalog-repository";
import { appSession } from "./session.svelte";
import { PERMISSIONS } from "./types";

function normalizeText(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function nowIso(): string {
  return new Date().toISOString();
}

function makeId(prefix: string): string {
  return `${prefix}-${crypto.randomUUID()}`;
}

function validateEanDigits(raw: string | null): string | null {
  if (!raw || !raw.trim()) return null;
  const digits = raw.replace(/[\s-]/g, "");
  if (!/^\d{8}$|^\d{12}$|^\d{13}$|^\d{14}$/.test(digits)) {
    throw new Error(
      "Código EAN/GTIN inválido. Informe 8, 12, 13 ou 14 dígitos numéricos.",
    );
  }
  const check = Number(digits[digits.length - 1]);
  let sum = 0;
  for (let i = digits.length - 2, pos = 0; i >= 0; i--, pos++) {
    const d = Number(digits[i]);
    sum += d * (pos % 2 === 0 ? 3 : 1);
  }
  const expected = (10 - (sum % 10)) % 10;
  if (check !== expected) {
    throw new Error("Código EAN/GTIN inválido: dígito verificador incorreto.");
  }
  return digits;
}

function validateNcmDigits(raw: string | null): string | null {
  if (!raw || !raw.trim()) return null;
  const digits = raw.replace(/[.\s-]/g, "");
  if (!/^\d{8}$/.test(digits)) {
    throw new Error(
      "Código NCM inválido. Deve conter exatamente 8 dígitos numéricos.",
    );
  }
  return digits;
}

export class MockCatalogRepository implements CatalogRepository {
  private brands: Brand[] = [];
  private categories: EquipmentCategory[] = [];
  private definitions: AttributeDefinition[] = [];
  private models: EquipmentModel[] = [];
  private attrValues: EquipmentModelAttributeValue[] = [];
  private mediaItems: EquipmentModelMediaView[] = [];
  private blobStore = new Map<string, Uint8Array>();

  private tenantId(): string {
    return appSession.session?.tenant_id ?? "tenant-demo";
  }

  private userId(): string | null {
    return appSession.session?.user_id ?? "user-demo";
  }

  private assertPermission(perm: string): void {
    const session = appSession.session;
    if (!session) return;
    if (!session.permissions.includes(perm)) {
      throw new Error(`Acesso negado: permissão '${perm}' necessária.`);
    }
  }

  async listBrands(includeInactive = false): Promise<Brand[]> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    return this.brands
      .filter(
        (b) =>
          b.tenant_id === tid && !b.deleted_at && (includeInactive || b.active),
      )
      .sort((a, b) => a.name_normalized.localeCompare(b.name_normalized));
  }

  async createBrand(input: CreateBrandInput): Promise<Brand> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const name = input.name.trim();
    if (!name) throw new Error("Nome da marca é obrigatório.");
    const norm = normalizeText(name);
    const tid = this.tenantId();
    if (
      this.brands.some(
        (b) =>
          b.tenant_id === tid && !b.deleted_at && b.name_normalized === norm,
      )
    ) {
      throw new Error(`Já existe uma marca cadastrada com o nome '${name}'.`);
    }
    const ts = nowIso();
    const brand: Brand = {
      id: makeId("brand"),
      tenant_id: tid,
      name,
      name_normalized: norm,
      active: input.active,
      created_at: ts,
      updated_at: ts,
      deleted_at: null,
      created_by: this.userId(),
    };
    this.brands.push(brand);
    return brand;
  }

  async updateBrand(id: string, input: UpdateBrandInput): Promise<Brand> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.brands.findIndex(
      (b) => b.tenant_id === tid && b.id === id && !b.deleted_at,
    );
    if (idx === -1) throw new Error("Marca não encontrada.");
    const existing = this.brands[idx];
    if (existing.updated_at !== input.expected_updated_at) {
      throw new Error(
        "A marca foi alterada por outra pessoa. Recarregue e tente novamente.",
      );
    }
    const name = input.name.trim();
    if (!name) throw new Error("Nome da marca é obrigatório.");
    const norm = normalizeText(name);
    if (
      this.brands.some(
        (b) =>
          b.tenant_id === tid &&
          !b.deleted_at &&
          b.id !== id &&
          b.name_normalized === norm,
      )
    ) {
      throw new Error(`Já existe outra marca cadastrada com o nome '${name}'.`);
    }
    const updated: Brand = {
      ...existing,
      name,
      name_normalized: norm,
      active: input.active,
      updated_at: nowIso(),
    };
    this.brands[idx] = updated;
    return updated;
  }

  async deleteBrand(id: string): Promise<void> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.brands.findIndex(
      (b) => b.tenant_id === tid && b.id === id && !b.deleted_at,
    );
    if (idx === -1) throw new Error("Marca não encontrada.");
    if (
      this.models.some(
        (m) => m.tenant_id === tid && !m.deleted_at && m.brand_id === id,
      )
    ) {
      throw new Error(
        "Não é possível excluir a marca pois existem modelos de equipamentos vinculados.",
      );
    }
    const ts = nowIso();
    this.brands[idx] = { ...this.brands[idx], deleted_at: ts, updated_at: ts };
  }

  async listCategoriesTree(
    includeInactive = false,
  ): Promise<EquipmentCategoryTreeItem[]> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    const cats = this.categories.filter(
      (c) =>
        c.tenant_id === tid && !c.deleted_at && (includeInactive || c.active),
    );
    const byParent = new Map<string | null, EquipmentCategory[]>();
    for (const c of cats) {
      const list = byParent.get(c.parent_id) ?? [];
      list.push(c);
      byParent.set(c.parent_id, list);
    }
    for (const list of byParent.values()) {
      list.sort(
        (a, b) =>
          a.sort_order - b.sort_order ||
          a.name_normalized.localeCompare(b.name_normalized),
      );
    }

    const result: EquipmentCategoryTreeItem[] = [];
    const walk = (parentId: string | null, depth: number) => {
      for (const cat of byParent.get(parentId) ?? []) {
        const own_attributes = this.definitions
          .filter(
            (d) =>
              d.tenant_id === tid && d.category_id === cat.id && !d.deleted_at,
          )
          .sort(
            (a, b) =>
              a.sort_order - b.sort_order || a.label.localeCompare(b.label),
          );
        const model_count = this.models.filter(
          (m) =>
            m.tenant_id === tid && !m.deleted_at && m.category_id === cat.id,
        ).length;
        result.push({ category: cat, depth, own_attributes, model_count });
        walk(cat.id, depth + 1);
      }
    };
    walk(null, 1);
    return result;
  }

  async getCategoryEffectiveAttributes(
    categoryId: string,
  ): Promise<InheritedAttributeDefinition[]> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    const cats = this.categories.filter(
      (c) => c.tenant_id === tid && !c.deleted_at,
    );
    const byId = new Map(cats.map((c) => [c.id, c]));
    if (!byId.has(categoryId)) throw new Error("Categoria não encontrada.");

    const chain: EquipmentCategory[] = [];
    let cursor: string | null = categoryId;
    const seen = new Set<string>();
    while (cursor && !seen.has(cursor)) {
      seen.add(cursor);
      const cat = byId.get(cursor);
      if (!cat) break;
      chain.push(cat);
      cursor = cat.parent_id;
    }
    chain.reverse();

    const out: InheritedAttributeDefinition[] = [];
    for (const cat of chain) {
      const defs = this.definitions
        .filter(
          (d) =>
            d.tenant_id === tid &&
            d.category_id === cat.id &&
            !d.deleted_at &&
            d.active,
        )
        .sort(
          (a, b) =>
            a.sort_order - b.sort_order || a.label.localeCompare(b.label),
        );
      for (const def of defs) {
        out.push({
          definition: def,
          source_category_id: cat.id,
          source_category_name: cat.name,
          is_inherited: cat.id !== categoryId,
        });
      }
    }
    return out;
  }

  private validateTreePlacement(
    tid: string,
    selfId: string | null,
    parentId: string | null,
  ): void {
    const cats = this.categories.filter(
      (c) => c.tenant_id === tid && !c.deleted_at,
    );
    const byId = new Map(cats.map((c) => [c.id, c]));
    if (selfId && parentId === selfId) {
      throw new Error("Uma categoria não pode ser pai de si mesma.");
    }
    let parentDepth = 0;
    let cursor = parentId;
    const seen = new Set<string>();
    while (cursor) {
      if (seen.has(cursor) || cursor === selfId) {
        throw new Error("Ciclo detectado na hierarquia de categorias.");
      }
      seen.add(cursor);
      const node = byId.get(cursor);
      if (!node) throw new Error("Categoria pai não encontrada.");
      parentDepth += 1;
      if (parentDepth >= 4) {
        throw new Error(
          "Profundidade máxima de 4 níveis na árvore de categorias excedida.",
        );
      }
      cursor = node.parent_id;
    }
  }

  async createCategory(input: CreateCategoryInput): Promise<EquipmentCategory> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const name = input.name.trim();
    if (!name) throw new Error("Nome da categoria é obrigatório.");
    const norm = normalizeText(name);
    const parentId = input.parent_id?.trim() || null;
    this.validateTreePlacement(tid, null, parentId);
    if (
      this.categories.some(
        (c) =>
          c.tenant_id === tid &&
          !c.deleted_at &&
          c.parent_id === parentId &&
          c.name_normalized === norm,
      )
    ) {
      throw new Error(`Já existe uma categoria '${name}' neste nível.`);
    }
    const ts = nowIso();
    const cat: EquipmentCategory = {
      id: makeId("cat"),
      tenant_id: tid,
      parent_id: parentId,
      name,
      name_normalized: norm,
      description: input.description?.trim() || null,
      sort_order: input.sort_order,
      active: input.active,
      created_at: ts,
      updated_at: ts,
      deleted_at: null,
      created_by: this.userId(),
    };
    this.categories.push(cat);
    return cat;
  }

  async updateCategory(
    id: string,
    input: UpdateCategoryInput,
  ): Promise<EquipmentCategory> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.categories.findIndex(
      (c) => c.tenant_id === tid && c.id === id && !c.deleted_at,
    );
    if (idx === -1) throw new Error("Categoria não encontrada.");
    const existing = this.categories[idx];
    if (existing.updated_at !== input.expected_updated_at) {
      throw new Error(
        "A categoria foi alterada por outra pessoa. Recarregue e tente novamente.",
      );
    }
    const name = input.name.trim();
    if (!name) throw new Error("Nome da categoria é obrigatório.");
    const norm = normalizeText(name);
    const parentId = input.parent_id?.trim() || null;
    this.validateTreePlacement(tid, id, parentId);
    if (
      this.categories.some(
        (c) =>
          c.tenant_id === tid &&
          !c.deleted_at &&
          c.id !== id &&
          c.parent_id === parentId &&
          c.name_normalized === norm,
      )
    ) {
      throw new Error(`Já existe outra categoria '${name}' neste nível.`);
    }
    const updated: EquipmentCategory = {
      ...existing,
      parent_id: parentId,
      name,
      name_normalized: norm,
      description: input.description?.trim() || null,
      sort_order: input.sort_order,
      active: input.active,
      updated_at: nowIso(),
    };
    this.categories[idx] = updated;
    return updated;
  }

  async deleteCategory(id: string): Promise<void> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.categories.findIndex(
      (c) => c.tenant_id === tid && c.id === id && !c.deleted_at,
    );
    if (idx === -1) throw new Error("Categoria não encontrada.");
    if (
      this.categories.some(
        (c) => c.tenant_id === tid && !c.deleted_at && c.parent_id === id,
      )
    ) {
      throw new Error(
        "Não é possível excluir a categoria pois ela possui subcategorias ativas.",
      );
    }
    if (
      this.models.some(
        (m) => m.tenant_id === tid && !m.deleted_at && m.category_id === id,
      )
    ) {
      throw new Error(
        "Não é possível excluir a categoria pois existem modelos de equipamentos vinculados.",
      );
    }
    const ts = nowIso();
    this.categories[idx] = {
      ...this.categories[idx],
      deleted_at: ts,
      updated_at: ts,
    };
  }

  async createAttributeDefinition(
    input: CreateAttributeDefinitionInput,
  ): Promise<AttributeDefinition> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const fieldKey = input.field_key.trim().toLowerCase();
    if (!/^[a-z0-9_]{1,60}$/.test(fieldKey)) {
      throw new Error(
        "Chave do atributo deve conter apenas letras minúsculas, números e '_'.",
      );
    }
    const label = input.label.trim();
    if (!label) throw new Error("Rótulo do atributo é obrigatório.");
    const siblingDefs = this.definitions.filter(
      (d) =>
        d.tenant_id === tid &&
        d.category_id === input.category_id &&
        !d.deleted_at,
    );
    if (siblingDefs.length >= 50) {
      throw new Error("Limite máximo de 50 atributos por categoria atingido.");
    }
    if (siblingDefs.some((d) => d.field_key === fieldKey)) {
      throw new Error(
        `Já existe um atributo com a chave '${fieldKey}' nesta categoria.`,
      );
    }
    const choices =
      input.data_type === "select"
        ? input.choices.map((c) => c.trim()).filter(Boolean)
        : [];
    if (input.data_type === "select" && choices.length === 0) {
      throw new Error(
        "Atributos do tipo seleção devem ter pelo menos 1 opção.",
      );
    }
    const ts = nowIso();
    const def: AttributeDefinition = {
      id: makeId("attr"),
      tenant_id: tid,
      category_id: input.category_id,
      field_key: fieldKey,
      label,
      data_type: input.data_type,
      unit: input.unit?.trim() || null,
      choices,
      is_required: input.is_required,
      sort_order: input.sort_order,
      active: input.active,
      created_at: ts,
      updated_at: ts,
      deleted_at: null,
      created_by: this.userId(),
    };
    this.definitions.push(def);
    return def;
  }

  async updateAttributeDefinition(
    id: string,
    input: UpdateAttributeDefinitionInput,
  ): Promise<AttributeDefinition> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.definitions.findIndex(
      (d) => d.tenant_id === tid && d.id === id && !d.deleted_at,
    );
    if (idx === -1) throw new Error("Definição de atributo não encontrada.");
    const existing = this.definitions[idx];
    if (existing.updated_at !== input.expected_updated_at) {
      throw new Error(
        "O atributo foi alterado por outra pessoa. Recarregue e tente novamente.",
      );
    }
    const fieldKey = input.field_key.trim().toLowerCase();
    const label = input.label.trim();
    const choices =
      input.data_type === "select"
        ? input.choices.map((c) => c.trim()).filter(Boolean)
        : [];
    const activeValues = this.attrValues.filter(
      (v) =>
        v.tenant_id === tid &&
        v.definition_id === id &&
        !v.deleted_at &&
        this.models.some(
          (m) => m.tenant_id === tid && m.id === v.model_id && !m.deleted_at,
        ) &&
        (v.value_text !== null ||
          v.value_number !== null ||
          v.value_bool !== null),
    );
    if (input.data_type !== existing.data_type && activeValues.length > 0) {
      throw new Error(
        `Não é possível alterar o tipo do atributo '${existing.label}' pois ele já possui valores preenchidos em ${activeValues.length} modelo(s).`,
      );
    }
    if (existing.data_type === "select" && input.data_type === "select") {
      const removedInUse = Array.from(
        new Set(
          activeValues
            .map((v) => v.value_text)
            .filter(
              (val): val is string => Boolean(val) && !choices.includes(val!),
            ),
        ),
      );
      if (removedInUse.length > 0) {
        throw new Error(
          `Não é possível remover opções do atributo '${existing.label}' que já estão em uso: ${removedInUse.join(", ")}.`,
        );
      }
    }
    const updated: AttributeDefinition = {
      ...existing,
      field_key: fieldKey,
      label,
      data_type: input.data_type,
      unit: input.unit?.trim() || null,
      choices,
      is_required: input.is_required,
      sort_order: input.sort_order,
      active: input.active,
      updated_at: nowIso(),
    };
    this.definitions[idx] = updated;
    return updated;
  }

  async deleteAttributeDefinition(id: string): Promise<void> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.definitions.findIndex(
      (d) => d.tenant_id === tid && d.id === id && !d.deleted_at,
    );
    if (idx === -1) throw new Error("Definição de atributo não encontrada.");
    const ts = nowIso();
    this.definitions[idx] = {
      ...this.definitions[idx],
      deleted_at: ts,
      updated_at: ts,
    };
  }

  private collectSubtreeIds(tid: string, rootId: string): Set<string> {
    const cats = this.categories.filter(
      (c) => c.tenant_id === tid && !c.deleted_at,
    );
    const out = new Set<string>([rootId]);
    let changed = true;
    while (changed) {
      changed = false;
      for (const c of cats) {
        if (c.parent_id && out.has(c.parent_id) && !out.has(c.id)) {
          out.add(c.id);
          changed = true;
        }
      }
    }
    return out;
  }

  async listEquipmentModels(
    filter: CatalogListFilter,
  ): Promise<CatalogListResult> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    const subtree = filter.category_id
      ? this.collectSubtreeIds(tid, filter.category_id)
      : null;
    const terms = filter.query
      ? normalizeText(filter.query).split(/\s+/).filter(Boolean)
      : [];

    const filtered = this.models
      .filter((m) => {
        if (m.tenant_id !== tid || m.deleted_at) return false;
        if (
          filter.active !== undefined &&
          filter.active !== null &&
          m.active !== filter.active
        ) {
          return false;
        }
        if (filter.brand_id && m.brand_id !== filter.brand_id) return false;
        if (filter.control_mode && m.control_mode !== filter.control_mode)
          return false;
        if (subtree && !subtree.has(m.category_id)) return false;
        for (const t of terms) {
          if (!m.search_text.includes(t)) return false;
        }
        return true;
      })
      .sort(
        (a, b) =>
          a.name_sort.localeCompare(b.name_sort) || a.id.localeCompare(b.id),
      );

    const total_count = filtered.length;
    const limit = Math.max(1, Math.min(100, filter.limit || 50));
    const offset = filter.cursor ? Number(filter.cursor) || 0 : 0;
    const slice = filtered.slice(offset, offset + limit);
    const next_cursor =
      offset + limit < total_count ? String(offset + limit) : null;

    const items: EquipmentModelListItem[] = slice.map((m) => {
      const primaryPhoto = this.mediaItems.find(
        (mi) =>
          mi.tenant_id === tid &&
          mi.model_id === m.id &&
          mi.kind === "image" &&
          mi.is_primary,
      );
      return {
        model: m,
        category_name:
          this.categories.find((c) => c.id === m.category_id)?.name ?? "",
        brand_name: m.brand_id
          ? (this.brands.find((b) => b.id === m.brand_id)?.name ?? null)
          : null,
        primary_thumb_blob_id: primaryPhoto
          ? (primaryPhoto.thumb_blob_id ?? primaryPhoto.blob_id)
          : null,
        primary_thumb_mime_type: primaryPhoto
          ? (primaryPhoto.thumb_mime_type ?? primaryPhoto.mime_type)
          : null,
      };
    });

    return { items, next_cursor, total_count };
  }

  async getEquipmentModel(id: string): Promise<EquipmentModelWithDetails> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    const model = this.models.find(
      (m) => m.tenant_id === tid && m.id === id && !m.deleted_at,
    );
    if (!model) throw new Error("Modelo de equipamento não encontrado.");
    const category_name =
      this.categories.find((c) => c.id === model.category_id)?.name ?? "";
    const brand_name = model.brand_id
      ? (this.brands.find((b) => b.id === model.brand_id)?.name ?? null)
      : null;
    const effective_attributes = await this.getCategoryEffectiveAttributes(
      model.category_id,
    );
    const attribute_values = this.attrValues.filter(
      (v) => v.tenant_id === tid && v.model_id === model.id && !v.deleted_at,
    );
    return {
      model,
      category_name,
      brand_name,
      effective_attributes,
      attribute_values,
    };
  }

  async createEquipmentModel(
    input: CreateEquipmentModelInput,
  ): Promise<EquipmentModelWithDetails> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const name = input.name.trim();
    if (!name) throw new Error("Nome do modelo é obrigatório.");
    const ean = validateEanDigits(input.ean);
    const ncm = validateNcmDigits(input.ncm);
    const category = this.categories.find(
      (c) => c.tenant_id === tid && c.id === input.category_id && !c.deleted_at,
    );
    if (!category) throw new Error("Categoria selecionada não encontrada.");
    const brand = input.brand_id
      ? this.brands.find(
          (b) =>
            b.tenant_id === tid && b.id === input.brand_id && !b.deleted_at,
        )
      : null;
    if (input.brand_id && !brand)
      throw new Error("Marca selecionada não encontrada.");

    const ts = nowIso();
    const id = makeId("model");
    const search_text = normalizeText(
      [
        name,
        category.name,
        brand?.name,
        input.model_code,
        input.sku,
        ean,
        ncm,
        input.description,
      ]
        .filter(Boolean)
        .join(" "),
    );

    const model: EquipmentModel = {
      id,
      tenant_id: tid,
      category_id: category.id,
      brand_id: brand?.id ?? null,
      name,
      name_sort: normalizeText(name),
      model_code: input.model_code?.trim() || null,
      sku: input.sku?.trim() || null,
      ean,
      ncm,
      description: input.description?.trim() || null,
      control_mode: input.control_mode,
      weight_kg: input.weight_kg,
      length_cm: input.length_cm,
      width_cm: input.width_cm,
      height_cm: input.height_cm,
      power_watts: input.power_watts,
      voltage: input.voltage,
      amperage: input.amperage,
      requires_case: input.requires_case,
      units_per_case: input.units_per_case,
      daily_rate_cents: input.daily_rate_cents,
      replacement_value_cents: input.replacement_value_cents,
      cost_price_cents: input.cost_price_cents,
      active: input.active,
      search_text,
      created_at: ts,
      updated_at: ts,
      deleted_at: null,
      created_by: this.userId(),
    };
    this.models.push(model);

    for (const av of input.attribute_values) {
      if (
        av.value_text !== null ||
        av.value_number !== null ||
        av.value_bool !== null
      ) {
        this.attrValues.push({
          id: makeId("aval"),
          tenant_id: tid,
          model_id: id,
          definition_id: av.definition_id,
          value_text: av.value_text,
          value_number: av.value_number,
          value_bool: av.value_bool,
          created_at: ts,
          updated_at: ts,
          deleted_at: null,
          created_by: this.userId(),
        });
      }
    }

    return this.getEquipmentModel(id);
  }

  async updateEquipmentModel(
    id: string,
    input: UpdateEquipmentModelInput,
  ): Promise<EquipmentModelWithDetails> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.models.findIndex(
      (m) => m.tenant_id === tid && m.id === id && !m.deleted_at,
    );
    if (idx === -1) throw new Error("Modelo de equipamento não encontrado.");
    const existing = this.models[idx];
    if (existing.updated_at !== input.expected_updated_at) {
      throw new Error(
        "O modelo foi alterado por outra pessoa. Recarregue e tente novamente.",
      );
    }
    const name = input.name.trim();
    if (!name) throw new Error("Nome do modelo é obrigatório.");
    const ean = validateEanDigits(input.ean);
    const ncm = validateNcmDigits(input.ncm);
    const category = this.categories.find(
      (c) => c.tenant_id === tid && c.id === input.category_id && !c.deleted_at,
    );
    if (!category) throw new Error("Categoria selecionada não encontrada.");
    const brand = input.brand_id
      ? this.brands.find(
          (b) =>
            b.tenant_id === tid && b.id === input.brand_id && !b.deleted_at,
        )
      : null;

    const ts = nowIso();
    const search_text = normalizeText(
      [
        name,
        category.name,
        brand?.name,
        input.model_code,
        input.sku,
        ean,
        ncm,
        input.description,
      ]
        .filter(Boolean)
        .join(" "),
    );

    this.models[idx] = {
      ...existing,
      category_id: category.id,
      brand_id: brand?.id ?? null,
      name,
      name_sort: normalizeText(name),
      model_code: input.model_code?.trim() || null,
      sku: input.sku?.trim() || null,
      ean,
      ncm,
      description: input.description?.trim() || null,
      control_mode: input.control_mode,
      weight_kg: input.weight_kg,
      length_cm: input.length_cm,
      width_cm: input.width_cm,
      height_cm: input.height_cm,
      power_watts: input.power_watts,
      voltage: input.voltage,
      amperage: input.amperage,
      requires_case: input.requires_case,
      units_per_case: input.units_per_case,
      daily_rate_cents: input.daily_rate_cents,
      replacement_value_cents: input.replacement_value_cents,
      cost_price_cents: input.cost_price_cents,
      active: input.active,
      search_text,
      updated_at: ts,
    };

    for (let i = 0; i < this.attrValues.length; i++) {
      if (
        this.attrValues[i].model_id === id &&
        !this.attrValues[i].deleted_at
      ) {
        this.attrValues[i] = {
          ...this.attrValues[i],
          deleted_at: ts,
          updated_at: ts,
        };
      }
    }
    for (const av of input.attribute_values) {
      if (
        av.value_text !== null ||
        av.value_number !== null ||
        av.value_bool !== null
      ) {
        this.attrValues.push({
          id: makeId("aval"),
          tenant_id: tid,
          model_id: id,
          definition_id: av.definition_id,
          value_text: av.value_text,
          value_number: av.value_number,
          value_bool: av.value_bool,
          created_at: ts,
          updated_at: ts,
          deleted_at: null,
          created_by: this.userId(),
        });
      }
    }

    return this.getEquipmentModel(id);
  }

  async setEquipmentModelActive(id: string, active: boolean): Promise<void> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.models.findIndex(
      (m) => m.tenant_id === tid && m.id === id && !m.deleted_at,
    );
    if (idx === -1) throw new Error("Modelo de equipamento não encontrado.");
    this.models[idx] = { ...this.models[idx], active, updated_at: nowIso() };
  }

  async deleteEquipmentModel(id: string): Promise<void> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.models.findIndex(
      (m) => m.tenant_id === tid && m.id === id && !m.deleted_at,
    );
    if (idx === -1) throw new Error("Modelo de equipamento não encontrado.");
    const ts = nowIso();
    this.models[idx] = { ...this.models[idx], deleted_at: ts, updated_at: ts };
  }

  async loadSuggestedCategories(): Promise<SuggestedCategoriesResult> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const specs: Array<{
      parent?: string;
      name: string;
      sort_order: number;
      attrs: Array<{
        field_key: string;
        label: string;
        data_type: "text" | "number" | "boolean" | "select";
        unit?: string;
        choices?: string[];
      }>;
    }> = [
      {
        name: "Áudio",
        sort_order: 1,
        attrs: [
          {
            field_key: "impedancia_ohms",
            label: "Impedância",
            data_type: "number",
            unit: "Ω",
          },
        ],
      },
      {
        parent: "Áudio",
        name: "Mesas de som",
        sort_order: 1,
        attrs: [
          {
            field_key: "canais",
            label: "Canais",
            data_type: "number",
            unit: "ch",
          },
          {
            field_key: "protocolo_digital",
            label: "Protocolo Digital",
            data_type: "select",
            choices: ["Analógica", "Dante", "AES50", "MADI", "USB"],
          },
        ],
      },
      {
        parent: "Áudio",
        name: "Microfones",
        sort_order: 2,
        attrs: [
          {
            field_key: "tipo_captacao",
            label: "Tipo de Captação",
            data_type: "select",
            choices: ["Dinâmico", "Condensador", "Fita"],
          },
          { field_key: "sem_fio", label: "Sem fio", data_type: "boolean" },
        ],
      },
      {
        parent: "Áudio",
        name: "Caixas de som",
        sort_order: 3,
        attrs: [
          {
            field_key: "potencia_rms_w",
            label: "Potência RMS",
            data_type: "number",
            unit: "W",
          },
          { field_key: "caixa_ativa", label: "Ativa", data_type: "boolean" },
        ],
      },
      {
        name: "Iluminação",
        sort_order: 2,
        attrs: [
          {
            field_key: "protocolo_dmx",
            label: "Protocolo DMX",
            data_type: "select",
            choices: ["DMX512", "Art-Net", "sACN", "Sem DMX"],
          },
        ],
      },
      {
        parent: "Iluminação",
        name: "Moving Heads",
        sort_order: 1,
        attrs: [
          {
            field_key: "tipo_feixe",
            label: "Tipo de Feixe",
            data_type: "select",
            choices: ["Beam", "Spot", "Wash", "Híbrido"],
          },
          {
            field_key: "canais_dmx",
            label: "Canais DMX",
            data_type: "number",
            unit: "ch",
          },
        ],
      },
      {
        parent: "Iluminação",
        name: "Refletores LED",
        sort_order: 2,
        attrs: [
          {
            field_key: "grau_abertura",
            label: "Grau de Abertura",
            data_type: "number",
            unit: "°",
          },
        ],
      },
      {
        name: "Vídeo e Projeção",
        sort_order: 3,
        attrs: [
          {
            field_key: "resolucao_nativa",
            label: "Resolução Nativa",
            data_type: "select",
            choices: ["HD", "Full HD", "WUXGA", "4K UHD"],
          },
        ],
      },
      {
        parent: "Vídeo e Projeção",
        name: "Projetores",
        sort_order: 1,
        attrs: [
          {
            field_key: "lumens_ansi",
            label: "Luminosidade",
            data_type: "number",
            unit: "lm",
          },
        ],
      },
      {
        parent: "Vídeo e Projeção",
        name: "Painéis de LED",
        sort_order: 2,
        attrs: [
          {
            field_key: "pixel_pitch_mm",
            label: "Pixel Pitch",
            data_type: "number",
            unit: "mm",
          },
          {
            field_key: "uso_externo_ip65",
            label: "Uso Externo (IP65)",
            data_type: "boolean",
          },
        ],
      },
      {
        name: "Informática",
        sort_order: 4,
        attrs: [
          {
            field_key: "sistema_operacional",
            label: "Sistema Operacional",
            data_type: "text",
          },
        ],
      },
      { parent: "Informática", name: "Notebooks", sort_order: 1, attrs: [] },
      {
        name: "Estrutura e Box Truss",
        sort_order: 5,
        attrs: [
          {
            field_key: "material_estrutura",
            label: "Material",
            data_type: "select",
            choices: ["Alumínio", "Aço"],
          },
        ],
      },
      {
        parent: "Estrutura e Box Truss",
        name: "Treliças Q30",
        sort_order: 1,
        attrs: [],
      },
      {
        name: "Energia e Cabeamento",
        sort_order: 6,
        attrs: [
          {
            field_key: "comprimento_m",
            label: "Comprimento",
            data_type: "number",
            unit: "m",
          },
          { field_key: "conector", label: "Conector", data_type: "text" },
        ],
      },
      {
        parent: "Energia e Cabeamento",
        name: "Cabos de Áudio",
        sort_order: 1,
        attrs: [],
      },
      {
        parent: "Energia e Cabeamento",
        name: "Cabos de Energia",
        sort_order: 2,
        attrs: [],
      },
      {
        name: "DJ",
        sort_order: 7,
        attrs: [],
      },
      {
        parent: "DJ",
        name: "Players (CDJ)",
        sort_order: 1,
        attrs: [
          {
            field_key: "pro_dj_link",
            label: "Pro DJ Link",
            data_type: "boolean",
          },
        ],
      },
      {
        parent: "DJ",
        name: "Mixers DJ",
        sort_order: 2,
        attrs: [
          {
            field_key: "canais",
            label: "Canais",
            data_type: "number",
            unit: "ch",
          },
          { field_key: "efeitos", label: "Efeitos", data_type: "boolean" },
        ],
      },
      {
        parent: "DJ",
        name: "Controladoras DJ",
        sort_order: 3,
        attrs: [
          {
            field_key: "canais",
            label: "Canais",
            data_type: "number",
            unit: "ch",
          },
          { field_key: "efeitos", label: "Efeitos", data_type: "boolean" },
        ],
      },
      {
        name: "Acessórios",
        sort_order: 8,
        attrs: [],
      },
      {
        parent: "Acessórios",
        name: "Pedestais e suportes",
        sort_order: 1,
        attrs: [
          {
            field_key: "carga_maxima_kg",
            label: "Carga Máxima",
            data_type: "number",
            unit: "kg",
          },
        ],
      },
      {
        parent: "Acessórios",
        name: "Cases e racks",
        sort_order: 2,
        attrs: [
          {
            field_key: "unidades_rack_u",
            label: "Unidades de Rack",
            data_type: "number",
            unit: "U",
          },
        ],
      },
      {
        parent: "Acessórios",
        name: "Fitas e consumíveis",
        sort_order: 3,
        attrs: [],
      },
    ];

    const rootIds = new Map<string, string>();
    let categories_created = 0;
    let attributes_created = 0;

    for (const spec of specs) {
      const parentId = spec.parent ? (rootIds.get(spec.parent) ?? null) : null;
      const norm = normalizeText(spec.name);
      let cat = this.categories.find(
        (c) =>
          c.tenant_id === tid &&
          !c.deleted_at &&
          c.parent_id === parentId &&
          c.name_normalized === norm,
      );
      if (!cat) {
        cat = await this.createCategory({
          parent_id: parentId,
          name: spec.name,
          description: null,
          sort_order: spec.sort_order,
          active: true,
        });
        categories_created += 1;
      }
      if (!spec.parent) {
        rootIds.set(spec.name, cat.id);
      }

      const existingKeys = new Set(
        this.definitions
          .filter(
            (d) =>
              d.tenant_id === tid && d.category_id === cat.id && !d.deleted_at,
          )
          .map((d) => d.field_key),
      );
      for (let i = 0; i < spec.attrs.length; i++) {
        const a = spec.attrs[i];
        if (!existingKeys.has(a.field_key)) {
          await this.createAttributeDefinition({
            category_id: cat.id,
            field_key: a.field_key,
            label: a.label,
            data_type: a.data_type,
            unit: a.unit ?? null,
            choices: a.choices ?? [],
            is_required: false,
            sort_order: i + 1,
            active: true,
          });
          attributes_created += 1;
        }
      }
    }

    return { categories_created, attributes_created };
  }

  async listModelMedia(modelId: string): Promise<EquipmentModelMediaView[]> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    return this.mediaItems
      .filter((m) => m.tenant_id === tid && m.model_id === modelId)
      .sort(
        (a, b) =>
          Number(b.is_primary) - Number(a.is_primary) ||
          a.sort_order - b.sort_order ||
          a.created_at.localeCompare(b.created_at),
      );
  }

  async getMediaBlobBytes(blobId: string): Promise<Uint8Array> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const bytes = this.blobStore.get(blobId);
    if (!bytes) {
      throw new Error("Blob de mídia não encontrado.");
    }
    return bytes;
  }

  async attachModelPhoto(
    input: AttachModelPhotoInput,
  ): Promise<EquipmentModelMediaView> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const model = this.models.find(
      (m) => m.tenant_id === tid && m.id === input.model_id && !m.deleted_at,
    );
    if (!model) throw new Error("Modelo de equipamento não encontrado.");

    const existingPhotos = this.mediaItems.filter(
      (m) =>
        m.tenant_id === tid &&
        m.model_id === input.model_id &&
        m.kind === "image",
    );
    if (existingPhotos.length >= 12) {
      throw new Error(
        "Limite máximo de 12 fotos por modelo de equipamento atingido.",
      );
    }

    const title = input.title.trim();
    if (!title || title.length > 150) {
      throw new Error("O título da mídia deve ter entre 1 e 150 caracteres.");
    }

    const blobId = makeId("blob");
    const thumbBlobId = makeId("tblob");
    this.blobStore.set(blobId, new Uint8Array(input.display_bytes));
    this.blobStore.set(thumbBlobId, new Uint8Array(input.thumb_bytes));

    const isPrimary = !existingPhotos.some((p) => p.is_primary);
    const maxSort = existingPhotos.reduce(
      (max, p) => Math.max(max, p.sort_order),
      -1,
    );
    const ts = nowIso();

    const item: EquipmentModelMediaView = {
      id: makeId("emm"),
      tenant_id: tid,
      model_id: input.model_id,
      asset_id: makeId("asset"),
      kind: "image",
      blob_id: blobId,
      blob_sha256: blobId.padEnd(64, "0").slice(0, 64),
      mime_type: input.display_mime_type,
      thumb_blob_id: thumbBlobId,
      thumb_sha256: thumbBlobId.padEnd(64, "0").slice(0, 64),
      thumb_mime_type: input.thumb_mime_type,
      title,
      doc_type: null,
      origin: input.origin,
      credit_note: input.credit_note?.trim() || null,
      language: null,
      version_note: null,
      original_filename: input.original_filename.trim() || "foto.webp",
      width: 800,
      height: 600,
      byte_size: input.display_bytes.byteLength,
      is_primary: isPrimary,
      sort_order: maxSort + 1,
      created_at: ts,
      updated_at: ts,
    };

    this.mediaItems.push(item);
    return item;
  }

  async attachModelDocument(
    input: AttachModelDocumentInput,
  ): Promise<EquipmentModelMediaView> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const model = this.models.find(
      (m) => m.tenant_id === tid && m.id === input.model_id && !m.deleted_at,
    );
    if (!model) throw new Error("Modelo de equipamento não encontrado.");

    const existingDocs = this.mediaItems.filter(
      (m) =>
        m.tenant_id === tid &&
        m.model_id === input.model_id &&
        m.kind === "document",
    );
    if (existingDocs.length >= 20) {
      throw new Error(
        "Limite máximo de 20 documentos por modelo de equipamento atingido.",
      );
    }

    const title = input.title.trim();
    if (!title || title.length > 150) {
      throw new Error("O título da mídia deve ter entre 1 e 150 caracteres.");
    }

    const blobId = makeId("blob");
    this.blobStore.set(blobId, new Uint8Array(input.bytes));
    const maxSort = existingDocs.reduce(
      (max, d) => Math.max(max, d.sort_order),
      -1,
    );
    const ts = nowIso();

    const item: EquipmentModelMediaView = {
      id: makeId("emm"),
      tenant_id: tid,
      model_id: input.model_id,
      asset_id: makeId("asset"),
      kind: "document",
      blob_id: blobId,
      blob_sha256: blobId.padEnd(64, "0").slice(0, 64),
      mime_type: "application/pdf",
      thumb_blob_id: null,
      thumb_sha256: null,
      thumb_mime_type: null,
      title,
      doc_type: input.doc_type,
      origin: input.origin,
      credit_note: input.credit_note?.trim() || null,
      language: input.language?.trim() || null,
      version_note: input.version_note?.trim() || null,
      original_filename: input.original_filename.trim() || "documento.pdf",
      width: null,
      height: null,
      byte_size: input.bytes.byteLength,
      is_primary: false,
      sort_order: maxSort + 1,
      created_at: ts,
      updated_at: ts,
    };

    this.mediaItems.push(item);
    return item;
  }

  async updateModelMediaMetadata(
    input: UpdateModelMediaInput,
  ): Promise<EquipmentModelMediaView> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const idx = this.mediaItems.findIndex(
      (m) => m.tenant_id === tid && m.id === input.media_link_id,
    );
    if (idx === -1) throw new Error("Mídia do modelo não encontrada.");
    const existing = this.mediaItems[idx];
    if (existing.updated_at !== input.expected_updated_at) {
      throw new Error(
        "Os metadados desta mídia foram alterados por outra operação. Recarregue antes de salvar.",
      );
    }
    const title = input.title.trim();
    if (!title || title.length > 150) {
      throw new Error("O título da mídia deve ter entre 1 e 150 caracteres.");
    }
    const updated: EquipmentModelMediaView = {
      ...existing,
      title,
      doc_type: existing.kind === "document" ? input.doc_type : null,
      origin: input.origin,
      credit_note: input.credit_note?.trim() || null,
      language:
        existing.kind === "document" ? input.language?.trim() || null : null,
      version_note:
        existing.kind === "document"
          ? input.version_note?.trim() || null
          : null,
      updated_at: nowIso(),
    };
    this.mediaItems[idx] = updated;
    return updated;
  }

  async reorderModelMedia(
    input: ReorderModelMediaInput,
  ): Promise<EquipmentModelMediaView[]> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const ts = nowIso();
    input.ordered_link_ids.forEach((linkId, index) => {
      const idx = this.mediaItems.findIndex(
        (m) =>
          m.tenant_id === tid &&
          m.model_id === input.model_id &&
          m.kind === input.kind &&
          m.id === linkId,
      );
      if (idx !== -1) {
        this.mediaItems[idx] = {
          ...this.mediaItems[idx],
          sort_order: index,
          updated_at: ts,
        };
      }
    });
    return this.listModelMedia(input.model_id);
  }

  async setPrimaryModelPhoto(
    mediaLinkId: string,
  ): Promise<EquipmentModelMediaView> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const target = this.mediaItems.find(
      (m) => m.tenant_id === tid && m.id === mediaLinkId && m.kind === "image",
    );
    if (!target) throw new Error("Foto do modelo não encontrada.");
    const ts = nowIso();
    this.mediaItems = this.mediaItems.map((m) => {
      if (
        m.tenant_id === tid &&
        m.model_id === target.model_id &&
        m.kind === "image"
      ) {
        return {
          ...m,
          is_primary: m.id === mediaLinkId,
          updated_at: ts,
        };
      }
      return m;
    });
    return this.mediaItems.find((m) => m.id === mediaLinkId)!;
  }

  async removeModelMedia(mediaLinkId: string): Promise<void> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    const tid = this.tenantId();
    const target = this.mediaItems.find(
      (m) => m.tenant_id === tid && m.id === mediaLinkId,
    );
    if (!target) throw new Error("Vínculo de mídia não encontrado.");

    this.mediaItems = this.mediaItems.filter((m) => m.id !== mediaLinkId);

    if (target.kind === "image" && target.is_primary) {
      const remaining = this.mediaItems
        .filter(
          (m) =>
            m.tenant_id === tid &&
            m.model_id === target.model_id &&
            m.kind === "image",
        )
        .sort((a, b) => a.sort_order - b.sort_order);
      if (remaining.length > 0) {
        const nextId = remaining[0].id;
        this.mediaItems = this.mediaItems.map((m) =>
          m.id === nextId ? { ...m, is_primary: true } : m,
        );
      }
    }
  }

  async collectUnreferencedMedia(): Promise<MediaGarbageCollectionResult> {
    this.assertPermission(PERMISSIONS.CATALOG_MANAGE);
    return {
      deleted_assets: 0,
      deleted_blobs: 0,
      deleted_files: 0,
    };
  }

  async openModelDocumentExternal(mediaLinkId: string): Promise<string> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    const doc = this.mediaItems.find(
      (m) =>
        m.tenant_id === tid && m.id === mediaLinkId && m.kind === "document",
    );
    if (!doc) throw new Error("Documento não encontrado.");
    return `/temp-media/${doc.original_filename}`;
  }

  async saveModelDocumentAs(mediaLinkId: string): Promise<string | null> {
    this.assertPermission(PERMISSIONS.CATALOG_READ);
    const tid = this.tenantId();
    const doc = this.mediaItems.find(
      (m) =>
        m.tenant_id === tid && m.id === mediaLinkId && m.kind === "document",
    );
    if (!doc) throw new Error("Documento não encontrado.");
    return `/downloads/${doc.original_filename}`;
  }
}
