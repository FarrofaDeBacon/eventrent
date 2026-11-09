import { describe, expect, it } from "vitest";
import {
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

    it("converte HEX para OKLCH em pontos de referência conhecidos", () => {
      // Branco puro: L = 1, C ~ 0
      const white = hexToOklch("#ffffff");
      expect(white.l).toBeCloseTo(1, 2);
      expect(white.c).toBeCloseTo(0, 2);

      // Preto puro: L = 0, C = 0
      const black = hexToOklch("#000000");
      expect(black.l).toBeCloseTo(0, 2);
      expect(black.c).toBeCloseTo(0, 2);

      // Azul Tailwind #2563eb
      const blue = hexToOklch("#2563eb");
      expect(blue.l).toBeGreaterThan(0.5);
      expect(blue.l).toBeLessThan(0.7);
      expect(blue.c).toBeGreaterThan(0.15);
      expect(blue.h).toBeGreaterThan(240);
      expect(blue.h).toBeLessThan(280);
    });

    it("formata string CSS oklch válida", () => {
      const css = oklchToCss(0.65, 0.2, 260);
      expect(css).toBe("oklch(0.65 0.2 260)");
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

    it("calcula razões de contraste exatas nos casos fundamentais", () => {
      // Preto contra branco = 21:1
      expect(calculateContrast("#000000", "#ffffff")).toBe(21);
      expect(calculateContrast("#ffffff", "#000000")).toBe(21);

      // Branco contra branco = 1:1
      expect(calculateContrast("#ffffff", "#ffffff")).toBe(1);

      // Azul Tailwind #2563eb contra branco (#ffffff) >= 4.5:1
      const contrast = calculateContrast("#2563eb", "#ffffff");
      expect(contrast).toBeGreaterThanOrEqual(4.5);
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
