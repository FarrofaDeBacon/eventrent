import { z } from "zod";

export const BusinessErrorCodeSchema = z.enum([
  "Validation",
  "Conflict",
  "Unauthorized",
  "Forbidden",
  "NotFound",
  "AccountLocked",
]);
export type BusinessErrorCode = z.infer<typeof BusinessErrorCodeSchema>;

export const BusinessErrorSchema = z.object({
  code: BusinessErrorCodeSchema,
  message: z.string().min(1),
});
export type BusinessError = z.infer<typeof BusinessErrorSchema>;

/**
 * Validador para mensagens de erro retornadas pelo IPC do Tauri.
 * Converte erros em formato texto ou objeto para o erro de negócio canônico.
 */
export function parseBusinessErrorMessage(errorMsg: string): {
  code: BusinessErrorCode;
  message: string;
} {
  const normalized = errorMsg.trim();
  if (normalized.toLowerCase().includes("bloqueada")) {
    return { code: "AccountLocked", message: normalized };
  }
  if (
    normalized.toLowerCase().includes("não autenticado") ||
    normalized.toLowerCase().includes("sessão") ||
    normalized.toLowerCase().includes("unauthorized")
  ) {
    return { code: "Unauthorized", message: normalized };
  }
  if (
    normalized.toLowerCase().includes("permissão") ||
    normalized.toLowerCase().includes("forbidden")
  ) {
    return { code: "Forbidden", message: normalized };
  }
  if (
    normalized.toLowerCase().includes("conflito") ||
    normalized.toLowerCase().includes("conflitante")
  ) {
    return { code: "Conflict", message: normalized };
  }
  if (
    normalized.toLowerCase().includes("não encontrado") ||
    normalized.toLowerCase().includes("not found")
  ) {
    return { code: "NotFound", message: normalized };
  }
  return { code: "Validation", message: normalized };
}
