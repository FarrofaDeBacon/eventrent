export interface BatchMatchableModel {
  id: string;
  name: string;
  sku: string | null;
  model_code: string | null;
  ean: string | null;
  brand_name?: string | null;
  category_name?: string | null;
}

export type BatchMatchStatus = "matched" | "ambiguous" | "unmatched";
export type BatchMatchField = "sku" | "model_code" | "ean" | null;

export interface BatchPhotoMatchResult<
  T extends BatchMatchableModel = BatchMatchableModel,
> {
  filename: string;
  normalizedStem: string;
  status: BatchMatchStatus;
  matchedBy: BatchMatchField;
  matchedModel: T | null;
  candidates: T[];
}

/**
 * Remove a extensão final do nome do arquivo.
 */
export function extractFilenameStem(filename: string): string {
  const base = filename.split(/[/\\]/).pop() ?? filename;
  const lastDot = base.lastIndexOf(".");
  if (lastDot <= 0) {
    return base.trim();
  }
  return base.slice(0, lastDot).trim();
}

/**
 * Normaliza chave para comparação (sem acentos, minúscula, espaços colapsados).
 */
export function normalizeBatchMatchKey(raw: string): string {
  return raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim()
    .replace(/\s+/g, " ");
}

/**
 * Casa o nome do arquivo (sem extensão e normalizado), estritamente nesta ordem:
 * 1. SKU (`sku`)
 * 2. Código do modelo (`model_code`)
 * 3. EAN (`ean`)
 */
export function matchFilenameToCatalogModels<T extends BatchMatchableModel>(
  filename: string,
  models: T[],
): BatchPhotoMatchResult<T> {
  const stem = extractFilenameStem(filename);
  const normalizedStem = normalizeBatchMatchKey(stem);

  if (!normalizedStem) {
    return {
      filename,
      normalizedStem: "",
      status: "unmatched",
      matchedBy: null,
      matchedModel: null,
      candidates: [],
    };
  }

  // 1. Tenta casar por SKU
  const skuMatches = models.filter(
    (m) => m.sku && normalizeBatchMatchKey(m.sku) === normalizedStem,
  );
  if (skuMatches.length === 1) {
    return {
      filename,
      normalizedStem,
      status: "matched",
      matchedBy: "sku",
      matchedModel: skuMatches[0],
      candidates: skuMatches,
    };
  }
  if (skuMatches.length > 1) {
    return {
      filename,
      normalizedStem,
      status: "ambiguous",
      matchedBy: "sku",
      matchedModel: null,
      candidates: skuMatches,
    };
  }

  // 2. Tenta casar por Código do Modelo (model_code)
  const codeMatches = models.filter(
    (m) =>
      m.model_code && normalizeBatchMatchKey(m.model_code) === normalizedStem,
  );
  if (codeMatches.length === 1) {
    return {
      filename,
      normalizedStem,
      status: "matched",
      matchedBy: "model_code",
      matchedModel: codeMatches[0],
      candidates: codeMatches,
    };
  }
  if (codeMatches.length > 1) {
    return {
      filename,
      normalizedStem,
      status: "ambiguous",
      matchedBy: "model_code",
      matchedModel: null,
      candidates: codeMatches,
    };
  }

  // 3. Tenta casar por EAN
  const eanMatches = models.filter(
    (m) => m.ean && normalizeBatchMatchKey(m.ean) === normalizedStem,
  );
  if (eanMatches.length === 1) {
    return {
      filename,
      normalizedStem,
      status: "matched",
      matchedBy: "ean",
      matchedModel: eanMatches[0],
      candidates: eanMatches,
    };
  }
  if (eanMatches.length > 1) {
    return {
      filename,
      normalizedStem,
      status: "ambiguous",
      matchedBy: "ean",
      matchedModel: null,
      candidates: eanMatches,
    };
  }

  return {
    filename,
    normalizedStem,
    status: "unmatched",
    matchedBy: null,
    matchedModel: null,
    candidates: [],
  };
}
