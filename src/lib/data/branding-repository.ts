import type {
  PublicBranding,
  TenantBranding,
  UpdateBrandingInput,
} from "./types";

/**
 * Contrato inegociável da camada de dados para o Branding do Tenant.
 * As telas conhecem apenas esta interface, nunca o meio de transporte (Tauri invoke ou HTTP).
 */
export interface BrandingRepository {
  getPublicBranding(): Promise<PublicBranding | null>;
  getBranding(tenantId: string): Promise<TenantBranding | null>;
  saveBranding(
    tenantId: string,
    input: UpdateBrandingInput,
  ): Promise<TenantBranding>;
}
