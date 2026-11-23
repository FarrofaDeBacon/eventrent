import { z } from "zod";

export const PersonTypeSchema = z.enum(["individual", "legal"]);
export const DocumentTypeSchema = z.enum(["cpf", "cnpj", "foreign", "none"]);

export const CustomerListItemSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  person_type: z.string(),
  name: z.string(),
  trade_name: z.string().nullable().optional(),
  document_type: z.string(),
  document_number: z.string().nullable().optional(),
  primary_city: z.string().nullable().optional(),
  primary_email: z.string().nullable().optional(),
  primary_phone: z.string().nullable().optional(),
  primary_state: z.string().nullable().optional(),
  roles: z.array(z.string()),
  active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const CustomerDetailSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  person_type: z.string(),
  name: z.string(),
  trade_name: z.string().nullable().optional(),
  document_type: z.string(),
  document_number: z.string().nullable().optional(),
  state_registration: z.string().nullable().optional(),
  municipal_registration: z.string().nullable().optional(),
  credit_limit_cents: z.number().nullable().optional(),
  notes: z.string().nullable().optional(),
  active: z.boolean(),
  search_text: z.string().optional(),
  name_sort: z.string().optional(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
  created_by: z.string().nullable().optional(),
  updated_by: z.string().nullable().optional(),
});

export const CustomerContactSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  customer_id: z.string(),
  name: z.string(),
  role: z.string().nullable().optional(),
  email: z.string().nullable().optional(),
  phone: z.string().nullable().optional(),
  is_primary: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
});

export const CustomerAddressSchema = z.object({
  id: z.string(),
  tenant_id: z.string(),
  customer_id: z.string(),
  label: z.string(),
  postal_code: z.string(),
  street: z.string(),
  number: z.string(),
  complement: z.string().nullable().optional(),
  neighborhood: z.string(),
  city: z.string(),
  state: z.string(),
  is_primary: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
  deleted_at: z.string().nullable().optional(),
});

export const CustomerWithDetailsSchema = z.object({
  customer: CustomerDetailSchema,
  contacts: z.array(CustomerContactSchema),
  addresses: z.array(CustomerAddressSchema),
  roles: z.array(z.string()),
});

export const CustomerPageSchema = z.object({
  items: z.array(CustomerListItemSchema),
  next_cursor: z.string().nullable().optional(),
  total: z.number(),
  page: z.number().optional(),
  page_size: z.number().optional(),
  total_pages: z.number().optional(),
});
