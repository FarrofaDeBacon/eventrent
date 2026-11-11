import { applyBranding } from "$lib/theme/branding";
import { getAuthRepository, hasPermission } from "./index";
import {
  DEFAULT_COMPANY_NAME,
  type AppStatus,
  type SessionInfo,
  type TenantBranding,
} from "./types";

class AppSessionState {
  initialized = $state(false);
  hasOwner = $state(false);
  defaultTenantId = $state("018f0000-0000-7000-8000-000000000001");
  session = $state<SessionInfo | null>(null);
  companyName = $state(DEFAULT_COMPANY_NAME);
  logoUrl = $state<string | null>(null);
  logoDarkUrl = $state<string | null>(null);
  iconUrl = $state<string | null>(null);

  applyStatus(status: AppStatus) {
    this.defaultTenantId = status.default_tenant_id;
    this.hasOwner = status.has_owner;
    this.session = status.session;
    const activeTenant = status.session?.tenant_id || status.default_tenant_id;
    if (activeTenant && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("eventrent:brand:last", activeTenant);
      } catch {
        // Ignora erros de quota ou modo privado
      }
    }
    if (status.branding) {
      this.setBranding(status.branding, activeTenant);
    }
    this.initialized = true;
  }

  setBranding(
    branding: Pick<
      TenantBranding,
      | "company_name"
      | "primary_color"
      | "secondary_color"
      | "accent_color"
      | "logo_url"
      | "logo_dark_url"
      | "icon_url"
    >,
    tenantId?: string,
  ) {
    this.companyName = branding.company_name || DEFAULT_COMPANY_NAME;
    this.logoUrl = branding.logo_url ?? null;
    this.logoDarkUrl = branding.logo_dark_url ?? null;
    this.iconUrl = branding.icon_url ?? null;
    applyBranding(branding);

    // Cache leve para prevenção de FOUC: grava SOMENTE cores e nome, NUNCA dados binários de imagens.
    // Escopado por tenant para isolamento completo em ambientes multitenant.
    if (typeof localStorage !== "undefined") {
      try {
        const targetTenant = tenantId || this.defaultTenantId || "local";
        const cachePayload = {
          company_name: this.companyName,
          primary_color: branding.primary_color,
          secondary_color: branding.secondary_color,
          accent_color: branding.accent_color,
        };
        localStorage.setItem(
          `eventrent:brand:${targetTenant}`,
          JSON.stringify(cachePayload),
        );
        localStorage.setItem("eventrent:brand:last", targetTenant);
      } catch {
        // Ignora erros de quota ou modo privado
      }
    }

    if (typeof document !== "undefined") {
      document.title = `${this.companyName} — EventRent`;
    }
  }

  setSession(session: SessionInfo | null) {
    this.session = session;
    if (session) {
      this.hasOwner = true;
      if (session.tenant_id && typeof localStorage !== "undefined") {
        try {
          localStorage.setItem("eventrent:brand:last", session.tenant_id);
        } catch {
          // Ignora erros de quota ou modo privado
        }
      }
    }
  }

  can(permission: string): boolean {
    return hasPermission(this.session, permission);
  }

  async refresh(): Promise<AppStatus> {
    const repo = getAuthRepository();
    const status = await repo.getAppStatus();
    this.applyStatus(status);
    return status;
  }
}

export const appSession = new AppSessionState();
