# Relatório de Execução — Fase 1B.2 (Correções de Revisão + Cadastro de Clientes)

**Branch**: `fase-1b2-clientes`  
**Base**: `main` (após merge do PR #5 — commit `dce1d62`, tag `v0.4.0-fase1b1`)  
**Data**: 11/11/2026  
**Status**: Concluída (pronta para abertura de PR, revisão, merge e tag pelo usuário)

---

## 1. Tarefa 0 — Correções Obrigatórias de Revisão

| Item    | Descrição                                                                                                                           | Status    | Commit    | Arquivos Principais                                                                                                                  |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------- | --------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **0.1** | Congelamento dinâmico de qualquer migração na `main` e prova em 5 cenários com nomes reais                                          | CONCLUÍDO | `3c11dee` | `scripts/check-migrations.js`, `scripts/test-check-migrations.js`, `.github/workflows/ci.yml`, `scripts/verify.js`                   |
| **0.2** | Razão bruta de contraste para aprovação WCAG AA (>= 4.5) e arredondamento apenas para exibição                                      | CONCLUÍDO | `e45083f` | `src/lib/theme/color.ts`, `src/lib/theme/color.spec.ts`                                                                              |
| **0.3** | BrandingValidator com enum de mime, sem `<svg`, pré-checagem de tamanho e validação de dimensões via `imagesize` (ADR-0010)         | CONCLUÍDO | `04ee47f` | `crates/core/src/domain/branding.rs`, `crates/core/src/testing.rs`, `docs/decisions/0010-uso-crate-imagesize-validacao-dimensoes.md` |
| **0.4** | `map_sqlx_err` com classificação estrita por códigos SQLite (5, 6, 261, 517, 773, 262, 518, 2067, 1555, 787) e ocultação de SQL cru | CONCLUÍDO | `ce29ad0` | `crates/store-sqlite/src/lib.rs`                                                                                                     |
| **0.5** | Sincronização do ponteiro `eventrent:brand:last`, isolamento multitenant no script anti-FOUC e verificação de CSP                   | CONCLUÍDO | `d35aba4` | `src/lib/data/session.svelte.ts`, `src/app.html`, `src/lib/data/branding-repository.spec.ts`                                         |
| **0.6** | Validação estrita de HEX com regex e rejeição de `#1g0000` no `hexToRgb`                                                            | CONCLUÍDO | `63d958e` | `src/lib/theme/color.ts`, `src/lib/theme/color.spec.ts`                                                                              |
| **0.7** | Atualização do ADR-0008: semântica de retry após COMMIT (SQLite seguro vs. PostgreSQL in-doubt transaction)                         | CONCLUÍDO | `17e6530` | `docs/decisions/0008-transacoes-nas-stores.md`                                                                                       |
| **0.8** | Relatório parcial, clarificação de processos (merges/tags) e listagem exaustiva de testes R3/R4                                     | CONCLUÍDO | `84fc571` | `docs/relatorios/fase-1b-2.md`                                                                                                       |

---

## 2. Esclarecimento de Governança e Processo (Merges e Tags)

- **PR #4**: O merge e a tag `v0.3.0-fase1b0` foram realizados pelo agente localmente na rodada anterior.
- **PR #5**: O merge foi executado pelo **usuário** na interface web do GitHub (merge commit `dce1d62`). O agente realizou o `git checkout main`, `git pull`, criou a tag `v0.4.0-fase1b1` sob comando do usuário, deu push na tag para `origin` e abriu a branch de trabalho `fase-1b2-clientes`.
- **Diretriz Definitiva**: A partir desta fase, **merges de Pull Requests e criação de tags de release pertencem exclusivamente ao usuário**. O agente nunca realizará merge na `main` nem criará tags sem instrução pontual explícita.

---

## 3. Listagem Exaustiva dos Testes de Verificação

### 3.1 Testes R3 — Validação Estrita de Imagens e Base64 (`crates/core/src/testing.rs`)

Implementados em `test_branding_validator_images_magic_bytes_and_formats` e submódulos de teste do `core`:

1. **Rejeição por tamanho codificado prévio**: Validação de limite de 350.000 bytes base64 antes de acionar decodificação em memória, prevenindo esgotamento de heap.
2. **Rejeição por prefixo MIME inválido ou não suportado**: Prefixos como `data:image/svg+xml;base64,`, `data:image/gif;base64,` ou strings sem `base64,` são rejeitados com erro de validação.
3. **Rejeição de caracteres fora do alfabeto Base64**: Testes com strings contendo caracteres ilegais (`$`, `@`, `!`, espaços) falham na decodificação em modo estrito `engine::GeneralPurpose::new(&alphabet::STANDARD, general_purpose::PAD)`.
4. **Rejeição de padding incorreto ou truncado**: Strings base64 com bytes faltando ou com caracteres `=` em posições incorretas são rejeitadas.
5. **Rejeição de imagem vazia ou truncada**: Slices decodificados com tamanho inferior ao cabeçalho mínimo ou sem chunks válidos são rejeitados.
6. **Rejeição por divergência de magic bytes**: Prefixos declarando PNG com payload real JPEG ou WebP (e vice-versa) são rejeitados.
7. **Validação de dimensões com `imagesize` (ADR-0010)**:
   - Logos superiores a 512×512 pixels são rejeitadas.
   - Ícones superiores a 128×128 pixels são rejeitados.
8. **Proteção contra bomba de descompressão (decompression bomb)**: PNGs ou imagens que declaram dimensões monstruosas (ex: 30.000×30.000 px) são interceptadas pelo leitor de cabeçalho do `imagesize` sem alocar buffers de pixels.
9. **Aceitação de imagens legítimas**: PNG, JPEG e WebP válidos com dimensões $\le 512\times 512$ e tamanho $\le 256$ KB para logos, e $\le 128\times 128$ e tamanho $\le 64$ KB para ícones passam com sucesso.

### 3.2 Testes R4 — Matemática de Cores, OKLCH e Contraste WCAG (`src/lib/theme/color.spec.ts`)

1. **Conversão sRGB <-> OKLab <-> OKLCH com pontos de referência oficiais (Björn Ottosson)**:
   - Vermelho puro (`#ff0000`): $L = 0.628$, $C = 0.2577$, $h = 29.23^\circ$
   - Azul puro (`#0000ff`): $L = 0.452$, $C = 0.3132$, $h = 264.05^\circ$
   - Verde puro (`#00ff00`): $L = 0.8664$, $C = 0.2948$, $h = 142.5^\circ$
   - Branco puro (`#ffffff`): $L = 1.0$, $C = 0.0$
   - Preto puro (`#000000`): $L = 0.0$, $C = 0.0$
2. **Matrizes Direta e Inversa OKLab**:
   - Teste de ida e volta garantindo tolerância $\le 10^{-4}$ entre sRGB -> linear -> OKLab -> linear -> sRGB.
3. **Gamut Mapping**:
   - Ajuste de croma iterativo garantindo valores dentro de $[0, 1]$ em sRGB sem corte abrupto de matiz.
4. **Razão de Contraste Bruta WCAG 2.1**:
   - `#ffffff` vs `#000000` $= 21.0:1$.
   - `#ffffff` vs `#ffffff` $= 1.0:1$.
   - Limiares críticos de aprovação: `4.496:1` e `4.4999:1` retornam `false` em `isAccessibleContrast` (não aprovados para AA), enquanto `4.5000:1` retorna `true`.
   - `formatContrastRatio` formata `4.496` como `4.5` apenas para visualização humana, mantendo o critério de aprovação estritamente na razão bruta.
5. **Geração de Escala Semântica em 11 Passos**:
   - Monotonicidade decrescente de luminância de 50 ($L \approx 0.97$) até 950 ($L \approx 0.14$).
6. **Ajuste Automático de Contraste (`autoAdjustContrast`)**:
   - Ajusta cores claras ou escuras insuficientes para atingir contraste $\ge 4.5:1$ contra o fundo correspondente.
7. **Validação Estrita de Hexadecimal**:
   - Rejeição de strings inválidas (`#fff`, `invalid`, `#gggggg`).
   - Rejeição de strings com caracteres não-hexadecimais camuflados (`#1g0000`, `#12345z`, `#12 456`).
   - Validador `isValidHexColor` para feedback ao vivo no formulário.

---

## 4. Tarefa 1 — Cadastro de Clientes (Itens 1.1 a 1.9)

| Item    | Descrição                                                                                                      | Status    | Commit        | Arquivos Principais                                                                                                                           |
| ------- | -------------------------------------------------------------------------------------------------------------- | --------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| **1.1** | Validação de CPF e CNPJ numérico/alfanumérico (IN RFB nº 2.229/2025 `ASCII - 48`) + máscara LGPD               | CONCLUÍDO | `606ca36`     | `crates/core/src/domain/document.rs`, `crates/core/src/testing.rs`                                                                            |
| **1.2** | Migração `0004_customers.sql` (`customer`, `customer_contact`, `customer_address` e índices parciais)          | CONCLUÍDO | `f01983e`     | `crates/store-sqlite/migrations/0004_customers.sql`                                                                                           |
| **1.3** | Entidades de cliente, normalização sem acentos (`search_text`, `name_sort`) e cursor Base64                    | CONCLUÍDO | `b9f88d7`     | `crates/core/src/domain/customer.rs`, `crates/core/src/testing.rs`                                                                            |
| **1.4** | Trait `CustomerStore`, serviços CRUD em `CoreServices`, auditoria LGPD e implementação SQLite                  | CONCLUÍDO | `785b62a`     | `crates/core/src/stores.rs`, `crates/core/src/services.rs`, `crates/store-sqlite/src/lib.rs`, `crates/core/src/testing.rs`                    |
| **1.5** | Módulo Frontend de clientes (lista virtualizada 20k, formulário PF/PJ dinâmico, ficha e busca via `Ctrl+K`)    | CONCLUÍDO | `ede16fd`     | `src/lib/components/virtual-list.svelte`, `src/lib/components/customer-form.svelte`, `src/routes/customers/**`, `src/lib/utils/document.ts`   |
| **1.6** | Comandos Tauri de clientes com guarda `require_session` e registro em `COMMAND_REGISTRY`                       | CONCLUÍDO | `7d3558d`     | `src-tauri/src/commands/customers.rs`, `src-tauri/src/guards.rs`, `src-tauri/src/lib.rs`                                                      |
| **1.7** | Benchmark SQLite de 20.000 clientes e teste unitário de virtualização DOM (`VirtualList`)                      | CONCLUÍDO | `b4dac02`     | `crates/store-sqlite/src/lib.rs`, `src/lib/components/virtual-list.spec.ts`, `vite.config.ts`                                                 |
| **1.8** | Testes unitários de documentos/RBAC e fluxo E2E completo Playwright (`Ctrl+K`, CNPJ alfanumérico, soft delete) | CONCLUÍDO | `4c84ab3`     | `src/lib/utils/document.spec.ts`, `src/lib/data/customer-repository.spec.ts`, `src/lib/data/mock-auth-repository.ts`, `e2e/customers.spec.ts` |
| **1.9** | Atualização da documentação (`02-modelo-dados`, `03-regras-negocio`, `seguranca`) e relatório final            | CONCLUÍDO | _Este commit_ | `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md`, `docs/seguranca.md`, `docs/relatorios/fase-1b-2.md`                                   |

---

## 5. Métricas de Performance, Testes e Qualidade

### 5.1. Benchmark SQLite 20.000 Clientes e Virtualização DOM

- **Benchmark SQLite (`test_benchmark_20k_customers_search_and_pagination`)**:
  - Inserção em massa de 20.000 registros: `870 ms`
  - Primeira página por cursor (`page1`, 50 itens): `1 ms`
  - Segunda página por cursor composto (`page2_cursor`, 50 itens): `1 ms`
  - Busca desacentuada indexada (`search`): `11 ms` (requisito `< 500 ms`)
- **Virtualização DOM (`VirtualList` em `src/lib/components/virtual-list.spec.ts`)**:
  - Lista com 20.000 itens renderiza janela inferior a 40 elementos `[data-virtual-index]` ativos no DOM, com rolagem direta verificada até o item 10.000.

### 5.2. Resultados Consolidados dos Portões de Verificação

- **`node scripts/verify.js`** (executado antes de cada commit):
  1. `svelte-check`: `0` erros e `0` avisos.
  2. Lint & Cores: `0` cores literais em `28` componentes `.svelte`, Prettier e ESLint aprovados.
  3. Vitest (`pnpm test:unit`): `6` arquivos, `34/34` testes aprovados (`color`, `document`, `auth-and-users`, `customer-repository`, `branding-repository`, `virtual-list`).
  4. Congelamento de migrações (`scripts/test-check-migrations.js`): `5/5` cenários de prova aprovados (`0001`, `0002` e `0003` protegidas; `0004_customers.sql` permitida).
  5. `cargo fmt --check`: aprovado.
  6. `cargo clippy --all-targets -- -D warnings`: aprovado sem avisos.
  7. `cargo test --workspace`: `43/43` testes aprovados (`8` em `eventrent_app_lib`, `14` em `eventrent_core`, `21` em `eventrent_store_sqlite`).
- **Suíte E2E Playwright (`pnpm test:e2e`)**:
  - `3/3` fluxos aprovados em Chromium headless (`smoke.spec.ts`, `branding.spec.ts`, `customers.spec.ts`).

---

## 6. Próximos Passos (Ação Exclusiva do Usuário)

1. Abrir Pull Request da branch `fase-1b2-clientes` para `main` no GitHub.
2. Conferir os dois checks verdes do CI (`verify` Windows e Ubuntu).
3. Após revisão e aprovação do PR, realizar o merge (`Create a merge commit`) e criar a tag de release correspondente.
