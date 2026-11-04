import type { BrandingConfig } from "./tokens";

/**
 * Converte cor HEX (#RRGGBB) para luminância estimada e define token de cor de texto em OKLCH.
 */
function getContrastForeground(hex: string): string {
  const cleanHex = hex.replace("#", "");
  if (cleanHex.length !== 6) return "oklch(0.99 0 0)";

  const r = parseInt(cleanHex.substring(0, 2), 16);
  const g = parseInt(cleanHex.substring(2, 4), 16);
  const b = parseInt(cleanHex.substring(4, 6), 16);

  // Fórmula de luminância relativa WCAG
  const yiq = (r * 299 + g * 587 + b * 114) / 1000;
  return yiq >= 128 ? "oklch(0.145 0.015 260)" : "oklch(0.99 0 0)";
}

/**
 * Aplica as cores da marca (branding) e configurações do tenant às variáveis CSS na raiz do DOM.
 * Nenhuma cor fixa nos componentes; tudo reage dinamicamente.
 */
export function applyBranding(branding: BrandingConfig): void {
  if (typeof document === "undefined") return;

  const root = document.documentElement;

  if (branding.primary_color) {
    root.style.setProperty("--primary", branding.primary_color);
    root.style.setProperty("--ring", branding.primary_color);
    const primaryFg = getContrastForeground(branding.primary_color);
    root.style.setProperty("--primary-foreground", primaryFg);
  }

  if (branding.secondary_color) {
    root.style.setProperty("--secondary", branding.secondary_color);
    const secondaryFg = getContrastForeground(branding.secondary_color);
    root.style.setProperty("--secondary-foreground", secondaryFg);
  }

  if (branding.accent_color) {
    root.style.setProperty("--accent", branding.accent_color);
    const accentFg = getContrastForeground(branding.accent_color);
    root.style.setProperty("--accent-foreground", accentFg);
  }

  if (typeof window !== "undefined") {
    window.dispatchEvent(
      new CustomEvent("eventrent:branding", { detail: branding }),
    );
  }
}
