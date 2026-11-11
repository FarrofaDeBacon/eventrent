#!/usr/bin/env node
import { execSync } from "child_process";
import fs from "fs";
import os from "os";
import path from "path";
import { checkFrozenMigrations } from "./check-migrations.js";

function run(cmd, cwd) {
  return execSync(cmd, { cwd, encoding: "utf8", stdio: "pipe" });
}

console.log(
  "🧪 Iniciando Prova Estrutural do Congelamento Dinâmico de Migrações (Item 0.1)...",
);

const tmpDir = fs.mkdtempSync(
  path.join(os.tmpdir(), "eventrent-migration-freeze-test-"),
);

try {
  run("git init -b main", tmpDir);
  run('git config user.name "Test Runner"', tmpDir);
  run('git config user.email "test@eventrent.local"', tmpDir);

  const migDir = path.join(tmpDir, "crates", "store-sqlite", "migrations");
  fs.mkdirSync(migDir, { recursive: true });

  // Cria as migrações REAIS presentes no repositório EventRent
  const mig1 = path.join(migDir, "0001_init_tenant_and_branding.sql");
  const mig2 = path.join(migDir, "0002_users_roles_and_audit.sql");
  const mig3 = path.join(migDir, "0003_branding_dark_logo_and_icon.sql");

  fs.writeFileSync(
    mig1,
    "-- 0001_init_tenant_and_branding.sql\nCREATE TABLE tenant_branding (id TEXT PRIMARY KEY);\n",
  );
  fs.writeFileSync(
    mig2,
    "-- 0002_users_roles_and_audit.sql\nCREATE TABLE app_user (id TEXT PRIMARY KEY);\nCREATE TABLE audit_log (id TEXT PRIMARY KEY);\n",
  );
  fs.writeFileSync(
    mig3,
    "-- 0003_branding_dark_logo_and_icon.sql\nALTER TABLE tenant_branding ADD COLUMN logo_dark_url TEXT;\nALTER TABLE tenant_branding ADD COLUMN icon_url TEXT;\n",
  );

  run("git add .", tmpDir);
  run(
    'git commit -m "chore: base migrations 0001, 0002 and 0003 already on main"',
    tmpDir,
  );

  // ==========================================
  // CENÁRIO 1: Alterar migração existente 0001 -> DEVE FALHAR
  // ==========================================
  console.log(
    "\n--- CENÁRIO 1: Alterar migração existente na main (0001_init_tenant_and_branding.sql) ---",
  );
  run("git checkout -b branch-cenario-1", tmpDir);
  fs.appendFileSync(mig1, "-- alteracao indevida na migracao congelada 0001\n");
  run("git add .", tmpDir);
  run('git commit -m "test: modifica 0001 indevidamente"', tmpDir);

  const res1 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res1.passed ? "PASSA" : "FALHA"}`);
  console.log(res1.output);
  if (res1.passed) {
    throw new Error("Cenário 1 deveria ter FALHADO, mas passou!");
  }

  // ==========================================
  // CENÁRIO 2: Alterar migração recente 0003 (já incorporada à main) -> DEVE FALHAR
  // ==========================================
  console.log(
    "\n--- CENÁRIO 2: Alterar migração congelada na main (0003_branding_dark_logo_and_icon.sql) ---",
  );
  run("git checkout main", tmpDir);
  run("git checkout -b branch-cenario-2", tmpDir);
  fs.appendFileSync(
    mig3,
    "-- alteracao indevida na migracao 0003 incorporada\n",
  );
  run("git add .", tmpDir);
  run('git commit -m "test: modifica 0003 indevidamente"', tmpDir);

  const res2 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res2.passed ? "PASSA" : "FALHA"}`);
  console.log(res2.output);
  if (res2.passed) {
    throw new Error(
      "Cenário 2 deveria ter FALHADO ao alterar a 0003, mas passou!",
    );
  }

  // ==========================================
  // CENÁRIO 3: Apagar migração existente 0002 -> DEVE FALHAR
  // ==========================================
  console.log(
    "\n--- CENÁRIO 3: Apagar migração existente na main (0002_users_roles_and_audit.sql) ---",
  );
  run("git checkout main", tmpDir);
  run("git checkout -b branch-cenario-3", tmpDir);
  fs.unlinkSync(mig2);
  run("git add -u", tmpDir);
  run('git commit -m "test: remove 0002 indevidamente"', tmpDir);

  const res3 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res3.passed ? "PASSA" : "FALHA"}`);
  console.log(res3.output);
  if (res3.passed) {
    throw new Error("Cenário 3 deveria ter FALHADO, mas passou!");
  }

  // ==========================================
  // CENÁRIO 4: Renomear migração existente 0002 -> DEVE FALHAR
  // ==========================================
  console.log(
    "\n--- CENÁRIO 4: Renomear migração existente na main (0002_users_roles_and_audit.sql) ---",
  );
  run("git checkout main", tmpDir);
  run("git checkout -b branch-cenario-4", tmpDir);
  const mig2Renamed = path.join(migDir, "0002_users_renamed.sql");
  fs.renameSync(mig2, mig2Renamed);
  run("git add -A", tmpDir);
  run('git commit -m "test: renomeia 0002 indevidamente"', tmpDir);

  const res4 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res4.passed ? "PASSA" : "FALHA"}`);
  console.log(res4.output);
  if (res4.passed) {
    throw new Error("Cenário 4 deveria ter FALHADO, mas passou!");
  }

  // ==========================================
  // CENÁRIO 5: Adicionar migração nova sequencial (0004_customers.sql) -> DEVE PASSAR
  // ==========================================
  console.log(
    "\n--- CENÁRIO 5: Adicionar migração nova sequencial (0004_customers.sql) ---",
  );
  run("git checkout main", tmpDir);
  run("git checkout -b branch-cenario-5", tmpDir);
  const mig4 = path.join(migDir, "0004_customers.sql");
  fs.writeFileSync(
    mig4,
    "-- 0004_customers.sql\nCREATE TABLE customer (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL);\n",
  );
  run("git add .", tmpDir);
  run('git commit -m "feat: adiciona nova migracao 0004"', tmpDir);

  const res5 = checkFrozenMigrations(tmpDir, "main");
  console.log(`Resultado: ${res5.passed ? "PASSA" : "FALHA"}`);
  console.log(res5.output);
  if (!res5.passed) {
    throw new Error(
      "Cenário 5 deveria ter PASSADO ao adicionar 0004, mas falhou!",
    );
  }

  console.log(
    "\n✅ Todos os 5 cenários de prova de congelamento foram validados com 100% de sucesso!",
  );
} finally {
  fs.rmSync(tmpDir, { recursive: true, force: true });
}
