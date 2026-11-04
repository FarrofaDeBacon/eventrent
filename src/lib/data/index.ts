import type { BrandingRepository } from "./branding-repository";
import { MockBrandingRepository } from "./mock-branding-repository";
import { TauriBrandingRepository } from "./tauri-branding-repository";

export * from "./branding-repository";
export * from "./mock-branding-repository";
export * from "./tauri-branding-repository";
export * from "./types";

let cachedRepository: BrandingRepository | null = null;

/**
 * Retorna a instância ativa do repositório de branding.
 * Utiliza o Tauri IPC quando em ambiente desktop; fallback seguro para mock em navegador/testes.
 */
export function getBrandingRepository(): BrandingRepository {
  if (cachedRepository) {
    return cachedRepository;
  }

  const isTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  if (isTauri) {
    cachedRepository = new TauriBrandingRepository();
  } else {
    cachedRepository = new MockBrandingRepository();
  }

  return cachedRepository;
}

export function setBrandingRepository(repo: BrandingRepository): void {
  cachedRepository = repo;
}
