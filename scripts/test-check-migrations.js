#!/usr/bin/env node
import { execSync } from "child_process";
import fs from "fs";
import os from "os";
import path from "path";

function run(cmd, cwd) {
  return execSync(cmd, { cwd, encoding: "utf8", stdio: "pipe" });
}

function checkMigrations(cwd, baseRef = "main") {
  try {
    const diff = run(`git diff --name-only ${baseRef}...HEAD`, cwd).trim();
    const lines = diff ? diff.split(/\r?\n/) : [];
    const modifiedFrozen = lines.filter((file) =>
      /^crates\/store-sqlite\/migrations\/000[12]_/.test(file),
    );

    if (modifiedFrozen.length > 0) {
      return {
        passed: false,
        output: `FALHA DETECTADA: As seguintes migrações congeladas (0001/0002) foram alteradas/removidas:\n${modifiedFrozen.map((f) => "  - " + f).join("\n")}`,
      };
    }

    return {
      passed: true,
      output:
        "SUCESSO: Nenhuma migração congelada (0001 ou 0002) foi alterada.",
    };
  } catch (err) {
    return {
      passed: false,
      output: `ERRO DE EXECUÇÃO: ${err.message}`,
    };
  }
}

console.log(
  "🧪 Iniciando Prova Estrutural do Congelamento de Migrações (Item 0.8)...",
);

const tmpDir = fs.mkdtempSync(
  path.join(os.tmpdir(), "eventrent-migration-check-"),
);

try {
  run("git init -b main", tmpDir);
  run('git config user.name "Test Runner"', tmpDir);
  run('git config user.email "test@eventrent.local"', tmpDir);

  const migDir = path.join(tmpDir, "crates", "store-sqlite", "migrations");
  fs.mkdirSync(migDir, { recursive: true });

  const mig1 = path.join(migDir, "0001_initial.sql");
  const mig2 = path.join(migDir, "0002_user_lockout.sql");
  fs.writeFileSync(mig1, "-- 0001 initial schema\nCREATE TABLE app_user ();\n");
  fs.writeFileSync(
    mig2,
    "-- 0002 lockout\nALTER TABLE app_user ADD COLUMN failed_attempts INTEGER;\n",
  );

  run("git add .", tmpDir);
  run('git commit -m "chore: initial migrations 0001 and 0002"', tmpDir);

  // ==========================================
  // CENÁRIO 1: Alterar migração existente -> DEVE FALHAR
  // ==========================================
  console.log("\n--- CENÁRIO 1: Alterar migração existente (0001) ---");
  run("git checkout -b branch-cenario-1", tmpDir);
  fs.appendFileSync(mig1, "-- alteracao indevida na migracao congelada\n");
  run("git add .", tmpDir);
  run('git commit -m "test: modifica 0001 indevidamente"', tmpDir);

  const res1 = checkMigrations(tmpDir, "main");
  console.log(`Resultado: ${res1.passed ? "PASSA" : "FALHA"}`);
  console.log(res1.output);
  if (res1.passed) {
    throw new Error("Cenário 1 deveria ter FALHADO, mas passou!");
  }

  // ==========================================
  // CENÁRIO 2: Adicionar migração nova (0003) -> DEVE PASSAR
  // ==========================================
  console.log("\n--- CENÁRIO 2: Adicionar migração nova (0003) ---");
  run("git checkout main", tmpDir);
  run("git checkout -b branch-cenario-2", tmpDir);
  const mig3 = path.join(migDir, "0003_branding_dark_and_icon.sql");
  fs.writeFileSync(
    mig3,
    "-- 0003 nova migracao permitida\nALTER TABLE tenant_branding ADD COLUMN logo_dark_url TEXT;\n",
  );
  run("git add .", tmpDir);
  run('git commit -m "feat: adiciona migracao nova 0003"', tmpDir);

  const res2 = checkMigrations(tmpDir, "main");
  console.log(`Resultado: ${res2.passed ? "PASSA" : "FALHA"}`);
  console.log(res2.output);
  if (!res2.passed) {
    throw new Error("Cenário 2 deveria ter PASSADO, mas falhou!");
  }

  // ==========================================
  // CENÁRIO 3: Renomear ou apagar migração (0002) -> DEVE FALHAR
  // ==========================================
  console.log(
    "\n--- CENÁRIO 3: Apagar ou renomear migração existente (0002) ---",
  );
  run("git checkout main", tmpDir);
  run("git checkout -b branch-cenario-3", tmpDir);
  fs.unlinkSync(mig2);
  run("git add -u", tmpDir);
  run('git commit -m "test: apaga 0002 indevidamente"', tmpDir);

  const res3 = checkMigrations(tmpDir, "main");
  console.log(`Resultado: ${res3.passed ? "PASSA" : "FALHA"}`);
  console.log(res3.output);
  if (res3.passed) {
    throw new Error("Cenário 3 deveria ter FALHADO, mas passou!");
  }

  console.log(
    "\n✅ Todos os 3 cenários de prova de congelamento foram validados com 100% de sucesso!",
  );
} finally {
  fs.rmSync(tmpDir, { recursive: true, force: true });
}
