import { describe, it, expect, beforeEach } from "vitest";
import { MockBrandingRepository } from "./mock-branding-repository";
import type { BrandingRepository } from "./branding-repository";
import { DEFAULT_COMPANY_NAME } from "./types";

describe("BrandingRepository (Data Layer Port & Adapter)", () => {
  let repository: BrandingRepository;
  const tenantId = "018f0000-0000-7000-8000-000000000001";

  beforeEach(() => {
    repository = new MockBrandingRepository();
  });

  it("deve carregar o branding inicial do tenant com nome padrão 'Minha empresa'", async () => {
    const branding = await repository.getBranding(tenantId);
    expect(branding).not.toBeNull();
    expect(branding?.company_name).toBe(DEFAULT_COMPANY_NAME);
    expect(branding?.primary_color).toBe("#2563eb");
    expect(branding?.secondary_color).toBe("#475569");
  });

  it("deve atualizar e persistir as alterações de branding do tenant", async () => {
    const updated = await repository.saveBranding(tenantId, {
      company_name: "Som & Iluminação Prime",
      primary_color: "#10b981",
      secondary_color: "#0f172a",
      accent_color: "#f59e0b",
      logo_url: "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
    });

    expect(updated.company_name).toBe("Som & Iluminação Prime");
    expect(updated.primary_color).toBe("#10b981");
    expect(updated.secondary_color).toBe("#0f172a");
    expect(updated.accent_color).toBe("#f59e0b");
    expect(updated.logo_url).toBe("data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=");

    // Confirma que a nova consulta retorna o estado atualizado
    const reloaded = await repository.getBranding(tenantId);
    expect(reloaded?.company_name).toBe("Som & Iluminação Prime");
    expect(reloaded?.primary_color).toBe("#10b981");
  });

  it("deve retornar null para tenant inexistente", async () => {
    const notFound = await repository.getBranding("tenant-inexistente-123");
    expect(notFound).toBeNull();
  });
});
