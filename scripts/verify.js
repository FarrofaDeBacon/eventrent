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
    name: "Prova de Congelamento das Migrações (0001/0002)",
    command: "node",
    args: ["scripts/test-check-migrations.js"],
  },
  {
    name: "Rustfmt Check",
    command: "cargo",
    args: ["fmt", "--all", "--", "--check"],
  },
  {
    name: "Clippy Linter",
    command: "cargo",
    args: ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
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
