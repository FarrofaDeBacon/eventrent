#!/usr/bin/env node
import { spawnSync } from "child_process";

const steps = [
  { name: "SvelteKit Check & Typecheck", command: "pnpm", args: ["check"] },
  { name: "Lint de Código & Cores", command: "pnpm", args: ["lint"] },
  {
    name: "Testes Unitários Frontend (Vitest)",
    command: "pnpm",
    args: ["test:run"],
  },
  {
    name: "Build Frontend de Produção (sem mocks)",
    command: "pnpm",
    args: ["build"],
  },
  {
    name: "Verificação Estática de Ausência de Mocks no Pacote (check-no-mocks-in-build)",
    command: "node",
    args: ["scripts/check-no-mocks-in-build.js"],
  },
  {
    name: "Prova Estrutural de Congelamento Dinâmico de Migrações",
    command: "node",
    args: ["scripts/test-check-migrations.js"],
  },
  {
    name: "Prova Estrutural de Trechos de Código em Relatórios (test-check-report-snippets)",
    command: "node",
    args: ["scripts/test-check-report-snippets.js"],
  },
  {
    name: "Verificação Estrutural de Snippets em Relatórios (check-report-snippets)",
    command: "node",
    args: ["scripts/check-report-snippets.js"],
  },
  {
    name: "Rustfmt Check",
    command: "cargo",
    args: ["fmt", "--all", "--", "--check"],
  },
  {
    name: "Clippy Linter (Workspace)",
    command: "cargo",
    args: ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
  },
  {
    name: "Clippy Linter (eventrent-core sem unificação)",
    command: "cargo",
    args: ["clippy", "-p", "eventrent-core", "--", "-D", "warnings"],
  },
  {
    name: "Clippy Linter (eventrent-store-sqlite sem unificação)",
    command: "cargo",
    args: ["clippy", "-p", "eventrent-store-sqlite", "--", "-D", "warnings"],
  },
  {
    name: "Clippy Linter (eventrent-app sem unificação)",
    command: "cargo",
    args: ["clippy", "-p", "eventrent-app", "--", "-D", "warnings"],
  },
  {
    name: "Testes Unitários Rust Workspace",
    command: "cargo",
    args: ["test", "--workspace"],
  },
];

console.log("🛡️  Iniciando Portão de Qualidade do EventRent (verify)...");
const startTime = Date.now();

for (let i = 0; i < steps.length; i++) {
  const step = steps[i];
  console.log(`\n[${i + 1}/${steps.length}] Executando: ${step.name}...`);
  const result = spawnSync(step.command, step.args, {
    stdio: "inherit",
    shell: true,
  });

  if (result.status !== 0) {
    console.error(
      `\n❌ FALHA no passo: ${step.name} (código de saída: ${result.status})`,
    );
    process.exit(result.status || 1);
  }
}

const elapsedSec = ((Date.now() - startTime) / 1000).toFixed(1);
console.log(
  `\n✅ Todos os ${steps.length} passos de verificação passaram com sucesso em ${elapsedSec}s!`,
);
process.exit(0);
