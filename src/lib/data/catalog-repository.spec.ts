import { describe, it, expect, beforeEach } from "vitest";
import {
  appSession,
  canAccessRoute,
  MockCatalogRepository,
  PERMISSIONS,
  type SessionInfo,
} from "./index";

function makeSession(permissions: string[]): SessionInfo {
  const now = new Date().toISOString();
  return {
    user_id: "user-1",
    tenant_id: "tenant-1",
    name: "Teste",
    email: "teste@eventrent.com",
    role_id: "role-1",
    role_name: "Perfil",
    role_slug: "custom",
    permissions,
    authenticated_at: now,
    last_activity_at: now,
    inactivity_timeout_minutes: 30,
  };
}

describe("CatalogRepository & Rotas (Fase 2A)", () => {
  let repo: MockCatalogRepository;

  beforeEach(() => {
    repo = new MockCatalogRepository();
    appSession.session = makeSession([
      PERMISSIONS.CATALOG_READ,
      PERMISSIONS.CATALOG_MANAGE,
    ]);
  });

  it("carrega categorias sugeridas de forma idempotente e resolve herança de atributos", async () => {
    const first = await repo.loadSuggestedCategories();
    expect(first.categories_created).toBeGreaterThan(10);
    expect(first.attributes_created).toBeGreaterThan(10);

    const second = await repo.loadSuggestedCategories();
    expect(second.categories_created).toBe(0);
    expect(second.attributes_created).toBe(0);

    const tree = await repo.listCategoriesTree();
    const mesas = tree.find((i) => i.category.name === "Mesas de som");
    expect(mesas).toBeDefined();

    const attrs = await repo.getCategoryEffectiveAttributes(mesas!.category.id);
    const keys = attrs.map((a) => a.definition.field_key);
    expect(keys).toContain("impedancia_ohms");
    expect(keys).toContain("canais");
    expect(
      attrs.find((a) => a.definition.field_key === "impedancia_ohms")
        ?.is_inherited,
    ).toBe(true);
  });

  it("valida profundidade máxima de 4 níveis, EAN e NCM no repositório", async () => {
    const n1 = await repo.createCategory({
      parent_id: null,
      name: "N1",
      description: null,
      sort_order: 1,
      active: true,
    });
    const n2 = await repo.createCategory({
      parent_id: n1.id,
      name: "N2",
      description: null,
      sort_order: 1,
      active: true,
    });
    const n3 = await repo.createCategory({
      parent_id: n2.id,
      name: "N3",
      description: null,
      sort_order: 1,
      active: true,
    });
    const n4 = await repo.createCategory({
      parent_id: n3.id,
      name: "N4",
      description: null,
      sort_order: 1,
      active: true,
    });

    await expect(
      repo.createCategory({
        parent_id: n4.id,
        name: "N5",
        description: null,
        sort_order: 1,
        active: true,
      }),
    ).rejects.toThrow(/4 níveis/);

    await expect(
      repo.createEquipmentModel({
        category_id: n1.id,
        brand_id: null,
        name: "Teste EAN",
        model_code: null,
        sku: null,
        ean: "7891000315508",
        ncm: null,
        description: null,
        control_mode: "serialized",
        weight_kg: null,
        length_cm: null,
        width_cm: null,
        height_cm: null,
        power_watts: null,
        voltage: null,
        amperage: null,
        requires_case: false,
        units_per_case: null,
        daily_rate_cents: 10000,
        replacement_value_cents: null,
        cost_price_cents: null,
        active: true,
        attribute_values: [],
      }),
    ).rejects.toThrow(/dígito verificador/);
  });

  it("aplica canAccessRoute corretamente para /catalog, /catalog/new e /catalog/categories", () => {
    const warehouse = makeSession([
      PERMISSIONS.CATALOG_READ,
      PERMISSIONS.CATALOG_MANAGE,
    ]);
    const tech = makeSession([PERMISSIONS.CATALOG_READ]);
    const finance = makeSession([PERMISSIONS.SUPPLIER_MANAGE]);

    expect(canAccessRoute(warehouse, "/catalog")).toBe(true);
    expect(canAccessRoute(warehouse, "/catalog/new")).toBe(true);
    expect(canAccessRoute(warehouse, "/catalog/categories")).toBe(true);

    expect(canAccessRoute(tech, "/catalog")).toBe(true);
    expect(canAccessRoute(tech, "/catalog/mod-1")).toBe(true);
    expect(canAccessRoute(tech, "/catalog/categories")).toBe(true);
    expect(canAccessRoute(tech, "/catalog/new")).toBe(false);

    expect(canAccessRoute(finance, "/catalog")).toBe(false);
    expect(canAccessRoute(finance, "/catalog/new")).toBe(false);
  });
});
