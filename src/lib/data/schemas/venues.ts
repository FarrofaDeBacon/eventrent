import { z } from "zod";

export const VenueListItemSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  city: z.string().nullable().optional(),
  state: z.string().nullable().optional(),
  primary_contact_name: z.string().nullable().optional(),
  primary_contact_phone: z.string().nullable().optional(),
  spaces_count: z.number().optional(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const VenueDetailSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  name_sort: z.string().optional(),
  street: z.string().nullable().optional(),
  number: z.string().nullable().optional(),
  complement: z.string().nullable().optional(),
  neighborhood: z.string().nullable().optional(),
  city: z.string().nullable().optional(),
  state: z.string().nullable().optional(),
  postal_code: z.string().nullable().optional(),
  country: z.string().nullable().optional(),
  internal_transfer_minutes: z.number().nullable().optional(),
  access_conditions: z.string().nullable().optional(),
  dock_levelers: z.number().nullable().optional(),
  door_height_meters: z.number().nullable().optional(),
  door_width_meters: z.number().nullable().optional(),
  generator_space: z.boolean().nullable().optional(),
  main_grounding: z.boolean().nullable().optional(),
  power_distribution_distance_meters: z.number().nullable().optional(),
  power_voltage: z.string().nullable().optional(),
  water_point: z.boolean().nullable().optional(),
  notes: z.string().nullable().optional(),
  search_text: z.string().optional(),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
  updated_by: z.string().nullable().optional(),
});

export const VenueSpaceSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  venue_id: z.string(),
  name: z.string(),
  capacity: z.number().nullable().optional(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
});

export const VenueContactSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  venue_id: z.string(),
  name: z.string(),
  role: z.string().nullable().optional(),
  email: z.string().nullable().optional(),
  phone: z.string().nullable().optional(),
  is_primary: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
});

export const VenueWithDetailsSchema = z.object({
  venue: VenueDetailSchema,
  spaces: z.array(VenueSpaceSchema),
  contacts: z.array(VenueContactSchema),
});

export const VenuePageSchema = z.object({
  items: z.array(VenueListItemSchema),
  next_cursor: z.string().nullable().optional(),
  total: z.number(),
  page: z.number().optional(),
  page_size: z.number().optional(),
  total_pages: z.number().optional(),
});
