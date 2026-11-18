import type {
  CreateVenueInput,
  UpdateVenueInput,
  VenueContact,
  VenueFilter,
  VenueListItem,
  VenuePage,
  VenueSpace,
  VenueWithDetails,
} from "./types";
import type { VenueRepository } from "./venue-repository";

const STORAGE_KEY = "eventrent:mock:venues:v1";

function normalizeSearch(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function genId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `venue-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export class MockVenueRepository implements VenueRepository {
  private loadAll(): VenueWithDetails[] {
    if (typeof window === "undefined") return [];
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    try {
      return JSON.parse(raw) as VenueWithDetails[];
    } catch {
      return [];
    }
  }

  private saveAll(items: VenueWithDetails[]): void {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
    }
  }

  async list(filter: VenueFilter = {}): Promise<VenuePage> {
    const all = this.loadAll().filter((v) => !v.deleted_at);
    const queryNorm = filter.query ? normalizeSearch(filter.query) : "";

    const filtered = all
      .filter((v) => {
        if (filter.active_only === true && !v.active) return false;
        if (filter.active_only === false && v.active) return false;
        if (!queryNorm) return true;
        return v.search_text.includes(queryNorm);
      })
      .sort((a, b) => a.name_sort.localeCompare(b.name_sort));

    const limit = Math.min(Math.max(filter.limit ?? 50, 1), 100);
    const items: VenueListItem[] = filtered.slice(0, limit).map((v) => ({
      ...v,
      spaces_count: v.spaces.filter((s) => !s.deleted_at).length,
    }));

    return {
      items,
      next_cursor: null,
      has_more: filtered.length > limit,
      total_count: filtered.length,
    };
  }

  async getById(id: string): Promise<VenueWithDetails> {
    const found = this.loadAll().find((v) => v.id === id && !v.deleted_at);
    if (!found) {
      throw new Error("Local de evento não encontrado.");
    }
    return found;
  }

  async create(input: CreateVenueInput): Promise<VenueWithDetails> {
    const now = new Date().toISOString();
    const venueId = genId();
    const nameTrimmed = input.name.trim();
    if (!nameTrimmed) {
      throw new Error("Nome do local é obrigatório.");
    }

    const seenSpaces = new Set<string>();
    const spaces: VenueSpace[] = input.spaces.map((sp, idx) => {
      const spName = sp.name.trim();
      if (!spName) {
        throw new Error("Nome do espaço é obrigatório.");
      }
      const spNorm = normalizeSearch(spName);
      if (seenSpaces.has(spNorm)) {
        throw new Error(
          `Já existe um espaço com o nome '${spName}' neste local.`,
        );
      }
      seenSpaces.add(spNorm);
      if (
        sp.internal_transfer_minutes < 0 ||
        sp.internal_transfer_minutes > 240
      ) {
        throw new Error(
          "Deslocamento interno deve estar entre 0 e 240 minutos.",
        );
      }
      return {
        id: genId(),
        tenant_id: "mock-tenant",
        venue_id: venueId,
        name: spName,
        name_normalized: spNorm,
        capacity_people: sp.capacity_people ?? null,
        area_sqm: sp.area_sqm ?? null,
        ceiling_height_m: sp.ceiling_height_m ?? null,
        power_available: sp.power_available ?? null,
        loading_access: sp.loading_access ?? null,
        rigging_points: sp.rigging_points ?? null,
        acoustics_notes: sp.acoustics_notes ?? null,
        internal_transfer_minutes: sp.internal_transfer_minutes ?? 0,
        notes: sp.notes ?? null,
        active: sp.active ?? true,
        sort_order: sp.sort_order ?? idx,
        created_at: now,
        updated_at: now,
        deleted_at: null,
        created_by: "mock-user",
      };
    });

    const contacts: VenueContact[] = input.contacts.map((ct, idx) => ({
      id: genId(),
      tenant_id: "mock-tenant",
      venue_id: venueId,
      name: ct.name.trim(),
      email: ct.email?.trim() || null,
      phone: ct.phone?.trim() || null,
      role: ct.role?.trim() || null,
      is_primary:
        ct.is_primary ||
        (!input.contacts.some((c) => c.is_primary) && idx === 0),
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "mock-user",
    }));

    const searchParts = [
      normalizeSearch(nameTrimmed),
      input.city ? normalizeSearch(input.city) : "",
      input.state ? normalizeSearch(input.state) : "",
      ...spaces.map((s) => s.name_normalized),
      ...contacts.map((c) => normalizeSearch(c.name)),
      ...contacts.map((c) => (c.phone ?? "").replace(/\D/g, "")),
    ].filter(Boolean);

    const created: VenueWithDetails = {
      id: venueId,
      tenant_id: "mock-tenant",
      name: nameTrimmed,
      street: input.street ?? null,
      number: input.number ?? null,
      complement: input.complement ?? null,
      neighborhood: input.neighborhood ?? null,
      city: input.city ?? null,
      state: input.state ?? null,
      postal_code: input.postal_code ?? null,
      country: input.country || "BR",
      parking_notes: input.parking_notes ?? null,
      loading_dock_notes: input.loading_dock_notes ?? null,
      access_hours: input.access_hours ?? null,
      notes: input.notes ?? null,
      active: true,
      search_text: searchParts.join(" "),
      name_sort: normalizeSearch(nameTrimmed),
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "mock-user",
      spaces,
      contacts,
    };

    const all = this.loadAll();
    all.push(created);
    this.saveAll(all);
    return created;
  }

  async update(input: UpdateVenueInput): Promise<VenueWithDetails> {
    const all = this.loadAll();
    const idx = all.findIndex((v) => v.id === input.id && !v.deleted_at);
    if (idx === -1) {
      throw new Error("Local de evento não encontrado.");
    }
    const existing = all[idx];
    if (existing.updated_at !== input.expected_updated_at) {
      throw new Error(
        "O cadastro do local foi alterado por outra pessoa. Recarregue os dados antes de salvar.",
      );
    }

    const now = new Date().toISOString();
    const nameTrimmed = input.name.trim();
    const existingSpaceMap = new Map(existing.spaces.map((s) => [s.id, s]));
    const seenSpaces = new Set<string>();

    const spaces: VenueSpace[] = input.spaces.map((sp, spIdx) => {
      const spName = sp.name.trim();
      const spNorm = normalizeSearch(spName);
      if (seenSpaces.has(spNorm)) {
        throw new Error(
          `Já existe um espaço com o nome '${spName}' neste local.`,
        );
      }
      seenSpaces.add(spNorm);
      const prev = sp.id ? existingSpaceMap.get(sp.id) : undefined;
      return {
        id: prev ? prev.id : genId(),
        tenant_id: existing.tenant_id,
        venue_id: existing.id,
        name: spName,
        name_normalized: spNorm,
        capacity_people: sp.capacity_people ?? null,
        area_sqm: sp.area_sqm ?? null,
        ceiling_height_m: sp.ceiling_height_m ?? null,
        power_available: sp.power_available ?? null,
        loading_access: sp.loading_access ?? null,
        rigging_points: sp.rigging_points ?? null,
        acoustics_notes: sp.acoustics_notes ?? null,
        internal_transfer_minutes: sp.internal_transfer_minutes ?? 0,
        notes: sp.notes ?? null,
        active: sp.active ?? true,
        sort_order: sp.sort_order ?? spIdx,
        created_at: prev ? prev.created_at : now,
        updated_at: now,
        deleted_at: null,
        created_by: prev ? prev.created_by : "mock-user",
      };
    });

    const existingContactMap = new Map(existing.contacts.map((c) => [c.id, c]));
    const contacts: VenueContact[] = input.contacts.map((ct, ctIdx) => {
      const prev = ct.id ? existingContactMap.get(ct.id) : undefined;
      return {
        id: prev ? prev.id : genId(),
        tenant_id: existing.tenant_id,
        venue_id: existing.id,
        name: ct.name.trim(),
        email: ct.email?.trim() || null,
        phone: ct.phone?.trim() || null,
        role: ct.role?.trim() || null,
        is_primary:
          ct.is_primary ||
          (!input.contacts.some((c) => c.is_primary) && ctIdx === 0),
        created_at: prev ? prev.created_at : now,
        updated_at: now,
        deleted_at: null,
        created_by: prev ? prev.created_by : "mock-user",
      };
    });

    const searchParts = [
      normalizeSearch(nameTrimmed),
      input.city ? normalizeSearch(input.city) : "",
      input.state ? normalizeSearch(input.state) : "",
      ...spaces.map((s) => s.name_normalized),
      ...contacts.map((c) => normalizeSearch(c.name)),
      ...contacts.map((c) => (c.phone ?? "").replace(/\D/g, "")),
    ].filter(Boolean);

    const updated: VenueWithDetails = {
      ...existing,
      name: nameTrimmed,
      street: input.street ?? null,
      number: input.number ?? null,
      complement: input.complement ?? null,
      neighborhood: input.neighborhood ?? null,
      city: input.city ?? null,
      state: input.state ?? null,
      postal_code: input.postal_code ?? null,
      country: input.country || "BR",
      parking_notes: input.parking_notes ?? null,
      loading_dock_notes: input.loading_dock_notes ?? null,
      access_hours: input.access_hours ?? null,
      notes: input.notes ?? null,
      search_text: searchParts.join(" "),
      name_sort: normalizeSearch(nameTrimmed),
      updated_at: now,
      spaces,
      contacts,
    };

    all[idx] = updated;
    this.saveAll(all);
    return updated;
  }

  async setActive(id: string, active: boolean): Promise<VenueWithDetails> {
    const all = this.loadAll();
    const idx = all.findIndex((v) => v.id === id && !v.deleted_at);
    if (idx === -1) throw new Error("Local de evento não encontrado.");
    all[idx] = {
      ...all[idx],
      active,
      updated_at: new Date().toISOString(),
    };
    this.saveAll(all);
    return all[idx];
  }

  async delete(id: string): Promise<void> {
    const all = this.loadAll();
    const idx = all.findIndex((v) => v.id === id && !v.deleted_at);
    if (idx === -1) throw new Error("Local de evento não encontrado.");
    const now = new Date().toISOString();
    all[idx] = {
      ...all[idx],
      deleted_at: now,
      updated_at: now,
    };
    this.saveAll(all);
  }

  async checkDuplicateNameCity(
    name: string,
    city?: string | null,
    excludeId?: string | null,
  ): Promise<boolean> {
    const nameNorm = normalizeSearch(name);
    if (!nameNorm) return false;
    const cityNorm = city ? normalizeSearch(city) : "";
    return this.loadAll().some((v) => {
      if (v.deleted_at) return false;
      if (excludeId && v.id === excludeId) return false;
      const vCityNorm = v.city ? normalizeSearch(v.city) : "";
      return v.name_sort === nameNorm && vCityNorm === cityNorm;
    });
  }
}
