#!/usr/bin/env node
/**
 * scripts/test-check-report-snippets.js
 *
 * Teste unitário para scripts/check-report-snippets.js.
 * Valida:
 * 1. Caso de sucesso com snippet idêntico ao repositório;
 * 2. Caso de divergência (falha quando o snippet do relatório foi alterado);
 * 3. Caso de erro (caminho ou linhas inválidas).
 */

import assert from "assert";
import { checkSnippetInMarkdown } from "./check-report-snippets.js";
import { extractSnippet, formatSnippetBlock } from "./report-snippet.js";

console.log("🧪 Iniciando testes de integridade para check-report-snippets...");

// 1. Caso de Sucesso
{
  const markdown = `# Relatório de Teste\n\n${formatSnippetBlock("package.json", 1, 4, "HEAD")}\n\nFim.`;

  const results = checkSnippetInMarkdown(markdown, "teste-sucesso.md");
  assert.strictEqual(
    results.length,
    1,
    "Deveria encontrar exatamente 1 snippet",
  );
  assert.strictEqual(
    results[0].status,
    "PASS",
    "Snippet idêntico deveria ter status PASS",
  );
  console.log("  ✅ Caso 1 aprovado: Trecho exato confere com PASS");
}

// 2. Caso de Divergência (Snippet adulterado / inventado)
{
  const tamperedMarkdown = `# Relatório com código inventado
<!-- snippet: package.json:1-4 -->
\`\`\`json
{
  "name": "sistema-inventado-que-nao-existe",
  "version": "9.9.9"
}
\`\`\`
`;

  const results = checkSnippetInMarkdown(
    tamperedMarkdown,
    "teste-divergencia.md",
  );
  assert.strictEqual(results.length, 1, "Deveria encontrar 1 snippet");
  assert.strictEqual(
    results[0].status,
    "FAIL",
    "Snippet adulterado DEVE falhar com FAIL",
  );
  assert(
    results[0].error.includes("Divergência detectada"),
    "Erro deve indicar divergência",
  );
  console.log(
    "  ✅ Caso 2 aprovado: Trecho divergente detectado e reprovado com FAIL",
  );
}

// 3. Caso de Erro (Arquivo inexistente ou linhas fora do limite)
{
  const invalidMarkdown = `# Relatório com arquivo inválido
<!-- snippet: arquivo-que-nao-existe.txt:1-10 -->
\`\`\`text
conteudo
\`\`\`
`;

  const results = checkSnippetInMarkdown(invalidMarkdown, "teste-erro.md");
  assert.strictEqual(results.length, 1, "Deveria encontrar 1 snippet");
  assert.strictEqual(
    results[0].status,
    "ERROR",
    "Arquivo inexistente deve gerar status ERROR",
  );
  console.log(
    "  ✅ Caso 3 aprovado: Arquivo ou linha inválida detectado com ERROR",
  );
}

// 4. Teste de múltiplos snippets com mix de acertos e falhas
{
  const mixedMarkdown = `# Relatório Misto
<!-- snippet: package.json:1-3 -->
\`\`\`json
${extractSnippet("package.json", 1, 3, "HEAD")}
\`\`\`

<!-- snippet: package.json:1-3 -->
\`\`\`json
{
  "nome_inventado": true
}
\`\`\`
`;

  const results = checkSnippetInMarkdown(mixedMarkdown, "teste-misto.md");
  assert.strictEqual(results.length, 2);
  assert.strictEqual(results[0].status, "PASS");
  assert.strictEqual(results[1].status, "FAIL");
  console.log(
    "  ✅ Caso 4 aprovado: Relatório misto identifica acerto e erro pontualmente",
  );
}

console.log(
  "🎉 Todos os testes de scripts/check-report-snippets.js passaram com sucesso!",
);
