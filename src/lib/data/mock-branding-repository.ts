import type { BrandingRepository } from "./branding-repository";
import type { TenantBranding, UpdateBrandingInput } from "./types";

export class MockBrandingRepository implements BrandingRepository {
  private branding: TenantBranding;

  constructor(initialBranding?: Partial<TenantBranding>) {
    this.branding = {
      id: "018f0000-0000-7000-8000-000000000002",
      tenant_id: "018f0000-0000-7000-8000-000000000001",
      company_name: initialBranding?.company_name ?? "EventRent Audiovisual",
      primary_color: initialBranding?.primary_color ?? "#2563eb",
      secondary_color: initialBranding?.secondary_color ?? "#475569",
      accent_color: initialBranding?.accent_color ?? "#7c3aed",
      logo_url: initialBranding?.logo_url ?? null,
      favicon_url: null,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
      deleted_at: null,
      created_by: null,
    };
  }

  async getBranding(tenantId: string): Promise<TenantBranding | null> {
    if (this.branding.tenant_id === tenantId) {
      return { ...this.branding };
    }
    return null;
  }

  async saveBranding(
    tenantId: string,
    input: UpdateBrandingInput,
  ): Promise<TenantBranding> {
    this.branding = {
      ...this.branding,
      tenant_id: tenantId,
      company_name: input.company_name,
      primary_color: input.primary_color,
      secondary_color: input.secondary_color,
      accent_color: input.accent_color,
      logo_url: input.logo_url ?? null,
      updated_at: new Date().toISOString(),
    };
    return { ...this.branding };
  }
}
