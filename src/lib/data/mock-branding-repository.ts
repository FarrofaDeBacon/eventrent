import type { BrandingRepository } from "./branding-repository";
import {
  getSharedMockState,
  saveSharedMockState,
} from "./mock-auth-repository";
import {
  DEFAULT_COMPANY_NAME,
  type PublicBranding,
  type TenantBranding,
  type UpdateBrandingInput,
} from "./types";

export class MockBrandingRepository implements BrandingRepository {
  private isolatedBranding: TenantBranding | null = null;

  constructor(initialBranding?: Partial<TenantBranding>) {
    if (initialBranding) {
      this.isolatedBranding = {
        id: "018f0000-0000-7000-8000-000000000002",
        tenant_id: "018f0000-0000-7000-8000-000000000001",
        company_name: initialBranding.company_name ?? DEFAULT_COMPANY_NAME,
        primary_color: initialBranding.primary_color ?? "#2563eb",
        secondary_color: initialBranding.secondary_color ?? "#475569",
        accent_color: initialBranding.accent_color ?? "#7c3aed",
        logo_url: initialBranding.logo_url ?? null,
        logo_dark_url: initialBranding.logo_dark_url ?? null,
        icon_url: initialBranding.icon_url ?? null,
        favicon_url: null,
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
        deleted_at: null,
        created_by: null,
      };
    }
  }

  async getPublicBranding(): Promise<PublicBranding | null> {
    const current = this.isolatedBranding ?? getSharedMockState().branding;
    if (!current) return null;
    return {
      company_name: current.company_name,
      primary_color: current.primary_color,
      secondary_color: current.secondary_color,
      accent_color: current.accent_color,
      logo_url: current.logo_url ?? null,
      logo_dark_url: current.logo_dark_url ?? null,
      icon_url: current.icon_url ?? null,
    };
  }

  async getBranding(tenantId: string): Promise<TenantBranding | null> {
    const current = this.isolatedBranding ?? getSharedMockState().branding;
    if (current.tenant_id === tenantId) {
      return { ...current };
    }
    return null;
  }

  async saveBranding(
    tenantId: string,
    input: UpdateBrandingInput,
  ): Promise<TenantBranding> {
    const base = this.isolatedBranding ?? getSharedMockState().branding;
    const updated: TenantBranding = {
      ...base,
      tenant_id: tenantId,
      company_name: input.company_name,
      primary_color: input.primary_color,
      secondary_color: input.secondary_color,
      accent_color: input.accent_color,
      logo_url: input.logo_url ?? null,
      logo_dark_url: input.logo_dark_url ?? null,
      icon_url: input.icon_url ?? null,
      updated_at: new Date().toISOString(),
    };

    if (this.isolatedBranding) {
      this.isolatedBranding = updated;
    } else {
      const state = getSharedMockState();
      state.branding = updated;
      saveSharedMockState(state);
    }

    return { ...updated };
  }
}
