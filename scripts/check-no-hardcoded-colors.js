#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "..");
const srcDir = path.join(rootDir, "src");

// Proíbe cores literais (hex, rgb, hsl, oklch) em componentes .svelte.
// Todas as cores devem vir de tokens CSS semânticos (--primary, --border, etc.)
// definidos em src/lib/theme/tokens.ts e src/app.css.

const HEX_COLOR_REGEX =
  /(?:bg|text|border|fill|stroke|ring)-\[#([0-9a-fA-F]{3,8})\]/g;
const INLINE_STYLE_COLOR_REGEX =
  /style\s*=\s*["'][^"']*(?:#[0-9a-fA-F]{3,8}|(?:rgb|rgba|hsl|hsla|oklch)\([^)]+\))[^"']*["']/g;

function findSvelteFiles(dir, fileList = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "node_modules" && entry.name !== ".svelte-kit") {
        findSvelteFiles(fullPath, fileList);
      }
    } else if (entry.name.endsWith(".svelte")) {
      fileList.push(fullPath);
    }
  }
  return fileList;
}

const svelteFiles = findSvelteFiles(srcDir);
const violations = [];

for (const filePath of svelteFiles) {
  const relPath = path.relative(rootDir, filePath).replace(/\\/g, "/");
  const content = fs.readFileSync(filePath, "utf-8");
  const lines = content.split("\n");

  lines.forEach((line, index) => {
    const lineNum = index + 1;
    const trimmed = line.trim();

    // Ignora comentários HTML e JS
    if (trimmed.startsWith("<!--") || trimmed.startsWith("//")) return;

    // 1. Proibição de classes Tailwind com cores literais: bg-[#...], text-[#...], etc.
    let match;
    while ((match = HEX_COLOR_REGEX.exec(line)) !== null) {
      violations.push({
        file: relPath,
        line: lineNum,
        found: match[0],
        rule: "Classe Tailwind arbitrária com cor literal não permitida. Use tokens semânticos (ex: bg-primary).",
      });
    }

    // 2. Proibição de style inline com cores hex/rgb/hsl literais
    while ((match = INLINE_STYLE_COLOR_REGEX.exec(line)) !== null) {
      // Permite bindings dinâmicos ou variáveis css (ex: style="--custom: ...")
      if (!match[0].includes("var(--")) {
        violations.push({
          file: relPath,
          line: lineNum,
          found: match[0],
          rule: "Estilo inline com cor literal fixa não permitido. Use variáveis CSS semânticas.",
        });
      }
    }

    // 3. Proibição de tags <style> com cores literais
    if (
      line.includes("color:") ||
      line.includes("background-color:") ||
      line.includes("border-color:")
    ) {
      if (
        /#([0-9a-fA-F]{3,8})\b/.test(line) &&
        !line.includes("var(") &&
        !line.includes("/*")
      ) {
        violations.push({
          file: relPath,
          line: lineNum,
          found: line.trim(),
          rule: "Declaração CSS com cor literal. Utilize var(--...) do design system.",
        });
      }
    }
  });
}

if (violations.length > 0) {
  console.error(
    `\n❌ Encontradas ${violations.length} violações de cores literais em arquivos .svelte:\n`,
  );
  for (const v of violations) {
    console.error(`  - ${v.file}:${v.line} -> ${v.found}`);
    console.error(`    Regra: ${v.rule}\n`);
  }
  process.exit(1);
} else {
  console.log(
    `✓ Verificação concluída: 0 cores literais em ${svelteFiles.length} componentes .svelte. Conformidade total com o design system semântico.`,
  );
  process.exit(0);
}
