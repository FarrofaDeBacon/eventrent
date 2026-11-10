import { describe, expect, it } from "vitest";
import {
  autoAdjustContrast,
  calculateContrast,
  generateColorScale,
  generateThemeVariables,
  getAccessibleForeground,
  getRelativeLuminance,
  hexToOklch,
  hexToRgb,
  oklchToCss,
  oklchToHex,
  rgbToHex,
  SCALE_STEPS,
} from "./color";

describe("Color Engine - OKLCH & WCAG Contrast (src/lib/theme/color.ts)", () => {
  describe("Conversões básicas de cor e espaço de cor", () => {
    it("converte HEX <-> RGB normalizado com precisão", () => {
      expect(hexToRgb("#ffffff")).toEqual([1, 1, 1]);
      expect(hexToRgb("#000000")).toEqual([0, 0, 0]);
      expect(hexToRgb("#ff0000")).toEqual([1, 0, 0]);

      expect(rgbToHex([1, 1, 1])).toBe("#ffffff");
      expect(rgbToHex([0, 0, 0])).toBe("#000000");
      expect(rgbToHex([1, 0, 0])).toBe("#ff0000");
    });

    it("rejeita formatos HEX inválidos com mensagens claras", () => {
      expect(() => hexToRgb("#fff")).toThrow("formato #RRGGBB");
      expect(() => hexToRgb("invalid")).toThrow();
      expect(() => hexToRgb("#gggggg")).toThrow("não-hexadecimais");
    });

    it("converte HEX para OKLCH com alta precisão nos valores de referência oficiais", () => {
      // 1. Vermelho puro (#ff0000)
      const red = hexToOklch("#ff0000");
      expect(Math.abs(red.l - 0.628)).toBeLessThanOrEqual(0.001);
      expect(Math.abs(red.c - 0.2577)).toBeLessThanOrEqual(0.002);
      expect(Math.abs(red.h - 29.23)).toBeLessThanOrEqual(0.1);

      // 2. Azul puro (#0000ff)
      const blue = hexToOklch("#0000ff");
      expect(Math.abs(blue.l - 0.452)).toBeLessThanOrEqual(0.001);
      expect(Math.abs(blue.c - 0.3132)).toBeLessThanOrEqual(0.002);
      expect(Math.abs(blue.h - 264.05)).toBeLessThanOrEqual(0.1);

      // 3. Verde puro (#00ff00)
      const green = hexToOklch("#00ff00");
      expect(Math.abs(green.l - 0.8664)).toBeLessThanOrEqual(0.001);
      expect(Math.abs(green.c - 0.2948)).toBeLessThanOrEqual(0.002);
      expect(Math.abs(green.h - 142.5)).toBeLessThanOrEqual(0.1);

      // 4. Branco puro (#ffffff): L = 1 e C = 0
      const white = hexToOklch("#ffffff");
      expect(white.l).toBe(1);
      expect(white.c).toBe(0);

      // 5. Preto puro (#000000): L = 0 e C = 0
      const black = hexToOklch("#000000");
      expect(black.l).toBe(0);
      expect(black.c).toBe(0);
    });

    it("garante ida e volta (round-trip) hex -> OKLCH -> hex sem perda alem de 1 unidade por canal", () => {
      const sampleHexes = [
        "#ff0000",
        "#00ff00",
        "#0000ff",
        "#ffffff",
        "#000000",
        "#2563eb",
        "#7c3aed",
        "#059669",
        "#d97706",
        "#dc2626",
        "#1e293b",
        "#f8fafc",
        "#767676",
        "#777777",
        "#123456",
        "#abcdef",
      ];

      for (const originalHex of sampleHexes) {
        const oklch = hexToOklch(originalHex);
        const mappedHex = oklchToHex(oklch.l, oklch.c, oklch.h);

        const [rOrig, gOrig, bOrig] = hexToRgb(originalHex).map((v) =>
          Math.round(v * 255),
        );
        const [rBack, gBack, bBack] = hexToRgb(mappedHex).map((v) =>
          Math.round(v * 255),
        );

        expect(Math.abs(rOrig - rBack)).toBeLessThanOrEqual(1);
        expect(Math.abs(gOrig - gBack)).toBeLessThanOrEqual(1);
        expect(Math.abs(bOrig - bBack)).toBeLessThanOrEqual(1);
      }
    });

    it("formata string CSS oklch válida", () => {
      const css = oklchToCss(0.65, 0.2, 260);
      expect(css).toBe("oklch(0.65 0.2 260)");
      expect(oklchToHex(1, 0, 0)).toBe("#ffffff");
      expect(oklchToHex(0, 0, 0)).toBe("#000000");
    });
  });

  describe("Geração da escala semântica em 11 passos (50 a 950)", () => {
    it("gera todos os 11 passos com luminosidade estritamente decrescente", () => {
      const scale = generateColorScale("#2563eb");

      // Deve conter todos os passos
      for (const step of SCALE_STEPS) {
        expect(scale[step]).toBeDefined();
        expect(scale[step].hex).toMatch(/^#[0-9a-f]{6}$/);
        expect(scale[step].oklch).toContain("oklch(");
      }

      // Passo 50 deve ser claro (L > 0.90)
      expect(scale[50].l).toBeGreaterThan(0.9);

      // Passo 950 deve ser escuro (L < 0.20)
      expect(scale[950].l).toBeLessThan(0.2);

      // Luminosidade decrescente monotonicamente
      for (let i = 0; i < SCALE_STEPS.length - 1; i++) {
        const current = scale[SCALE_STEPS[i]];
        const next = scale[SCALE_STEPS[i + 1]];
        expect(current.l).toBeGreaterThan(next.l);
      }
    });

    it("mapeia cores extremas sem artefatos ou valores fora do gamut", () => {
      // Amarelo intenso (#ffff00): testa preservação e contenção no gamut
      const yellowScale = generateColorScale("#ffff00");
      for (const step of SCALE_STEPS) {
        expect(yellowScale[step].hex).toMatch(/^#[0-9a-f]{6}$/);
      }

      // Cinza neutro (#808080)
      const grayScale = generateColorScale("#808080");
      for (const step of SCALE_STEPS) {
        expect(grayScale[step].hex).toMatch(/^#[0-9a-f]{6}$/);
      }
    });
  });

  describe("Cálculo de luminância e contraste WCAG 2.1", () => {
    it("calcula luminâncias relativas corretas", () => {
      expect(getRelativeLuminance("#ffffff")).toBeCloseTo(1, 3);
      expect(getRelativeLuminance("#000000")).toBeCloseTo(0, 3);
    });

    it("calcula razões de contraste exatas nos casos fundamentais e limites de conformidade AA", () => {
      // Preto contra branco = 21:1
      expect(calculateContrast("#000000", "#ffffff")).toBe(21);
      expect(calculateContrast("#ffffff", "#000000")).toBe(21);

      // Branco contra branco = 1:1
      expect(calculateContrast("#ffffff", "#ffffff")).toBe(1);

      // Limiar oficial WCAG 2.1 AA (4.5:1)
      // #767676 sobre branco = 4.54:1 (passa AA)
      const contrast76 = calculateContrast("#767676", "#ffffff");
      expect(contrast76).toBe(4.54);
      expect(contrast76).toBeGreaterThanOrEqual(4.5);

      // #777777 sobre branco = 4.48:1 (reprova AA)
      const contrast77 = calculateContrast("#777777", "#ffffff");
      expect(contrast77).toBe(4.48);
      expect(contrast77).toBeLessThan(4.5);

      // Azul Tailwind #2563eb contra branco (#ffffff) >= 4.5:1
      const contrast = calculateContrast("#2563eb", "#ffffff");
      expect(contrast).toBeGreaterThanOrEqual(4.5);
    });

    it("garante contraste mínimo de 4.5:1 em primary-foreground para um conjunto diverso de mais de 50 cores de marca", () => {
      const testPalette = [
        // Cores primárias puras e secundárias
        "#ff0000",
        "#00ff00",
        "#0000ff",
        "#ffff00",
        "#ff00ff",
        "#00ffff",
        // Amarelos e laranjas (desafio para contraste escuro)
        "#facc15",
        "#eab308",
        "#ca8a04",
        "#fbbf24",
        "#f59e0b",
        "#d97706",
        "#b45309",
        "#ea580c",
        "#c2410c",
        "#9a3412",
        // Tons muito claros (quase brancos)
        "#f8fafc",
        "#f1f5f9",
        "#e2e8f0",
        "#fafafa",
        "#f4f4f5",
        "#e4e4e7",
        "#fef2f2",
        "#fee2e2",
        "#fff7ed",
        "#ffedd5",
        "#fefce8",
        "#fef9c3",
        // Tons muito escuros (quase pretos)
        "#020617",
        "#0f172a",
        "#1e293b",
        "#09090b",
        "#18181b",
        "#27272a",
        "#450a0a",
        "#7f1d1d",
        "#14532d",
        "#1e3a8a",
        "#312e81",
        "#581c87",
        // Marcas conhecidas (azuis, verdes, roxos, laranjas)
        "#1877f2",
        "#006699",
        "#1da1f2",
        "#0a66c2",
        "#4b5563",
        "#374151",
        "#9ca3af",
        "#d1d5db",
        "#a1a1aa",
        "#71717a",
        // Verdes, azuis e roxos variados
        "#059669",
        "#10b981",
        "#34d399",
        "#0284c7",
        "#0ea5e9",
        "#38bdf8",
        "#2563eb",
        "#3b82f6",
        "#60a5fa",
        "#7c3aed",
        "#8b5cf6",
        "#a78bfa",
        "#c026d3",
        "#d946ef",
        "#e879f9",
      ];

      expect(testPalette.length).toBeGreaterThanOrEqual(50);

      for (const brandColor of testPalette) {
        const fg = getAccessibleForeground(brandColor);
        expect(
          fg.contrast,
          `Cor ${brandColor} com foreground ${fg.hex} deveria ter contraste >= 4.5, obteve ${fg.contrast}`,
        ).toBeGreaterThanOrEqual(4.5);
        expect(fg.isAccessible).toBe(true);
      }
    });

    it("ajusta automaticamente cores de baixo contraste para conformidade WCAG AA (>= 4.5:1)", () => {
      const lowContrastColors = ["#888888", "#9ca3af", "#a8a29e", "#64748b"];

      for (const color of lowContrastColors) {
        const adjusted = autoAdjustContrast(color);
        expect(adjusted).toMatch(/^#[0-9a-f]{6}$/);
        const fg = getAccessibleForeground(adjusted);
        expect(fg.contrast).toBeGreaterThanOrEqual(4.5);
        expect(fg.isAccessible).toBe(true);
      }
    });

    it("seleciona automaticamente foreground acessível atendendo WCAG AA (>= 4.5:1)", () => {
      // Fundo claro (amarelo #facc15): deve selecionar texto escuro
      const yellowFg = getAccessibleForeground("#facc15");
      expect(yellowFg.hex).toBe("#090d16");
      expect(yellowFg.contrast).toBeGreaterThanOrEqual(4.5);
      expect(yellowFg.isAccessible).toBe(true);

      // Fundo escuro (azul marinho #1e3a8a): deve selecionar texto branco
      const darkBlueFg = getAccessibleForeground("#1e3a8a");
      expect(darkBlueFg.hex).toBe("#ffffff");
      expect(darkBlueFg.contrast).toBeGreaterThanOrEqual(4.5);
      expect(darkBlueFg.isAccessible).toBe(true);

      // Fundo branco: texto escuro
      const whiteFg = getAccessibleForeground("#ffffff");
      expect(whiteFg.hex).toBe("#090d16");
      expect(whiteFg.isAccessible).toBe(true);

      // Fundo preto: texto claro
      const blackFg = getAccessibleForeground("#000000");
      expect(blackFg.hex).toBe("#ffffff");
      expect(blackFg.isAccessible).toBe(true);
    });
  });

  describe("Geração completa de variáveis de tema (generateThemeVariables)", () => {
    it("gera variáveis para tema claro e escuro contendo toda a escala 50-950", () => {
      const vars = generateThemeVariables({
        primary_color: "#2563eb",
        secondary_color: "#475569",
        accent_color: "#7c3aed",
      });

      expect(vars.light["--primary"]).toBe("#2563eb");
      expect(vars.light["--primary-50"]).toMatch(/^#[0-9a-f]{6}$/);
      expect(vars.light["--primary-950"]).toMatch(/^#[0-9a-f]{6}$/);
      expect(vars.light["--secondary-500"]).toMatch(/^#[0-9a-f]{6}$/);
      expect(vars.light["--accent-500"]).toMatch(/^#[0-9a-f]{6}$/);

      expect(vars.dark["--primary"]).toBeDefined();
      expect(vars.dark["--primary-foreground"]).toBeDefined();
      expect(vars.dark["--secondary-700"]).toBeDefined();
    });
  });
});
