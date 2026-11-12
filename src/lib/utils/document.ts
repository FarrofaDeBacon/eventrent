export type DocumentKind = "cpf" | "cnpj";

/** 27 Unidades Federativas do Brasil em ordem alfabética de sigla. */
export const BRAZILIAN_UFS = [
  "AC",
  "AL",
  "AP",
  "AM",
  "BA",
  "CE",
  "DF",
  "ES",
  "GO",
  "MA",
  "MT",
  "MS",
  "MG",
  "PA",
  "PB",
  "PR",
  "PE",
  "PI",
  "RJ",
  "RN",
  "RS",
  "RO",
  "RR",
  "SC",
  "SP",
  "SE",
  "TO",
] as const;

/**
 * Remove pontuação, espaços e converte letras para maiúsculas.
 * Mantém dígitos e letras ASCII [0-9A-Z] para suportar CNPJ Alfanumérico (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024).
 */
export function normalizeDocument(input: string): string {
  return input.replace(/[^a-zA-Z0-9]/g, "").toUpperCase();
}

/** Remove qualquer caractere não numérico. */
export function digitsOnly(input: string): string {
  return input.replace(/\D/g, "");
}

/**
 * Remove acentos e converte para minúsculas para busca textual tolerante.
 */
export function normalizeForSearch(input: string): string {
  return input
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim()
    .replace(/\s+/g, " ");
}

/**
 * Valida CPF (11 dígitos) com algoritmo Módulo 11 e rejeição de sequências repetidas.
 */
export function validateCpf(input: string): boolean {
  const clean = digitsOnly(input);
  if (clean.length !== 11) return false;
  if (/^(\d)\1{10}$/.test(clean)) return false;

  const digits = Array.from(clean, Number);

  let sum1 = 0;
  for (let i = 0; i < 9; i++) {
    sum1 += digits[i] * (10 - i);
  }
  const rem1 = sum1 % 11;
  const dv1 = rem1 < 2 ? 0 : 11 - rem1;
  if (digits[9] !== dv1) return false;

  let sum2 = 0;
  for (let i = 0; i < 10; i++) {
    sum2 += digits[i] * (11 - i);
  }
  const rem2 = sum2 % 11;
  const dv2 = rem2 < 2 ? 0 : 11 - rem2;
  return digits[10] === dv2;
}

/**
 * Valida CNPJ (14 caracteres), suportando tanto o padrão numérico quanto o
 * CNPJ ALFANUMÉRICO regulamentado pela Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024.
 *
 * Regras (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024):
 * - Posições 1..12: alfanuméricas [0-9A-Z]
 * - Posições 13..14 (dígitos verificadores): estritamente numéricas [0-9]
 * - Valor do caractere: charCodeAt(0) - 48 ('0'..'9' -> 0..9, 'A'..'Z' -> 17..42)
 */
export function validateCnpj(input: string): boolean {
  const clean = normalizeDocument(input);
  if (clean.length !== 14) return false;
  if (!/^[0-9A-Z]{12}[0-9]{2}$/.test(clean)) return false;
  if (/^([0-9A-Z])\1{13}$/.test(clean)) return false;

  const charValue = (ch: string): number => ch.charCodeAt(0) - 48;

  const weights1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
  let sum1 = 0;
  for (let i = 0; i < 12; i++) {
    sum1 += charValue(clean[i]) * weights1[i];
  }
  const rem1 = sum1 % 11;
  const dv1 = rem1 < 2 ? 0 : 11 - rem1;
  if (Number(clean[12]) !== dv1) return false;

  const weights2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
  let sum2 = 0;
  for (let i = 0; i < 12; i++) {
    sum2 += charValue(clean[i]) * weights2[i];
  }
  sum2 += dv1 * weights2[12];
  const rem2 = sum2 % 11;
  const dv2 = rem2 < 2 ? 0 : 11 - rem2;
  return Number(clean[13]) === dv2;
}

/** Formata CPF completo ou parcial durante digitação (`000.000.000-00`). */
export function maskCpf(input: string): string {
  const d = digitsOnly(input).slice(0, 11);
  if (d.length <= 3) return d;
  if (d.length <= 6) return `${d.slice(0, 3)}.${d.slice(3)}`;
  if (d.length <= 9) return `${d.slice(0, 3)}.${d.slice(3, 6)}.${d.slice(6)}`;
  return `${d.slice(0, 3)}.${d.slice(3, 6)}.${d.slice(6, 9)}-${d.slice(9, 11)}`;
}

/**
 * Formata CNPJ completo ou parcial durante digitação (`XX.XXX.XXX/XXXX-XX`),
 * permitindo caracteres alfanuméricos nas 12 primeiras posições e dígitos nas 2 últimas.
 */
export function maskCnpj(input: string): string {
  const clean = normalizeDocument(input).slice(0, 14);
  // Garante que posições 13 e 14 (índices 12 e 13) aceitem somente números
  let sanitized = "";
  for (let i = 0; i < clean.length; i++) {
    const ch = clean[i];
    if (i < 12) {
      sanitized += ch;
    } else if (/\d/.test(ch)) {
      sanitized += ch;
    }
  }

  if (sanitized.length <= 2) return sanitized;
  if (sanitized.length <= 5)
    return `${sanitized.slice(0, 2)}.${sanitized.slice(2)}`;
  if (sanitized.length <= 8) {
    return `${sanitized.slice(0, 2)}.${sanitized.slice(2, 5)}.${sanitized.slice(5)}`;
  }
  if (sanitized.length <= 12) {
    return `${sanitized.slice(0, 2)}.${sanitized.slice(2, 5)}.${sanitized.slice(5, 8)}/${sanitized.slice(8)}`;
  }
  return `${sanitized.slice(0, 2)}.${sanitized.slice(2, 5)}.${sanitized.slice(5, 8)}/${sanitized.slice(8, 12)}-${sanitized.slice(12, 14)}`;
}

/** Formata documento conforme tipo (`individual` -> CPF, `legal` -> CNPJ) ou tamanho. */
export function formatDocument(
  input: string,
  personType?: "individual" | "legal",
): string {
  const clean = normalizeDocument(input);
  if (personType === "individual" || (!personType && clean.length === 11)) {
    return clean.length === 11 ? maskCpf(clean) : input;
  }
  if (personType === "legal" || (!personType && clean.length === 14)) {
    return clean.length === 14 ? maskCnpj(clean) : input;
  }
  return input;
}

/** Máscara de CEP brasileiro (`00000-000`). */
export function maskCep(input: string): string {
  const d = digitsOnly(input).slice(0, 8);
  if (d.length <= 5) return d;
  return `${d.slice(0, 5)}-${d.slice(5, 8)}`;
}

/** Máscara de telefone brasileiro (`(00) 0000-0000` ou `(00) 00000-0000`). */
export function maskPhone(input: string): string {
  const d = digitsOnly(input).slice(0, 11);
  if (d.length === 0) return "";
  if (d.length <= 2) return `(${d}`;
  if (d.length <= 6) return `(${d.slice(0, 2)}) ${d.slice(2)}`;
  if (d.length <= 10) {
    return `(${d.slice(0, 2)}) ${d.slice(2, 6)}-${d.slice(6)}`;
  }
  return `(${d.slice(0, 2)}) ${d.slice(2, 7)}-${d.slice(7, 11)}`;
}
