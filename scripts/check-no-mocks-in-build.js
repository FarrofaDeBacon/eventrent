import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const FORBIDDEN_STRINGS = [
  "mock_hash_",
  "MODO DE DEMONSTRAÇÃO",
  "MockAuthRepository",
  "MockUserRepository",
  "MockBrandingRepository",
  "MockCustomerRepository",
  "MockVenueRepository",
  "MockCatalogRepository",
  "MockSettingsRepository",
];

const buildDir = path.resolve(process.cwd(), "build");

if (!fs.existsSync(buildDir)) {
  console.error(`❌ Diretório de build não encontrado em: ${buildDir}`);
  console.error("Execute 'pnpm build' antes de executar este script.");
  process.exit(1);
}

function scanDir(dir) {
  let files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files = files.concat(scanDir(fullPath));
    } else if (
      entry.isFile() &&
      (entry.name.endsWith(".js") || entry.name.endsWith(".html"))
    ) {
      files.push(fullPath);
    }
  }
  return files;
}

const filesToScan = scanDir(buildDir);
console.log(
  `🔍 Verificando ausência de mocks e artefatos de demo em ${filesToScan.length} arquivos no build de produção...`,
);

const violations = [];

for (const filePath of filesToScan) {
  const content = fs.readFileSync(filePath, "utf-8");
  const relPath = path.relative(process.cwd(), filePath);

  for (const forbidden of FORBIDDEN_STRINGS) {
    if (content.includes(forbidden)) {
      violations.push({
        file: relPath,
        forbidden,
      });
    }
  }
}

if (violations.length > 0) {
  console.error(
    `❌ FALHA: Foram encontradas ${violations.length} ocorrências de mocks ou artefatos de demo no pacote de produção:`,
  );
  for (const v of violations) {
    console.error(`  - [${v.forbidden}] em ${v.file}`);
  }
  console.error(
    "Garanta que os mocks e banners sejam completamente eliminados por tree-shaking no build de produção.",
  );
  process.exit(1);
}

console.log(
  "✅ SUCESSO: Nenhum mock, hash de teste ou banner de demonstração encontrado no pacote de produção.",
);
process.exit(0);
