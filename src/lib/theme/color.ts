/**
 * Conversões de cores sRGB <-> OKLab <-> OKLCH, geração de escala semântica em 11 passos,
 * mapeamento de gamut e cálculo de contraste acessível WCAG 2.1.
 * Implementação pura em TypeScript sem dependências externas.
 */

export interface OklchColor {
  l: number; // [0, 1]
  c: number; // [0, ~0.4]
  h: number; // [0, 360)
}

export type ScaleStep =
  50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 950;

export interface ScaleItem {
  hex: string;
  oklch: string;
  l: number;
  c: number;
  h: number;
}

export type ColorScale = Record<ScaleStep, ScaleItem>;

export const SCALE_STEPS: ScaleStep[] = [
  50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950,
];

// Target lightness and chroma multiplier for the 11-step scale
const SCALE_CONFIG: Record<ScaleStep, { l: number; cFactor: number }> = {
  50: { l: 0.97, cFactor: 0.25 },
  100: { l: 0.93, cFactor: 0.4 },
  200: { l: 0.86, cFactor: 0.65 },
  300: { l: 0.77, cFactor: 0.85 },
  400: { l: 0.67, cFactor: 0.95 },
  500: { l: 0.57, cFactor: 1.0 },
  600: { l: 0.47, cFactor: 0.98 },
  700: { l: 0.38, cFactor: 0.88 },
  800: { l: 0.29, cFactor: 0.75 },
  900: { l: 0.21, cFactor: 0.6 },
  950: { l: 0.14, cFactor: 0.4 },
};

/**
 * Converte componente sRGB normalizado [0, 1] em sRGB linear.
 */
