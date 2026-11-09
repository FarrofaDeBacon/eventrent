export interface ThemeTokens {
  background: string;
  foreground: string;
  card: string;
  cardForeground: string;
  popover: string;
  popoverForeground: string;
  primary: string;
  primaryForeground: string;
  secondary: string;
  secondaryForeground: string;
  muted: string;
  mutedForeground: string;
  accent: string;
  accentForeground: string;
  destructive: string;
  destructiveForeground: string;
  border: string;
  input: string;
  ring: string;
  radius: string;
}

export interface BrandingConfig {
  company_name: string;
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  logo_url?: string | null;
  logo_dark_url?: string | null;
  icon_url?: string | null;
}

export const DEFAULT_BRANDING_COLORS = {
  primary: "#2563eb",
  secondary: "#475569",
  accent: "#7c3aed",
} as const;

export interface ThemePreset {
  id: string;
  name: string;
  primary: string;
  secondary: string;
  accent: string;
}

export const THEME_PRESETS: ThemePreset[] = [
  {
    id: "default-blue",
    name: "Azul Safira (Padrão)",
    primary: "#2563eb",
    secondary: "#475569",
    accent: "#7c3aed",
  },
  {
    id: "emerald-tech",
    name: "Esmeralda Tech",
    primary: "#059669",
    secondary: "#334155",
    accent: "#0284c7",
  },
  {
    id: "amber-warm",
    name: "Âmbar Quente",
    primary: "#d97706",
    secondary: "#44403c",
    accent: "#ea580c",
  },
  {
    id: "royal-purple",
    name: "Roxo Real",
    primary: "#7c3aed",
    secondary: "#334155",
    accent: "#ec4899",
  },
  {
    id: "indigo-night",
    name: "Índigo Noturno",
    primary: "#4f46e5",
    secondary: "#1e293b",
    accent: "#06b6d4",
  },
  {
    id: "modern-charcoal",
    name: "Carvão Moderno",
    primary: "#374151",
    secondary: "#4b5563",
    accent: "#6366f1",
  },
];
