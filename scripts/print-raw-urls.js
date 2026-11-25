#!/usr/bin/env node
/**
 * scripts/print-raw-urls.js
 *
 * Recebe caminhos de arquivos relativos à raiz do repositório,
 * obtém o hash completo de 40 caracteres do HEAD via "git rev-parse HEAD",
 * monta as URLs raw do GitHub, faz requisição HTTP real para cada uma,
 * valida que o status é 200 (com retries para eventual propagação de CDN)
 * e imprime a lista de URLs validadas.
 *
 * Falha com código 1 se qualquer URL não responder 200.
 */

import { execSync } from "child_process";
import https from "https";

const REPO_OWNER = "FarrofaDeBacon";
const REPO_NAME = "eventrent";

function getFullHeadHash() {
  try {
    return execSync("git rev-parse HEAD", { encoding: "utf8" }).trim();
  } catch (err) {
    console.error("Erro ao obter hash completo do HEAD via git:", err.message);
    process.exit(1);
  }
}

function checkUrl(url, maxRetries = 3, delayMs = 1500) {
  return new Promise((resolve) => {
    let attempt = 0;

    function tryFetch() {
      attempt++;
      const req = https.get(url, (res) => {
        const statusCode = res.statusCode || 0;
        res.resume(); // drena stream

        if (statusCode === 200) {
          resolve({ url, ok: true, status: statusCode });
        } else if (attempt < maxRetries && statusCode === 404) {
          // CDN pode demorar alguns instantes após o push
          setTimeout(tryFetch, delayMs);
        } else {
          resolve({
            url,
            ok: false,
            status: statusCode,
            error: `Status HTTP ${statusCode}`,
          });
        }
      });

      req.on("error", (err) => {
        if (attempt < maxRetries) {
          setTimeout(tryFetch, delayMs);
        } else {
          resolve({ url, ok: false, status: 0, error: err.message });
        }
      });

      req.setTimeout(10000, () => {
        req.destroy();
        if (attempt < maxRetries) {
          setTimeout(tryFetch, delayMs);
        } else {
          resolve({ url, ok: false, status: 0, error: "Timeout de rede" });
        }
      });
    }

    tryFetch();
  });
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length === 0) {
    console.error(
      "Uso: node scripts/print-raw-urls.js <caminho1> [caminho2 ...]",
    );
    process.exit(1);
  }

  const hash = getFullHeadHash();
  if (!/^[0-9a-fA-F]{40}$/.test(hash)) {
    console.error(
      `Hash inválido retornado pelo git: '${hash}' (deve ter exatamente 40 caracteres hexadecimais).`,
    );
    process.exit(1);
  }

  console.log(`🔍 Validando URLs raw no GitHub para o commit: ${hash}\n`);

  const urls = args.map((filePath) => {
    const normalized = filePath.replace(/\\/g, "/").replace(/^\/+/, "");
    return {
      filePath: normalized,
      url: `https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/${hash}/${normalized}`,
    };
  });

  const results = await Promise.all(urls.map((item) => checkUrl(item.url)));
  let hasFailure = false;

  console.log("--------------------------------------------------");
  for (let i = 0; i < urls.length; i++) {
    const res = results[i];
    const path = urls[i].filePath;

    if (res.ok) {
      console.log(`✅ [200 OK] ${path}`);
      console.log(`   ${res.url}\n`);
    } else {
      hasFailure = true;
      console.error(`❌ [${res.status || "ERR"}] ${path}`);
      console.error(`   ${res.url}`);
      console.error(`   Motivo: ${res.error}\n`);
    }
  }
  console.log("--------------------------------------------------");

  if (hasFailure) {
    console.error("\n❌ Falha: uma ou mais URLs não retornaram status 200.");
    process.exit(1);
  } else {
    console.log(
      `\n🎉 Todas as ${urls.length} URLs foram confirmadas com status 200 OK.`,
    );
  }
}

main().catch((err) => {
  console.error("Erro fatal:", err);
  process.exit(1);
});