export function srgbToLinear(c: number): number {
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/**
 * Converte componente sRGB linear em sRGB normalizado [0, 1].
 */
export function linearToSrgb(c: number): number {
  return c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
}

export const HEX_COLOR_STRICT_REGEX = /^#[0-9a-fA-F]{6}$/;

/**
 * Valida se uma string é um código de cor hexadecimal estrito no formato #RRGGBB.
 */
export function isValidHexColor(hex: string): boolean {
  return HEX_COLOR_STRICT_REGEX.test(hex.trim());
}

/**
 * Converte cor HEX (#RRGGBB) para valores normalizados sRGB [0, 1].
 * Valida estritamente contra caracteres não-hexadecimais, rejeitando strings como '#1g0000'.
 */
export function hexToRgb(hex: string): [number, number, number] {
  const trimmed = hex.trim();
  const clean = trimmed.startsWith("#") ? trimmed.slice(1) : trimmed;
  if (!/^[0-9a-fA-F]{6}$/.test(clean)) {
    throw new Error(
      `Cor HEX inválida ou com caracteres não-hexadecimais: '${hex}'. Esperado formato #RRGGBB com 6 dígitos hexadecimais válidos.`,
    );
  }
  const r = parseInt(clean.substring(0, 2), 16) / 255;
  const g = parseInt(clean.substring(2, 4), 16) / 255;
  const b = parseInt(clean.substring(4, 6), 16) / 255;
  return [r, g, b];
}

/**
 * Converte valores normalizados sRGB [0, 1] para string HEX (#RRGGBB).
 */
export function rgbToHex(rgb: [number, number, number]): string {
  const toHex = (n: number) => {
    const clamped = Math.min(255, Math.max(0, Math.round(n * 255)));
    return clamped.toString(16).padStart(2, "0");
  };
  return `#${toHex(rgb[0])}${toHex(rgb[1])}${toHex(rgb[2])}`.toLowerCase();
}

/**
 * Converte sRGB linear para espaço OKLab.
 */
export function linearRgbToOklab(
  r: number,
  g: number,
  b: number,
): [number, number, number] {
  const l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
  const m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
  const s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

  const l_ = Math.cbrt(l);
  const m_ = Math.cbrt(m);
  const s_ = Math.cbrt(s);

  const L = 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_;
  const a = 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_;
  const bOklab = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_;

  return [L, a, bOklab];
}

/**
 * Converte OKLab para sRGB linear.
 */
export function oklabToLinearRgb(
  L: number,
  a: number,
  b: number,
): [number, number, number] {
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;

  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;

  const r = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const bRgb = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;

  return [r, g, bRgb];
}

/**
 * Converte HEX (#RRGGBB) para coordenadas OKLCH.
 */
export function hexToOklch(hex: string): OklchColor {
  const [r, g, b] = hexToRgb(hex);
  const rLin = srgbToLinear(r);
  const gLin = srgbToLinear(g);
  const bLin = srgbToLinear(b);

  const [L, a, bOklab] = linearRgbToOklab(rLin, gLin, bLin);
  const c = Math.sqrt(a * a + bOklab * bOklab);
  let h = (Math.atan2(bOklab, a) * 180) / Math.PI;
  if (h < 0) h += 360;
  if (c < 1e-5) h = 0;

  return {
    l: Math.round(L * 10000) / 10000,
    c: Math.round(c * 10000) / 10000,
    h: Math.round(h * 100) / 100,
  };
}

/**
 * Converte OKLCH para sRGB linear intermediário.
 */
function oklchToLinearRgbInternal(
  l: number,
  c: number,
  h: number,
): [number, number, number] {
  const hRad = (h * Math.PI) / 180;
  const a = c * Math.cos(hRad);
  const b = c * Math.sin(hRad);
  return oklabToLinearRgb(l, a, b);
}

function inGamut(rgb: [number, number, number]): boolean {
  const eps = 0.001; // Tolerância para imprecisões de ponto flutuante na fronteira do sRGB
  return (
    rgb[0] >= -eps &&
    rgb[0] <= 1 + eps &&
    rgb[1] >= -eps &&
    rgb[1] <= 1 + eps &&
    rgb[2] >= -eps &&
    rgb[2] <= 1 + eps
  );
}

/**
 * Mapeia OKLCH de volta para o gamut sRGB preservando luminosidade perceptual e matiz.
 */
export function oklchToGamutMappedRgb(
  l: number,
  c: number,
  h: number,
): [number, number, number] {
  if (l <= 0) return [0, 0, 0];
  if (l >= 1) return [1, 1, 1];

  let raw = oklchToLinearRgbInternal(l, c, h);
  let srgb = [
    linearToSrgb(raw[0]),
    linearToSrgb(raw[1]),
    linearToSrgb(raw[2]),
  ] as [number, number, number];

  if (inGamut(srgb)) {
    return [
      Math.min(1, Math.max(0, srgb[0])),
      Math.min(1, Math.max(0, srgb[1])),
      Math.min(1, Math.max(0, srgb[2])),
    ];
  }

  // Busca binária rápida no chroma para ajustar no gamut sRGB sem alteração de matiz ou brilho
  let low = 0;
  let high = c;
  let bestRgb: [number, number, number] = [0, 0, 0];

  for (let i = 0; i < 20; i++) {
    const mid = (low + high) / 2;
    raw = oklchToLinearRgbInternal(l, mid, h);
    srgb = [linearToSrgb(raw[0]), linearToSrgb(raw[1]), linearToSrgb(raw[2])];
    if (inGamut(srgb)) {
      bestRgb = [
        Math.min(1, Math.max(0, srgb[0])),
        Math.min(1, Math.max(0, srgb[1])),
        Math.min(1, Math.max(0, srgb[2])),
      ];
      low = mid;
    } else {
      high = mid;
    }
  }

  return bestRgb;
}

/**
 * Converte OKLCH para string HEX (#RRGGBB).
 */
export function oklchToHex(l: number, c: number, h: number): string {
  const rgb = oklchToGamutMappedRgb(l, c, h);
  return rgbToHex(rgb);
}

/**
 * Formata OKLCH como string CSS para uso em variáveis DOM: `oklch(L C H)`.
 */
export function oklchToCss(l: number, c: number, h: number): string {
  const round4 = (n: number) => Math.round(n * 10000) / 10000;
  const round2 = (n: number) => Math.round(n * 100) / 100;
  return `oklch(${round4(l)} ${round4(c)} ${round2(h)})`;
}

/**
 * Gera a escala semântica completa de 11 passos (50 a 950) a partir de uma cor primária HEX.
 */
export function generateColorScale(hex: string): ColorScale {
  const base = hexToOklch(hex);
  const scale = {} as ColorScale;

  for (const step of SCALE_STEPS) {
    const config = SCALE_CONFIG[step];
    const targetL = config.l;
    const targetC = base.c * config.cFactor;
    const targetH = base.h;

    const mappedHex = oklchToHex(targetL, targetC, targetH);
    const css = oklchToCss(targetL, targetC, targetH);

    scale[step] = {
      hex: mappedHex,
      oklch: css,
      l: targetL,
      c: targetC,
      h: targetH,
    };
  }

  return scale;
}

/**
 * Calcula a luminância relativa WCAG 2.1 (L = 0.2126R + 0.7152G + 0.0722B).
 */
export function getRelativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex);
  const rLin = srgbToLinear(r);
  const gLin = srgbToLinear(g);
  const bLin = srgbToLinear(b);
  return 0.2126 * rLin + 0.7152 * gLin + 0.0722 * bLin;
}

