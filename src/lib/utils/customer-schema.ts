import { z } from "zod";
import {
  BRAZILIAN_UFS,
  digitsOnly,
  validateCnpj,
  validateCpf,
  validateForeignDocument,
} from "./document";

export const contactSchema = z
  .object({
    id: z.string().optional().nullable(),
    name: z
      .string()
      .trim()
      .min(1, "O nome do contato é obrigatório.")
      .max(100, "Nome do contato excede o limite máximo de 100 caracteres."),
    email: z
      .string()
      .trim()
      .email("Informe um e-mail válido para o contato.")
      .or(z.literal(""))
      .optional()
      .nullable(),
    phone: z
      .string()
      .trim()
      .refine(
        (val) => {
          if (!val) return true;
          const d = digitsOnly(val);
          return d.length >= 10 && d.length <= 11;
        },
        { message: "Telefone deve ter DDD + 8 ou 9 dígitos." },
      )
      .optional()
      .nullable(),
    role: z
      .string()
      .trim()
      .max(60, "Função do contato excede o limite máximo de 60 caracteres.")
      .optional()
      .nullable(),
    is_primary: z.boolean().default(false),
  })
  .refine(
    (data) => {
      const hasEmail = Boolean(data.email && data.email.trim().length > 0);
      const hasPhone = Boolean(data.phone && digitsOnly(data.phone).length > 0);
      return hasEmail || hasPhone;
    },
    {
      message:
        "Cada contato deve possuir pelo menos um telefone ou e-mail preenchido.",
      path: ["email"],
    },
  );

export const addressSchema = z.object({
  id: z.string().optional().nullable(),
  label: z
    .string()
    .trim()
    .max(40, "Rótulo excede o limite máximo de 40 caracteres.")
    .default("Principal"),
  street: z
    .string()
    .trim()
    .min(1, "Informe o logradouro.")
    .max(150, "Logradouro excede 150 caracteres."),
  number: z
    .string()
    .trim()
    .min(1, "Informe o número (ou S/N).")
    .max(20, "Número excede 20 caracteres."),
  complement: z
    .string()
    .trim()
    .max(60, "Complemento excede 60 caracteres.")
    .optional()
    .nullable(),
  neighborhood: z
    .string()
    .trim()
    .min(1, "Informe o bairro.")
    .max(80, "Bairro excede 80 caracteres."),
  city: z
    .string()
    .trim()
    .min(1, "Informe a cidade.")
    .max(80, "Cidade excede 80 caracteres."),
  state: z
    .string()
    .trim()
    .toUpperCase()
    .refine((uf) => (BRAZILIAN_UFS as readonly string[]).includes(uf), {
      message: "Selecione uma UF válida.",
    }),
  postal_code: z
    .string()
    .trim()
    .refine((cep) => digitsOnly(cep).length === 8, {
      message: "O CEP deve conter 8 dígitos.",
    }),
  country: z.string().trim().min(2).max(2).default("BR"),
  is_primary: z.boolean().default(false),
});

export const customerFormSchema = z
  .object({
    person_type: z.enum(["individual", "legal"]),
    name: z
      .string()
      .trim()
      .min(1, "O nome / razão social é obrigatório.")
      .max(150, "Nome ou Razão Social excede 150 caracteres."),
    trade_name: z
      .string()
      .trim()
      .max(150, "Nome fantasia excede 150 caracteres.")
      .optional()
      .nullable(),
    document_type: z.enum(["cpf", "cnpj", "foreign", "none"]),
    document_number: z.string().trim().optional().nullable(),
    state_registration: z
      .string()
      .trim()
      .max(30, "Inscrição estadual excede 30 caracteres.")
      .optional()
      .nullable(),
    municipal_registration: z
      .string()
      .trim()
      .max(30, "Inscrição municipal excede 30 caracteres.")
      .optional()
      .nullable(),
    notes: z
      .string()
      .trim()
      .max(2000, "Notas excedem 2000 caracteres.")
      .optional()
      .nullable(),
    contacts: z
      .array(contactSchema)
      .min(1, "Cliente deve possuir pelo menos um contato.")
      .max(20, "Número máximo de 20 contatos excedido."),
    addresses: z
      .array(addressSchema)
      .max(20, "Número máximo de 20 endereços excedido.")
      .default([]),
  })
  .superRefine((data, ctx) => {
    const doc = (data.document_number ?? "").trim();

    if (data.document_type === "cpf") {
      if (data.person_type !== "individual") {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["document_type"],
          message: "Pessoa Jurídica não pode utilizar CPF.",
        });
      } else if (!validateCpf(doc)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["document_number"],
          message: "CPF inválido. Verifique os 11 dígitos informados.",
        });
      }
    } else if (data.document_type === "cnpj") {
      if (data.person_type !== "legal") {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["document_type"],
          message: "Pessoa Física não pode utilizar CNPJ.",
        });
      } else if (!validateCnpj(doc)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["document_number"],
          message:
            "CNPJ inválido. Verifique os 14 caracteres (numérico ou alfanumérico conforme Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024).",
        });
      }
    } else if (data.document_type === "foreign") {
      if (!validateForeignDocument(doc)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["document_number"],
          message:
            "Documento estrangeiro deve conter entre 5 e 30 caracteres alfanuméricos.",
        });
      }
    }

    const primaryContacts = data.contacts.filter((c) => c.is_primary).length;
    if (primaryContacts > 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["contacts"],
        message: "Apenas um contato pode ser marcado como principal.",
      });
    }

    const primaryAddresses = data.addresses.filter((a) => a.is_primary).length;
    if (primaryAddresses > 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["addresses"],
        message: "Apenas um endereço pode ser marcado como principal.",
      });
    }
  });

export type CustomerFormValues = z.infer<typeof customerFormSchema>;
