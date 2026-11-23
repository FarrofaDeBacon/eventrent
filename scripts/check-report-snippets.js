#!/usr/bin/env node
/**
 * scripts/check-report-snippets.js
 *
 * Valida a integridade estrutural de todos os trechos de código em relatórios markdown.
 * Para cada marcador:
 *   <!-- snippet: caminho:inicio-fim -->
 *   <!-- snippet: caminho:inicio-fim@hash -->
 * compara o conteúdo do bloco de código com o arquivo no HEAD (ou ref).
 * Falha (exit 1) se houver qualquer divergência.
 */

import fs from "fs";
import path from "path";
import { extractSnippet } from "./report-snippet.js";

export function checkSnippetInMarkdown(markdownContent) {
  const normalized = markdownContent.replace(/\r\n/g, "\n");
  const markerRegex =
    /(?:<!--\s*prettier-ignore\s*-->\s*\n)?<!--\s*snippet:\s*([^:\s]+):(\d+)-(\d+)(?:@([0-9a-fA-F]+))?\s*-->\s*```([a-zA-Z0-9_-]*)\n([\s\S]*?)\n```/g;

  const results = [];
  let match;

  while ((match = markerRegex.exec(normalized)) !== null) {
    const targetPath = match[1];
    const startLine = parseInt(match[2], 10);
    const endLine = parseInt(match[3], 10);
    const ref = match[4] || "HEAD";
    const reportSnippet = match[6];

    try {
      const realSnippet = extractSnippet(targetPath, startLine, endLine, ref);

      if (realSnippet === reportSnippet) {
        results.push({
          targetPath,
          startLine,
          endLine,
          ref,
          status: "PASS",
        });
      } else {
        const realLines = realSnippet.split("\n");
        const reportLines = reportSnippet.split("\n");

        results.push({
          targetPath,
          startLine,
          endLine,
          ref,
          status: "FAIL",
          error:
            `Divergência detectada no trecho ${targetPath}:${startLine}-${endLine} (@${ref}):\n` +
            `Esperado (repositório):\n${realLines.slice(0, 5).join("\n")}${realLines.length > 5 ? "\n..." : ""}\n\n` +
            `Encontrado (relatório):\n${reportLines.slice(0, 5).join("\n")}${reportLines.length > 5 ? "\n..." : ""}`,
        });
      }
    } catch (err) {
      results.push({
        targetPath,
        startLine,
        endLine,
        ref,
        status: "ERROR",
        error: `Erro ao extrair trecho de ${targetPath}:${startLine}-${endLine} (@${ref}): ${err.message}`,
      });
    }
  }

  return results;
}

export function runChecks(targetFiles = []) {
  let files = targetFiles;

  if (files.length === 0) {
    const relatoriosDir = path.resolve("docs/relatorios");
    if (fs.existsSync(relatoriosDir)) {
      files = fs
        .readdirSync(relatoriosDir)
        .filter((f) => f.endsWith(".md"))
        .map((f) => path.join(relatoriosDir, f));
    }
  }

  if (files.length === 0) {
    console.log(
      "ℹ️ Nenhum arquivo markdown encontrado para verificação de snippets.",
    );
    return 0;
  }

  let totalMarkers = 0;
  let failedCount = 0;

  console.log(
    "🔍 Verificando integridade estrutural de snippets em docs/relatorios/*.md...",
  );

  for (const file of files) {
    const content = fs.readFileSync(file, "utf8");
    const results = checkSnippetInMarkdown(content, file);

    if (results.length === 0) {
      continue;
    }

    console.log(
      `\n📄 Verificando: ${path.relative(process.cwd(), file)} (${results.length} snippets)`,
    );

    for (const res of results) {
      totalMarkers++;
      if (res.status === "PASS") {
        console.log(
          `  ✅ [PASS] ${res.targetPath}:${res.startLine}-${res.endLine} (@${res.ref})`,
        );
      } else {
        failedCount++;
        console.error(
          `  ❌ [${res.status}] ${res.targetPath}:${res.startLine}-${res.endLine} (@${res.ref})`,
        );
        console.error(`     ${res.error.replace(/\n/g, "\n     ")}`);
      }
    }
  }

  console.log("\n------------------------------------------------------------");
  if (failedCount > 0) {
    console.error(
      `❌ FALHA: ${failedCount} de ${totalMarkers} snippets apresentaram divergência ou erro.`,
    );
    return 1;
  } else {
    console.log(
      `✅ SUCESSO: Todos os ${totalMarkers} snippets verificados conferem rigorosamente com o código.`,
    );
    return 0;
  }
}

// Execução CLI
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) ===
    path.resolve(new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1"))
) {
  const specificFiles = process.argv.slice(2);
  const exitCode = runChecks(specificFiles);
  process.exit(exitCode);
}
