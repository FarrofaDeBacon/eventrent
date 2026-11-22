import { z } from "zod";

export const RoleWithPermissionsSchema = z.object({
  id: z.string(),
  name: z.string(),
  slug: z.string(),
  description: z.string().nullable(),
  is_system: z.boolean(),
  permissions: z.array(z.string()),
  tenant_id: z.string().optional(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
  auth_epoch: z.number().optional(),
});

export const UserWithRoleSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  name: z.string(),
  email: z.string(),
  role_id: z.string(),
  role_name: z.string(),
  role_slug: z.string(),
  permissions: z.array(z.string()),
  active: z.boolean(),
  last_login_at: z.string().nullable(),
  failed_attempts: z.number(),
  locked_until: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
  auth_epoch: z.number().optional(),
});
