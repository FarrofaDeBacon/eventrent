#!/usr/bin/env node
import { execFileSync } from "child_process";
import path from "path";

function runGit(args, cwd) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    stdio: "pipe",
  }).trim();
}

function parseMigrationVersion(filePath) {
  const baseName = path.posix.basename(filePath.replace(/\\/g, "/"));
  const match = /^(\d+)_.+\.sql$/.exec(baseName);
  if (!match) return null;
  return Number.parseInt(match[1], 10);
}

export function checkFrozenMigrations(
  cwd = process.cwd(),
  baseRef = "origin/main",
) {
  try {
    // 1. Obter lista de arquivos de migração que já existem na branch base (main)
    let baseFilesRaw = "";
    try {
      baseFilesRaw = runGit(
        ["ls-tree", "-r", "--name-only", baseRef, "--", "crates/*/migrations"],
        cwd,
      );
    } catch {
      const fallbackRef = baseRef.replace(/^origin\//, "");
      baseFilesRaw = runGit(
        [
          "ls-tree",
          "-r",
          "--name-only",
          fallbackRef,
          "--",
          "crates/*/migrations",
        ],
        cwd,
      );
      baseRef = fallbackRef;
    }

    const baseMigrationFiles = baseFilesRaw
      ? baseFilesRaw.split(/\r?\n/).map((f) => f.trim())
      : [];

    // Calcula a maior versão existente na base por diretório de migrações
    const maxVersionByDir = new Map();
    const baseVersionsByDir = new Map();
    for (const filePath of baseMigrationFiles) {
      const norm = filePath.replace(/\\/g, "/");
      const dir = path.posix.dirname(norm);
      const version = parseMigrationVersion(norm);
      if (version === null) continue;
      const currentMax = maxVersionByDir.get(dir) ?? 0;
      if (version > currentMax) maxVersionByDir.set(dir, version);
      if (!baseVersionsByDir.has(dir)) baseVersionsByDir.set(dir, new Set());
      baseVersionsByDir.get(dir).add(version);
    }

    // 2. Obter alterações nos diretórios crates/*/migrations em relação à base
    const diffStatusRaw = runGit(
      [
        "diff",
        "--name-status",
        `${baseRef}...HEAD`,
        "--",
        "crates/*/migrations",
      ],
      cwd,
    );

    const diffLines = diffStatusRaw ? diffStatusRaw.split(/\r?\n/) : [];
    const violations = [];
    const added = [];

    for (const line of diffLines) {
      const parts = line.split(/\t+/);
      const statusCode = parts[0].trim();
      const filePath = parts[1] ? parts[1].trim().replace(/\\/g, "/") : "";

      if (statusCode.startsWith("A")) {
        added.push(filePath);
      } else if (
        statusCode.startsWith("M") ||
        statusCode.startsWith("D") ||
        statusCode.startsWith("R") ||
        statusCode.startsWith("C")
      ) {
        const desc = statusCode.startsWith("M")
          ? "MODIFICADA"
          : statusCode.startsWith("D")
            ? "REMOVIDA"
            : "RENOMEADA";
        violations.push({ file: filePath, reason: desc });
      }
    }

    // 3. Validar que cada migração nova tem versão única e estritamente maior que a maior da base
    const seenNewVersionsByDir = new Map();
    for (const newFile of added) {
      const dir = path.posix.dirname(newFile);
      const version = parseMigrationVersion(newFile);
      if (version === null) {
        violations.push({
          file: newFile,
          reason: "NOME INVÁLIDO (esperado NNNN_descricao.sql)",
        });
        continue;
      }

      const maxBase = maxVersionByDir.get(dir) ?? 0;
      const baseVersions = baseVersionsByDir.get(dir) ?? new Set();
      if (!seenNewVersionsByDir.has(dir)) {
        seenNewVersionsByDir.set(dir, new Set());
      }
      const seenNew = seenNewVersionsByDir.get(dir);

      if (baseVersions.has(version) || seenNew.has(version)) {
        violations.push({
          file: newFile,
          reason: `VERSÃO DUPLICADA (${String(version).padStart(4, "0")})`,
        });
      } else if (version <= maxBase) {
        violations.push({
          file: newFile,
          reason: `VERSÃO MENOR OU IGUAL À MAIOR DA BASE (${String(version).padStart(4, "0")} <= ${String(maxBase).padStart(4, "0")})`,
        });
      }

      seenNew.add(version);
    }

    if (violations.length > 0) {
      const details = violations
        .map((v) => `  - [${v.reason}] ${v.file}`)
        .join("\n");
      return {
        passed: false,
        output: `FALHA DETECTADA: Violações de congelamento/versionamento de migrações em relação à ${baseRef}:\n${details}\nNenhuma migração incorporada à main pode ser alterada e novas migrações devem ter versão única e maior que a maior da base.`,
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
