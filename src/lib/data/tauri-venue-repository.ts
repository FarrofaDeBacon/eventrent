import { invoke } from "@tauri-apps/api/core";
import type {
  CreateVenueInput,
  UpdateVenueInput,
  VenueFilter,
  VenuePage,
  VenueWithDetails,
} from "./types";
import type { VenueRepository } from "./venue-repository";

export class TauriVenueRepository implements VenueRepository {
  async list(filter: VenueFilter = {}): Promise<VenuePage> {
    return invoke<VenuePage>("list_venues", { filter });
  }

  async getById(id: string): Promise<VenueWithDetails> {
    return invoke<VenueWithDetails>("get_venue", { id });
  }

  async create(input: CreateVenueInput): Promise<VenueWithDetails> {
    return invoke<VenueWithDetails>("create_venue", { input });
  }

  async update(input: UpdateVenueInput): Promise<VenueWithDetails> {
    return invoke<VenueWithDetails>("update_venue", { input });
  }

  async setActive(id: string, active: boolean): Promise<VenueWithDetails> {
    return invoke<VenueWithDetails>("set_venue_active", { id, active });
  }

  async delete(id: string): Promise<void> {
    return invoke<void>("delete_venue", { id });
  }

  async checkDuplicateNameCity(
    name: string,
    city?: string | null,
    excludeId?: string | null,
  ): Promise<boolean> {
    return invoke<boolean>("check_venue_duplicate_name_city", {
      name,
      city: city ?? null,
      excludeId: excludeId ?? null,
    });
  }
}
