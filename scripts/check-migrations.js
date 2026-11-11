#!/usr/bin/env node
import { execSync } from "child_process";

export function checkFrozenMigrations(
  cwd = process.cwd(),
  baseRef = "origin/main",
) {
  try {
    // 1. Obter lista de arquivos de migração que já existem na branch base (main)
    let baseFilesRaw = "";
    try {
      baseFilesRaw = execSync(
        `git ls-tree -r --name-only ${baseRef} -- crates/store-sqlite/migrations`,
        { cwd, encoding: "utf8", stdio: "pipe" },
      ).trim();
    } catch {
      // Fallback para 'main' local caso 'origin/main' não esteja configurado
      const fallbackRef = baseRef.replace(/^origin\//, "");
      baseFilesRaw = execSync(
        `git ls-tree -r --name-only ${fallbackRef} -- crates/store-sqlite/migrations`,
        { cwd, encoding: "utf8", stdio: "pipe" },
      ).trim();
      baseRef = fallbackRef;
    }

    const baseMigrationFiles = new Set(
      baseFilesRaw ? baseFilesRaw.split(/\r?\n/).map((f) => f.trim()) : [],
    );

    // 2. Obter o status de alterações no diretório de migrações em relação à base
    const diffStatusRaw = execSync(
      `git diff --name-status ${baseRef}...HEAD -- crates/store-sqlite/migrations`,
      { cwd, encoding: "utf8", stdio: "pipe" },
    ).trim();

    const diffLines = diffStatusRaw ? diffStatusRaw.split(/\r?\n/) : [];
    const violations = [];
    const added = [];

    for (const line of diffLines) {
      const parts = line.split(/\t+/);
      const statusCode = parts[0].trim();
      const filePath = parts[1] ? parts[1].trim() : "";

      if (statusCode.startsWith("A")) {
        // Arquivo novo adicionado
        added.push(filePath);
      } else if (
        statusCode.startsWith("M") ||
        statusCode.startsWith("D") ||
        statusCode.startsWith("R") ||
        statusCode.startsWith("C")
      ) {
        // Modificado, deletado ou renomeado
        const desc = statusCode.startsWith("M")
          ? "MODIFICADA"
          : statusCode.startsWith("D")
            ? "REMOVIDA"
            : "RENOMEADA";
        violations.push({ file: filePath, reason: desc });
      }
    }

    // 3. Checar também se alguma migração existente na base foi removida ou não consta no HEAD
    for (const baseFile of baseMigrationFiles) {
      if (!diffLines.some((l) => l.includes(baseFile))) {
        // Permaneceu intocada, correto
        continue;
      }
    }

    if (violations.length > 0) {
      const details = violations
        .map((v) => `  - [${v.reason}] ${v.file}`)
        .join("\n");
      return {
        passed: false,
        output: `FALHA DETECTADA: As seguintes migrações congeladas existentes na ${baseRef} foram alteradas:\n${details}\nNenhuma migração já incorporada à branch principal pode ser alterada. Crie uma nova migração sequencial para alterações de esquema.`,
      };
    }

    return {
      passed: true,
      output: `SUCESSO: Todas as migrações existentes na base ${baseRef} estão intactas.${added.length > 0 ? `\nNovas migrações adicionadas:\n${added.map((f) => "  + " + f).join("\n")}` : " Nenhuma migração adicionada."}`,
    };
  } catch (err) {
    return {
      passed: false,
      output: `ERRO DE EXECUÇÃO na checagem de migrações: ${err.message}`,
    };
  }
}

// Execução direta como script de CI / CLI
if (import.meta.url === `file://${process.argv[1].replace(/\\/g, "/")}`) {
  const baseRef = process.argv[2] || "origin/main";
  const result = checkFrozenMigrations(process.cwd(), baseRef);
  console.log(result.output);
  if (!result.passed) {
    process.exit(1);
  }
}
