import { generateThemeVariables } from "./color";
import type { BrandingConfig } from "./tokens";

/**
 * Aplica as cores da marca (branding) e configurações do tenant às variáveis CSS na raiz do DOM.
 * Gera dinamicamente a escala semântica em 11 passos (50 a 950) e cores acessíveis WCAG
 * para ambos os modos (claro e escuro). Nenhuma cor fixa nos componentes.
 */
export function applyBranding(branding: BrandingConfig): void {
  if (typeof document === "undefined") return;

  const primary = branding.primary_color || "#2563eb";
  const secondary = branding.secondary_color || "#475569";
  const accent = branding.accent_color || "#7c3aed";

  const { light, dark } = generateThemeVariables({
    primary_color: primary,
    secondary_color: secondary,
    accent_color: accent,
  });

  // Injeta tag <style> com regras para :root e .dark permitindo transição nativa de temas
  let styleEl = document.getElementById(
    "eventrent-theme-variables",
  ) as HTMLStyleElement | null;
  if (!styleEl) {
    styleEl = document.createElement("style");
    styleEl.id = "eventrent-theme-variables";
    document.head.appendChild(styleEl);
  }

  const lightCss = Object.entries(light)
    .map(([key, val]) => `  ${key}: ${val};`)
    .join("\n");

  const darkCss = Object.entries(dark)
    .map(([key, val]) => `  ${key}: ${val};`)
    .join("\n");

  styleEl.textContent = `:root {\n${lightCss}\n}\n.dark {\n${darkCss}\n}`;

  // Fallback direto no documentElement
  for (const [key, val] of Object.entries(light)) {
    document.documentElement.style.setProperty(key, val);
  }

  if (typeof window !== "undefined") {
    window.dispatchEvent(
      new CustomEvent("eventrent:branding", { detail: branding }),
    );
  }
}
