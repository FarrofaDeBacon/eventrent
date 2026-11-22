import { z } from "zod";

export const AllocationSettingsSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  internal_transfer_minutes: z.number(),
  external_transfer_minutes: z.number(),
  changeover_minutes: z.number(),
  updated_at: z.string(),
  updated_by: z.string().nullable().optional(),
});