/**
 * Calcula a razão bruta de contraste WCAG 2.1 entre duas cores (sem arredondamento).
 */
export function getRawContrastRatio(hex1: string, hex2: string): number {
  const lum1 = getRelativeLuminance(hex1);
  const lum2 = getRelativeLuminance(hex2);
  const lighter = Math.max(lum1, lum2);
  const darker = Math.min(lum1, lum2);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Avalia conformidade WCAG AA (taxa >= 4.5:1) usando estritamente a razão BRUTA sem arredondamentos prévios.
 * Exemplo: 4.4999 reprova (false), 4.5000 aprova (true).
 */
export function isAccessibleContrast(rawRatio: number): boolean {
  return rawRatio >= 4.5;
}

/**
 * Arredonda a taxa de contraste para 2 casas decimais estritamente para fins de exibição na interface.
 */
export function formatContrastRatio(ratio: number): number {
  return Math.round(ratio * 100) / 100;
}

/**
 * Calcula a taxa de contraste WCAG 2.1 entre duas cores (exibição arredondada para 2 casas decimais).
 */
export function calculateContrast(hex1: string, hex2: string): number {
  return formatContrastRatio(getRawContrastRatio(hex1, hex2));
}

export interface ForegroundOption {
  hex: string;
  css: string;
  contrast: number;
  isAccessible: boolean; // >= 4.5:1 (WCAG AA usando razão bruta)
}

/**
 * Escolhe automaticamente a cor de texto (foreground) acessível (branco ou escuro neutro)
 * garantindo taxa de contraste >= 4.5:1 (WCAG AA) em relação ao fundo usando a razão BRUTA.
 */
export function getAccessibleForeground(
  backgroundHex: string,
): ForegroundOption {
  const whiteHex = "#ffffff";
  const darkHex = "#090d16"; // Escuro neutro de alta fidelidade
  const whiteCss = "oklch(0.99 0 0)";
  const darkCss = "oklch(0.12 0.015 260)";

  const rawWhite = getRawContrastRatio(backgroundHex, whiteHex);
  const rawDark = getRawContrastRatio(backgroundHex, darkHex);

  const displayWhite = formatContrastRatio(rawWhite);
  const displayDark = formatContrastRatio(rawDark);

  if (isAccessibleContrast(rawWhite)) {
    return {
      hex: whiteHex,
      css: whiteCss,
      contrast: displayWhite,
      isAccessible: true,
    };
  }

  if (isAccessibleContrast(rawDark)) {
    return {
      hex: darkHex,
      css: darkCss,
      contrast: displayDark,
      isAccessible: true,
    };
  }

  // Se nenhum atingir 4.5:1 bruto, escolhe a opção com maior contraste relativo bruto
  if (rawWhite >= rawDark) {
    return {
      hex: whiteHex,
      css: whiteCss,
      contrast: displayWhite,
      isAccessible: false,
    };
  } else {
    return {
      hex: darkHex,
      css: darkCss,
      contrast: displayDark,
      isAccessible: false,
    };
  }
}

/**
 * Ajusta a luminosidade no espaço OKLCH para atingir taxa de contraste >= 4.5:1 (WCAG AA) usando razão bruta.
 * Preserva o matiz (h) e croma (c) da cor original da marca.
 */
export function autoAdjustContrast(hex: string): string {
  const oklch = hexToOklch(hex);
  const fg = getAccessibleForeground(hex);
  if (fg.isAccessible) {
    return hex;
  }

  const rawWhite = getRawContrastRatio(hex, "#ffffff");
  const rawDark = getRawContrastRatio(hex, "#090d16");

  if (rawWhite >= rawDark) {
    let l = oklch.l;
    for (let i = 0; i < 25; i++) {
      l = Math.max(0.05, l - 0.025);
      const candidateHex = oklchToHex(l, oklch.c, oklch.h);
      if (isAccessibleContrast(getRawContrastRatio(candidateHex, "#ffffff"))) {
        return candidateHex;
      }
    }
  } else {
    let l = oklch.l;
    for (let i = 0; i < 25; i++) {
      l = Math.min(0.95, l + 0.025);
      const candidateHex = oklchToHex(l, oklch.c, oklch.h);
      if (isAccessibleContrast(getRawContrastRatio(candidateHex, "#090d16"))) {
        return candidateHex;
      }
    }
  }

  return hex;
}

/**
 * Gera dicionário de variáveis CSS prontas para injeção nos modos claro e escuro.
 */
export function generateThemeVariables(branding: {
  primary_color: string;
  secondary_color: string;
  accent_color: string;
}): { light: Record<string, string>; dark: Record<string, string> } {
  const primaryScale = generateColorScale(branding.primary_color);
  const secondaryScale = generateColorScale(branding.secondary_color);
  const accentScale = generateColorScale(branding.accent_color);

  const primaryFg = getAccessibleForeground(branding.primary_color);
  const secondaryFg = getAccessibleForeground(branding.secondary_color);
  const accentFg = getAccessibleForeground(branding.accent_color);

  // Escala para tema claro
  const light: Record<string, string> = {
    "--primary": branding.primary_color,
    "--primary-foreground": primaryFg.css,
    "--secondary": branding.secondary_color,
    "--secondary-foreground": secondaryFg.css,
    "--accent": branding.accent_color,
    "--accent-foreground": accentFg.css,
    "--ring": branding.primary_color,
  };

  // Escala para tema escuro: ajusta primária para passo com boa legibilidade sobre fundos escuros
  const darkPrimary = primaryScale[400].hex;
  const darkPrimaryFg = getAccessibleForeground(darkPrimary);

  const dark: Record<string, string> = {
    "--primary": darkPrimary,
    "--primary-foreground": darkPrimaryFg.css,
    "--secondary": secondaryScale[700].hex,
    "--secondary-foreground": "oklch(0.98 0.005 260)",
    "--accent": accentScale[700].hex,
    "--accent-foreground": "oklch(0.98 0.005 260)",
    "--ring": darkPrimary,
  };

  // Injetar os 11 passos da escala de cada cor
  for (const step of SCALE_STEPS) {
    light[`--primary-${step}`] = primaryScale[step].hex;
    light[`--secondary-${step}`] = secondaryScale[step].hex;
    light[`--accent-${step}`] = accentScale[step].hex;

    dark[`--primary-${step}`] = primaryScale[step].hex;
    dark[`--secondary-${step}`] = secondaryScale[step].hex;
    dark[`--accent-${step}`] = accentScale[step].hex;
  }

  return { light, dark };
}
