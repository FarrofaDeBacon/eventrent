import type {
  CreateVenueInput,
  UpdateVenueInput,
  VenueFilter,
  VenuePage,
  VenueWithDetails,
} from "./types";

export interface VenueRepository {
  list(filter?: VenueFilter): Promise<VenuePage>;
  getById(id: string): Promise<VenueWithDetails>;
  create(input: CreateVenueInput): Promise<VenueWithDetails>;
  update(input: UpdateVenueInput): Promise<VenueWithDetails>;
  setActive(id: string, active: boolean): Promise<VenueWithDetails>;
  delete(id: string): Promise<void>;
  checkDuplicateNameCity(
    name: string,
    city?: string | null,
    excludeId?: string | null,
  ): Promise<boolean>;
}
