import { describe, it, expect, beforeEach } from "vitest";
import { MockBrandingRepository } from "./mock-branding-repository";
import { resetSharedMockState } from "./mock-auth-repository";
import type { BrandingRepository } from "./branding-repository";
import { DEFAULT_COMPANY_NAME } from "./types";

describe("BrandingRepository (Data Layer Port & Adapter)", () => {
  let repository: BrandingRepository;
  const tenantId = "018f0000-0000-7000-8000-000000000001";

  beforeEach(() => {
    resetSharedMockState();
    repository = new MockBrandingRepository();
  });

  it("deve carregar o branding inicial do tenant com nome padrão 'Minha empresa'", async () => {
    const branding = await repository.getBranding(tenantId);
    expect(branding).not.toBeNull();
    expect(branding?.company_name).toBe(DEFAULT_COMPANY_NAME);
    expect(branding?.primary_color).toBe("#2563eb");
    expect(branding?.secondary_color).toBe("#475569");
  });

  it("deve atualizar e persistir as alterações de branding do tenant incluindo logo escuro e ícone", async () => {
    const pngSample =
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=";
    const updated = await repository.saveBranding(tenantId, {
      company_name: "Som & Iluminação Prime",
      primary_color: "#10b981",
      secondary_color: "#0f172a",
      accent_color: "#f59e0b",
      logo_url: pngSample,
      logo_dark_url: pngSample,
      icon_url: pngSample,
    });

    expect(updated.company_name).toBe("Som & Iluminação Prime");
    expect(updated.primary_color).toBe("#10b981");
    expect(updated.secondary_color).toBe("#0f172a");
    expect(updated.accent_color).toBe("#f59e0b");
    expect(updated.logo_url).toBe(pngSample);
    expect(updated.logo_dark_url).toBe(pngSample);
    expect(updated.icon_url).toBe(pngSample);

    // Confirma que a nova consulta retorna o estado atualizado
    const reloaded = await repository.getBranding(tenantId);
    expect(reloaded?.company_name).toBe("Som & Iluminação Prime");
    expect(reloaded?.primary_color).toBe("#10b981");
    expect(reloaded?.logo_dark_url).toBe(pngSample);
    expect(reloaded?.icon_url).toBe(pngSample);
  });

  it("deve carregar o branding público sem requerer autenticação contendo campos de logo e ícone", async () => {
    const publicBranding = await repository.getPublicBranding();
    expect(publicBranding).not.toBeNull();
    expect(publicBranding?.company_name).toBe(DEFAULT_COMPANY_NAME);
    expect(publicBranding?.primary_color).toBe("#2563eb");
    expect(publicBranding?.secondary_color).toBe("#475569");
    expect(publicBranding?.accent_color).toBe("#7c3aed");
    expect(publicBranding).toHaveProperty("logo_url");
    expect(publicBranding).toHaveProperty("logo_dark_url");
    expect(publicBranding).toHaveProperty("icon_url");
  });

  it("deve retornar null para tenant inexistente", async () => {
    const notFound = await repository.getBranding("tenant-inexistente-123");
    expect(notFound).toBeNull();
  });

  it("deve gravar cache leve no localStorage escopado por tenant sem dados de imagens", async () => {
    const { appSession } = await import("./session.svelte");
    const testTenant = "tenant-audit-99";
    const pngSample =
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=";

    appSession.setBranding(
      {
        company_name: "Empresa Auditada",
        primary_color: "#123456",
        secondary_color: "#654321",
        accent_color: "#abcdef",
        logo_url: pngSample,
        logo_dark_url: pngSample,
        icon_url: pngSample,
      },
      testTenant,
    );

    const cachedRaw = localStorage.getItem(`eventrent:brand:${testTenant}`);
    expect(cachedRaw).not.toBeNull();
    const cached = JSON.parse(cachedRaw!);

    expect(cached).toEqual({
      company_name: "Empresa Auditada",
      primary_color: "#123456",
      secondary_color: "#654321",
      accent_color: "#abcdef",
    });

    // Garante que imagens pesadas JAMAIS são gravadas no localStorage
    expect(cached).not.toHaveProperty("logo_url");
    expect(cached).not.toHaveProperty("logo_dark_url");
    expect(cached).not.toHaveProperty("icon_url");
    expect(cachedRaw).not.toContain("data:image");
  });
});
