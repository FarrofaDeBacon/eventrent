import { invoke } from "@tauri-apps/api/core";
import type { BrandingRepository } from "./branding-repository";
import type { TenantBranding, UpdateBrandingInput } from "./types";

export class TauriBrandingRepository implements BrandingRepository {
  async getBranding(tenantId: string): Promise<TenantBranding | null> {
    return await invoke<TenantBranding | null>("get_branding", {
      tenantId,
    });
  }

  async saveBranding(
    tenantId: string,
    input: UpdateBrandingInput,
  ): Promise<TenantBranding> {
    return await invoke<TenantBranding>("save_branding", {
      tenantId,
      input,
    });
  }
}
