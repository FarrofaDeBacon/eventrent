#!/usr/bin/env node
/**
 * scripts/report-snippet.js
 *
 * Imprime ou atualiza trechos exatos de arquivos a partir do git HEAD (ou ref/hash especificado).
 * Formato do marcador em relatórios markdown:
 *   <!-- snippet: caminho/do/arquivo:inicio-fim -->
 *   ou
 *   <!-- snippet: caminho/do/arquivo:inicio-fim@hash -->
 */

import { execSync } from "child_process";
import fs from "fs";
import path from "path";

export function getFileContentAtRef(filePath, ref = "HEAD") {
  const normalizedPath = filePath.replace(/\\/g, "/");
  try {
    return execSync(`git show ${ref}:${normalizedPath}`, {
      encoding: "utf8",
      maxBuffer: 10 * 1024 * 1024,
      stdio: ["pipe", "pipe", "pipe"],
    });
  } catch (err) {
    if (fs.existsSync(normalizedPath)) {
      return fs.readFileSync(normalizedPath, "utf8");
    }
    throw new Error(
      `Não foi possível ler o arquivo '${normalizedPath}' no ref '${ref}': ${err.message}`,
    );
  }
}

export function extractSnippet(filePath, startLine, endLine, ref = "HEAD") {
  const content = getFileContentAtRef(filePath, ref);
  const normalized = content.replace(/\r\n/g, "\n");
  const lines = normalized.split("\n");

  if (startLine < 1 || startLine > lines.length) {
    throw new Error(
      `Linha inicial ${startLine} fora do intervalo (1..${lines.length}) em ${filePath}`,
    );
  }
  if (endLine < startLine || endLine > lines.length) {
    throw new Error(
      `Linha final ${endLine} inválida para intervalo (${startLine}..${lines.length}) em ${filePath}`,
    );
  }

  const selectedLines = lines.slice(startLine - 1, endLine);
  return selectedLines.join("\n");
}

export function detectLanguage(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  switch (ext) {
    case ".ts":
      return "typescript";
    case ".js":
      return "javascript";
    case ".rs":
      return "rust";
    case ".svelte":
      return "svelte";
    case ".sql":
      return "sql";
    case ".json":
      return "json";
    case ".html":
      return "html";
    case ".css":
      return "css";
    case ".sh":
      return "bash";
    default:
      return "";
  }
}

export function formatSnippetBlock(
  filePath,
  startLine,
  endLine,
  ref = "HEAD",
  langOverride = null,
) {
  const snippet = extractSnippet(filePath, startLine, endLine, ref);
  const lang = langOverride !== null ? langOverride : detectLanguage(filePath);
  const normalizedPath = filePath.replace(/\\/g, "/");
  const marker =
    ref && ref !== "HEAD"
      ? `<!-- snippet: ${normalizedPath}:${startLine}-${endLine}@${ref} -->`
      : `<!-- snippet: ${normalizedPath}:${startLine}-${endLine} -->`;

  return `<!-- prettier-ignore -->\n${marker}\n\`\`\`${lang}\n${snippet}\n\`\`\``;
}

export function updateReportSnippets(reportPath) {
  if (!fs.existsSync(reportPath)) {
    throw new Error(`Relatório não encontrado: ${reportPath}`);
  }

  const content = fs.readFileSync(reportPath, "utf8");
  const normalizedContent = content.replace(/\r\n/g, "\n");

  const markerRegex =
    /(?:<!--\s*prettier-ignore\s*-->\s*\n)?<!--\s*snippet:\s*([^:\s]+):(\d+)-(\d+)(?:@([0-9a-fA-F]+))?\s*-->\s*```([a-zA-Z0-9_-]*)\n([\s\S]*?)\n```/g;

  let replacedCount = 0;
  const updated = normalizedContent.replace(
    markerRegex,
    (match, targetPath, startStr, endStr, ref, lang) => {
      const startLine = parseInt(startStr, 10);
      const endLine = parseInt(endStr, 10);
      const targetRef = ref || "HEAD";

      try {
        const snippet = extractSnippet(
          targetPath,
          startLine,
          endLine,
          targetRef,
        );
        replacedCount++;
        const targetMarker =
          targetRef !== "HEAD"
            ? `<!-- snippet: ${targetPath}:${startLine}-${endLine}@${targetRef} -->`
            : `<!-- snippet: ${targetPath}:${startLine}-${endLine} -->`;
        return `<!-- prettier-ignore -->\n${targetMarker}\n\`\`\`${lang}\n${snippet}\n\`\`\``;
      } catch (err) {
        console.error(
          `[AVISO] Erro ao atualizar snippet para ${targetPath}:${startLine}-${endLine}: ${err.message}`,
        );
        return match;
      }
    },
  );

  fs.writeFileSync(reportPath, updated, "utf8");
  console.log(
    `✅ Atualizados ${replacedCount} snippets no relatório: ${reportPath}`,
  );
}

// Execução CLI
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) ===
    path.resolve(new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1"))
) {
  const args = process.argv.slice(2);

  if (args.includes("--update")) {
    const reportIdx = args.indexOf("--update") + 1;
    const reportPath = args[reportIdx];
    if (!reportPath) {
      console.error(
        "Uso: node scripts/report-snippet.js --update <caminho.md>",
      );
      process.exit(1);
    }
    updateReportSnippets(reportPath);
    process.exit(0);
  }

  if (args.length < 3) {
    console.error(
      "Uso: node scripts/report-snippet.js <caminho> <linhaInicio> <linhaFim> [--ref <ref>] [--raw] [--lang <lang>]",
    );
    process.exit(1);
  }

  const [filePath, startStr, endStr] = args;
  const startLine = parseInt(startStr, 10);
  const endLine = parseInt(endStr, 10);

  let ref = "HEAD";
  const refIdx = args.indexOf("--ref");
  if (refIdx !== -1 && args[refIdx + 1]) {
    ref = args[refIdx + 1];
  }

  const isRaw = args.includes("--raw");
  let langOverride = null;
  const langIdx = args.indexOf("--lang");
  if (langIdx !== -1 && args[langIdx + 1]) {
    langOverride = args[langIdx + 1];
  }

  try {
    if (isRaw) {
      const snippet = extractSnippet(filePath, startLine, endLine, ref);
      process.stdout.write(snippet + "\n");
    } else {
      const block = formatSnippetBlock(
        filePath,
        startLine,
        endLine,
        ref,
        langOverride,
      );
      process.stdout.write(block + "\n");
    }
  } catch (e) {
    console.error(`Erro: ${e.message}`);
    process.exit(1);
  }
}
