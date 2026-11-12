#!/usr/bin/env node
import { execFileSync } from "child_process";
import fs from "fs";
import os from "os";
import path from "path";
import { checkFrozenMigrations } from "./check-migrations.js";

function runGit(args, cwd) {
  return execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe" });
}

console.log(
  "🧪 Iniciando Prova Estrutural do Congelamento e Versionamento de Migrações (R11)...",
);

const tmpDir = fs.mkdtempSync(
  path.join(os.tmpdir(), "eventrent-migration-freeze-test-"),
);

try {
  runGit(["init", "-b", "main"], tmpDir);
  runGit(["config", "user.name", "Test Runner"], tmpDir);
  runGit(["config", "user.email", "test@eventrent.local"], tmpDir);

  const migDir = path.join(tmpDir, "crates", "store-sqlite", "migrations");
  fs.mkdirSync(migDir, { recursive: true });

  // Cria as migrações REAIS 0001..0005 na base main
  const mig1 = path.join(migDir, "0001_init_tenant_and_branding.sql");
  const mig2 = path.join(migDir, "0002_users_roles_and_audit.sql");
  const mig3 = path.join(migDir, "0003_branding_dark_logo_and_icon.sql");
  const mig4 = path.join(migDir, "0004_customers.sql");
  const mig5 = path.join(migDir, "0005_customers_rev.sql");

  fs.writeFileSync(
    mig1,
    "-- 0001_init_tenant_and_branding.sql\nCREATE TABLE tenant_branding (id TEXT PRIMARY KEY);\n",
  );
  fs.writeFileSync(
    mig2,
    "-- 0002_users_roles_and_audit.sql\nCREATE TABLE app_user (id TEXT PRIMARY KEY);\n",
  );
  fs.writeFileSync(
    mig3,
    "-- 0003_branding_dark_logo_and_icon.sql\nALTER TABLE tenant_branding ADD COLUMN logo_dark_url TEXT;\n",
  );
  fs.writeFileSync(
    mig4,
    "-- 0004_customers.sql\nCREATE TABLE customer (id TEXT PRIMARY KEY);\n",
  );
  fs.writeFileSync(
    mig5,
    "-- 0005_customers_rev.sql\nALTER TABLE customer ADD COLUMN active INTEGER NOT NULL DEFAULT 1;\n",
  );

  runGit(["add", "."], tmpDir);
  runGit(["commit", "-m", "chore: base migrations 0001..0005 on main"], tmpDir);

  // CENÁRIO 1: Alterar 0003 existente na main -> DEVE FALHAR
  console.log(
    "\n--- CENÁRIO 1: Alterar migração congelada na main (0003_branding_dark_logo_and_icon.sql) ---",
  );
  runGit(["checkout", "-b", "branch-cenario-1"], tmpDir);
  fs.appendFileSync(mig3, "-- alteracao indevida na migracao 0003\n");
  runGit(["add", "."], tmpDir);
  runGit(["commit", "-m", "test: modifica 0003 indevidamente"], tmpDir);

  const res1 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res1.passed ? "PASSA" : "FALHA"}\n${res1.output}`);
  if (res1.passed) throw new Error("Cenário 1 (alterar 0003) deveria falhar!");

  // CENÁRIO 2: Apagar 0002 existente -> DEVE FALHAR
  console.log(
    "\n--- CENÁRIO 2: Apagar migração existente na main (0002_users_roles_and_audit.sql) ---",
  );
  runGit(["checkout", "main"], tmpDir);
  runGit(["checkout", "-b", "branch-cenario-2"], tmpDir);
  fs.unlinkSync(mig2);
  runGit(["add", "-u"], tmpDir);
  runGit(["commit", "-m", "test: remove 0002 indevidamente"], tmpDir);

  const res2 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res2.passed ? "PASSA" : "FALHA"}\n${res2.output}`);
  if (res2.passed) throw new Error("Cenário 2 (apagar 0002) deveria falhar!");

  // CENÁRIO 3: Renomear 0002 existente -> DEVE FALHAR
  console.log(
    "\n--- CENÁRIO 3: Renomear migração existente na main (0002_users_roles_and_audit.sql) ---",
  );
  runGit(["checkout", "main"], tmpDir);
  runGit(["checkout", "-b", "branch-cenario-3"], tmpDir);
  fs.renameSync(mig2, path.join(migDir, "0002_users_renamed.sql"));
  runGit(["add", "-A"], tmpDir);
  runGit(["commit", "-m", "test: renomeia 0002 indevidamente"], tmpDir);

  const res3 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res3.passed ? "PASSA" : "FALHA"}\n${res3.output}`);
  if (res3.passed) throw new Error("Cenário 3 (renomear 0002) deveria falhar!");

  // CENÁRIO 4: Versão duplicada (0005_duplicada.sql) -> DEVE FALHAR
  console.log(
    "\n--- CENÁRIO 4: Adicionar migração com versão duplicada (0005_duplicada.sql) ---",
  );
  runGit(["checkout", "main"], tmpDir);
  runGit(["checkout", "-b", "branch-cenario-4"], tmpDir);
  fs.writeFileSync(
    path.join(migDir, "0005_duplicada.sql"),
    "-- versao duplicada\n",
  );
  runGit(["add", "."], tmpDir);
  runGit(["commit", "-m", "test: versao duplicada 0005"], tmpDir);

  const res4 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res4.passed ? "PASSA" : "FALHA"}\n${res4.output}`);
  if (res4.passed)
    throw new Error("Cenário 4 (versão duplicada 0005) deveria falhar!");

  // CENÁRIO 5: Versão menor que a maior existente na base (0003_retroativa.sql) -> DEVE FALHAR
  console.log(
    "\n--- CENÁRIO 5: Adicionar migração com versão menor que a maior da base (0003_retroativa.sql) ---",
  );
  runGit(["checkout", "main"], tmpDir);
  runGit(["checkout", "-b", "branch-cenario-5"], tmpDir);
  fs.writeFileSync(
    path.join(migDir, "0003_retroativa.sql"),
    "-- versao menor que 0005\n",
  );
  runGit(["add", "."], tmpDir);
  runGit(["commit", "-m", "test: versao menor que 0005"], tmpDir);

  const res5 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res5.passed ? "PASSA" : "FALHA"}\n${res5.output}`);
  if (res5.passed)
    throw new Error("Cenário 5 (versão menor que 0005) deveria falhar!");

  // CENÁRIO 6: Adicionar 0006 válida -> DEVE PASSAR
  console.log(
    "\n--- CENÁRIO 6: Adicionar migração nova sequencial válida (0006_catalog.sql) ---",
  );
  runGit(["checkout", "main"], tmpDir);
  runGit(["checkout", "-b", "branch-cenario-6"], tmpDir);
  fs.writeFileSync(
    path.join(migDir, "0006_catalog.sql"),
    "-- 0006_catalog.sql\nCREATE TABLE catalog_item (id TEXT PRIMARY KEY);\n",
  );
  runGit(["add", "."], tmpDir);
  runGit(["commit", "-m", "feat: adiciona migracao valida 0006"], tmpDir);

  const res6 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res6.passed ? "PASSA" : "FALHA"}\n${res6.output}`);
  if (!res6.passed)
    throw new Error("Cenário 6 (0006 válida) deveria ter passado!");

  console.log(
    "\n✅ Todos os 6 cenários de congelamento e versionamento de migrações passaram com 100% de sucesso!",
  );
} finally {
  fs.rmSync(tmpDir, { recursive: true, force: true });
}
