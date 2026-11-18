import { describe, it, expect, beforeEach } from "vitest";
import { canAccessRoute } from "./index";
import { MockVenueRepository } from "./mock-venue-repository";
import { MockSettingsRepository } from "./mock-settings-repository";
import { PERMISSIONS, ROLE_SLUGS, type SessionInfo } from "./types";

function makeSession(
  role_slug: SessionInfo["role_slug"],
  permissions: string[],
): SessionInfo {
  return {
    user_id: "u-1",
    tenant_id: "t-1",
    name: "Usuário Teste",
    email: "teste@eventrent.com",
    role_id: `role-${role_slug}`,
    role_name: role_slug,
    role_slug,
    permissions,
    authenticated_at: new Date().toISOString(),
    last_activity_at: new Date().toISOString(),
    inactivity_timeout_minutes: 30,
  };
}

describe("Venues & Settings RBAC + MockRepositories (Fase 1D)", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("aplica canAccessRoute corretamente para /venues e /settings/operation", () => {
    const owner = makeSession(ROLE_SLUGS.OWNER, Object.values(PERMISSIONS));
    const tech = makeSession(ROLE_SLUGS.TECHNICIAN, [PERMISSIONS.VENUE_READ]);
    const commercial = makeSession(ROLE_SLUGS.COMMERCIAL, [
      PERMISSIONS.VENUE_READ,
      PERMISSIONS.VENUE_MANAGE,
    ]);

    expect(canAccessRoute(owner, "/venues")).toBe(true);
    expect(canAccessRoute(owner, "/venues/new")).toBe(true);
    expect(canAccessRoute(owner, "/settings/operation")).toBe(true);

    expect(canAccessRoute(commercial, "/venues")).toBe(true);
    expect(canAccessRoute(commercial, "/venues/new")).toBe(true);
    expect(canAccessRoute(commercial, "/settings/operation")).toBe(false);

    expect(canAccessRoute(tech, "/venues")).toBe(true);
    expect(canAccessRoute(tech, "/venues/v-1")).toBe(true);
    expect(canAccessRoute(tech, "/venues/new")).toBe(false);
    expect(canAccessRoute(tech, "/settings/operation")).toBe(false);
  });

  it("MockVenueRepository cria, lista, detecta duplicado, preserva IDs de espaços e valida concorrência", async () => {
    const repo = new MockVenueRepository();
    const created = await repo.create({
      name: "Riocentro",
      city: "Rio de Janeiro",
      state: "RJ",
      country: "BR",
      internal_transfer_minutes: null,
      spaces: [{ name: "Pavilhão 1" }, { name: "Pavilhão 2" }],
      contacts: [
        {
          name: "Carlos Doca",
          phone: "+5521999887766",
          is_primary: true,
        },
      ],
    });

    expect(created.spaces).toHaveLength(2);
    expect(
      await repo.checkDuplicateNameCity("riocentro", "rio de janeiro"),
    ).toBe(true);
    expect(
      await repo.checkDuplicateNameCity(
        "riocentro",
        "rio de janeiro",
        created.id,
      ),
    ).toBe(false);

    const list = await repo.list({ query: "pavilhao 2" });
    expect(list.items).toHaveLength(1);
    expect(list.items[0].spaces_count).toBe(2);

    const p1Id = created.spaces[0].id;
    const updated = await repo.update({
      id: created.id,
      expected_updated_at: created.updated_at,
      name: "Riocentro",
      city: "Rio de Janeiro",
      state: "RJ",
      country: "BR",
      spaces: [{ id: p1Id, name: "Pavilhão 1" }],
      contacts: [],
    });
    expect(updated.spaces[0].id).toBe(p1Id);

    await expect(
      repo.update({
        id: created.id,
        expected_updated_at: created.updated_at,
        name: "Riocentro",
        country: "BR",
        spaces: [],
        contacts: [],
      }),
    ).rejects.toThrow(/alterado por outra pessoa/i);
  });

  it("MockSettingsRepository retorna defaults (30, 20, 60, 20) e valida faixa 0..240", async () => {
    const repo = new MockSettingsRepository();
    const initial = await repo.getAllocationSettings();
    expect(initial).toEqual({
      default_setup_minutes: 30,
      default_teardown_minutes: 20,
      default_turnover_buffer_minutes: 60,
      default_internal_transfer_minutes: 20,
    });

    const saved = await repo.updateAllocationSettings({
      default_setup_minutes: 45,
      default_teardown_minutes: 30,
      default_turnover_buffer_minutes: 90,
      default_internal_transfer_minutes: 25,
    });
    expect(saved.default_setup_minutes).toBe(45);
    expect(saved.default_internal_transfer_minutes).toBe(25);

    await expect(
      repo.updateAllocationSettings({
        default_setup_minutes: 241,
        default_teardown_minutes: 20,
        default_turnover_buffer_minutes: 60,
        default_internal_transfer_minutes: 20,
      }),
    ).rejects.toThrow(/0 e 240/);
  });
});
