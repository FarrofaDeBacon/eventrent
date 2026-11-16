# Relatório de Execução — Fase 1B.2, Revisão 1B.2-rev e Revisão 1B.2-rev2

**Branch**: `fase-1b2-rev`  
**Base**: `main` (após PR #6 `e64237f`)  
**Data**: 16/11/2026  
**Status**: Concluída (PR #7 aberto, aguardando checks do CI para revisão final do usuário sem merge automático)

---

## 1. Itens da Revisão Técnica (Fase 1B.2-rev: R1 a R12)

| Item    | Descrição                                                                                                                                                                                                       | Status    | Commit                | Arquivos Principais                                                                                                                                  |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **R1**  | Correção da citação da norma para Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024 em todo o repositório (0 ocorrências de `2.229/2025`)                                                              | CONCLUÍDO | `1ea8c9d`             | Código, testes, ADRs e especificações                                                                                                                |
| **R2**  | Qualidade do core: `#![deny(clippy::unwrap_used, clippy::expect_used)]`, eliminação de `unwrap`/`expect` em produção e isolamento de `testing_utils` sob `#[cfg(any(test, feature = "test-utils"))]`            | CONCLUÍDO | `534bb07`             | `crates/core/src/lib.rs`, `crates/core/src/domain/document.rs`, `crates/core/Cargo.toml`                                                             |
| **R3**  | Normalização estrita de documentos (rejeição de acentos e símbolos) e fixtures oficiais RFB/ENCAT (`12.ABC.345/01DE-35`) com máscara LGPD                                                                       | CONCLUÍDO | `a4ac48b`             | `crates/core/src/domain/document.rs`, `crates/core/src/testing.rs`, `src/lib/utils/document.ts`                                                      |
| **R4**  | Migração `0005_customers_rev.sql` (ADR-0011): reconstrução de tabelas com `tenant_id`, `created_by`, `document_type`, `document_number` opcional, `active`, `country` e remoção de `idx_customer_tenant_search` | CONCLUÍDO | `e15d8f8`             | `crates/store-sqlite/migrations/0005_customers_rev.sql`, `docs/decisions/0011-reconstrucao-tabelas-clientes-migracao-0005.md`                        |
| **R5**  | IDs estáveis: inputs com `id` opcional, upsert preservando IDs existentes e soft delete dos removidos na mesma transação                                                                                        | CONCLUÍDO | `74a2ca4`             | `crates/core/src/domain/customer.rs`, `crates/core/src/services/mod.rs`, `crates/store-sqlite/src/lib.rs`, `src/lib/components/customer-form.svelte` |
| **R6**  | Operação `set_active` (`customer:manage`), documentação de `delete_customer` (soft delete + liberação de documento), regra de bloqueio da Fase 4 e pendência de anonimização LGPD                               | CONCLUÍDO | `3e50978`             | `crates/core/src/services/mod.rs`, `src-tauri/src/commands/customers.rs`, `docs/03-regras-negocio.md`, `docs/seguranca.md`                           |
| **R7**  | Busca: remoção de marcas combinantes NFD `U+0300..U+036F` em `remove_diacritics` (testes com `"José"` em NFC/NFD) e documentação de limite de escala da busca por trecho e plano FTS5/`pg_trgm`                 | CONCLUÍDO | `ba6d93d`             | `crates/core/src/domain/customer.rs`, `crates/core/src/testing.rs`, `docs/03-regras-negocio.md`                                                      |
| **R8**  | Paginação por cursor `URL_SAFE_NO_PAD` com erro limpo de validação (`CoreError::Validation`) em caso de cursor inválido/adulterado e limite padrão 50 / teto 100                                                | CONCLUÍDO | `6de1b5b`             | `crates/core/src/domain/customer.rs`, `crates/core/src/services/mod.rs`, `crates/store-sqlite/src/lib.rs`                                            |
| **R9**  | LGPD: `mask_email_for_audit`, `mask_phone_for_audit`, `Debug` manual com redação de PII nos tipos de cliente e prova de auditoria sem dados completos                                                           | CONCLUÍDO | `27e5e27`             | `crates/core/src/domain/document.rs`, `crates/core/src/domain/customer.rs`, `crates/core/src/services/mod.rs`, `docs/seguranca.md`                   |
| **R10** | Paridade mock x core: exportação de `SYSTEM_ROLES` para `src/lib/data/fixtures/system-roles.json`, teste Rust que falha se desatualizado e teste Vitest no mock                                                 | CONCLUÍDO | `4ba2081`             | `src/lib/data/fixtures/system-roles.json`, `crates/core/src/testing.rs`, `src/lib/data/auth-and-users.spec.ts`                                       |
| **R11** | Ampliação de `check-migrations.js` cobrindo `crates/*/migrations`, validação de versão única estritamente crescente e prova nos 6 cenários de teste                                                             | CONCLUÍDO | `e721706` / `6f76204` | `scripts/check-migrations.js`, `scripts/test-check-migrations.js`                                                                                    |
| **R12** | Relatório de entrega, PR #7 aberto e listagem das URLs diretas para revisão                                                                                                                                     | CONCLUÍDO | _Este commit_         | `docs/relatorios/fase-1b-2.md`                                                                                                                       |

---

## 2. EXPLAIN QUERY PLAN da Busca por Trecho

Saída oficial do SQLite obtida no teste `test_migration_0005_rebuilds_and_preserves_existing_customer_data`:

```text
[EXPLAIN QUERY PLAN] SEARCH customer USING INDEX idx_customer_tenant_sort (tenant_id=?)
```

Comprovou-se que o índice `idx_customer_tenant_search` não é utilizado pelo SQLite em consultas `search_text LIKE '%termo%'` combinadas com ordenação por cursor. Sua remoção na migração `0005` economizou espaço em disco e eliminou overhead desnecessário de escrita em todas as operações de cadastro e atualização de clientes.

---

## 3. URLs Diretas para Revisão (Branch `fase-1b2-rev`)

- Migração 0005: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/crates/store-sqlite/migrations/0005_customers_rev.sql
- Serviços do Core: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/crates/core/src/services/mod.rs
- Traits de Store: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/crates/core/src/store/mod.rs
- SQLite Store: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/crates/store-sqlite/src/lib.rs
- Comandos Tauri de Clientes: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/src-tauri/src/commands/customers.rs
- Componente VirtualList: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/src/lib/components/virtual-list.svelte
- Script de Verificação de Migrações: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/scripts/check-migrations.js
- ADR-0011 (Reconstrução na 0005): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/fase-1b2-rev/docs/decisions/0011-reconstrucao-tabelas-clientes-migracao-0005.md

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

| Item    | Descrição                                                                                                                              | Status    | Commit        | Arquivos Principais                                                                                                                           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| **1.1** | Validação de CPF e CNPJ numérico/alfanumérico (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024 `ASCII - 48`) + máscara LGPD | CONCLUÍDO | `606ca36`     | `crates/core/src/domain/document.rs`, `crates/core/src/testing.rs`                                                                            |
| **1.2** | Migração `0004_customers.sql` (`customer`, `customer_contact`, `customer_address` e índices parciais)                                  | CONCLUÍDO | `f01983e`     | `crates/store-sqlite/migrations/0004_customers.sql`                                                                                           |
| **1.3** | Entidades de cliente, normalização sem acentos (`search_text`, `name_sort`) e cursor Base64                                            | CONCLUÍDO | `b9f88d7`     | `crates/core/src/domain/customer.rs`, `crates/core/src/testing.rs`                                                                            |
| **1.4** | Trait `CustomerStore`, serviços CRUD em `CoreServices`, auditoria LGPD e implementação SQLite                                          | CONCLUÍDO | `785b62a`     | `crates/core/src/stores.rs`, `crates/core/src/services.rs`, `crates/store-sqlite/src/lib.rs`, `crates/core/src/testing.rs`                    |
| **1.5** | Módulo Frontend de clientes (lista virtualizada 20k, formulário PF/PJ dinâmico, ficha e busca via `Ctrl+K`)                            | CONCLUÍDO | `ede16fd`     | `src/lib/components/virtual-list.svelte`, `src/lib/components/customer-form.svelte`, `src/routes/customers/**`, `src/lib/utils/document.ts`   |
| **1.6** | Comandos Tauri de clientes com guarda `require_session` e registro em `COMMAND_REGISTRY`                                               | CONCLUÍDO | `7d3558d`     | `src-tauri/src/commands/customers.rs`, `src-tauri/src/guards.rs`, `src-tauri/src/lib.rs`                                                      |
| **1.7** | Benchmark SQLite de 20.000 clientes e teste unitário de virtualização DOM (`VirtualList`)                                              | CONCLUÍDO | `b4dac02`     | `crates/store-sqlite/src/lib.rs`, `src/lib/components/virtual-list.spec.ts`, `vite.config.ts`                                                 |
| **1.8** | Testes unitários de documentos/RBAC e fluxo E2E completo Playwright (`Ctrl+K`, CNPJ alfanumérico, soft delete)                         | CONCLUÍDO | `4c84ab3`     | `src/lib/utils/document.spec.ts`, `src/lib/data/customer-repository.spec.ts`, `src/lib/data/mock-auth-repository.ts`, `e2e/customers.spec.ts` |
| **1.9** | Atualização da documentação (`02-modelo-dados`, `03-regras-negocio`, `seguranca`) e relatório final                                    | CONCLUÍDO | _Este commit_ | `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md`, `docs/seguranca.md`, `docs/relatorios/fase-1b-2.md`                                   |

---

## 5. Itens da Revisão Técnica Complementar (Fase 1B.2-rev2: Itens A a J)

| Item  | Descrição                                                                                                                                                                                 | Status    | Commit        | Arquivos Principais                                                                                                                                                                |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A** | FK composta `(tenant_id, customer_id)` nas filhas `customer_contact` e `customer_address` na migração `0005_customers_rev.sql` e atualização do ADR-0011                                  | CONCLUÍDO | `3596008`     | `crates/store-sqlite/migrations/0005_customers_rev.sql`, `docs/decisions/0011-reconstrucao-tabelas-clientes-migracao-0005.md`                                                      |
| **B** | Validações estritas no Core: CEP (8 dígitos), UF (27 siglas válidas), telefone (10/11 dígitos numéricos), e-mail RFC 5322 e limites máximos de tamanho e quantidade de itens filhas       | CONCLUÍDO | `62bed68`     | `crates/core/src/domain/customer.rs`, `crates/core/src/domain/mod.rs`, `crates/core/src/testing.rs`                                                                                |
| **C** | Contato obrigatório (pelo menos 1 com e-mail ou telefone) e endereço opcional ("quando existirem"), documentando que a Fase 4 vinculará o endereço do evento diretamente à locação        | CONCLUÍDO | `8b19b47`     | `crates/core/src/domain/customer.rs`, `src/lib/components/customer-form.svelte`, `docs/03-regras-negocio.md`                                                                       |
| **D** | Concorrência otimista com `expected_updated_at`, leituras movidas para dentro da transação e restrição de fallback sem transação (`PassthroughTxStoreBundle` apenas sob testes)           | CONCLUÍDO | `7e2ea78`     | `crates/core/src/error.rs`, `crates/core/src/services/mod.rs`, `crates/core/src/store/mod.rs`, `src/lib/components/customer-form.svelte`, `src/routes/customers/[id]/+page.svelte` |
| **E** | Alinhamento de `recover_password` com proteção anti-enumeração, timing seguro com hash Argon2 dummy e respeito estrito a `LoginPolicy`                                                    | CONCLUÍDO | `290ad72`     | `crates/core/src/services/mod.rs`, `crates/store-sqlite/src/lib.rs`, `src-tauri/src/commands/auth.rs`                                                                              |
| **F** | Indexação de telefones limpos (só dígitos) e nomes de contatos em `search_text`, limite de busca (100 caracteres / 8 palavras) e mensagem clara de duplicidade de documento ativo/inativo | CONCLUÍDO | `3663df6`     | `crates/core/src/domain/customer.rs`, `crates/core/src/services/mod.rs`, `crates/core/src/testing.rs`, `src/lib/data/customer-repository.spec.ts`                                  |
| **G** | Redação estrita de PII na auditoria (mascaramento de nomes de PF e e-mails de usuários/clientes) e documentação de segurança com referência ao Art. 16 da LGPD (retenção/descarte)        | CONCLUÍDO | `41befba`     | `crates/core/src/services/mod.rs`, `crates/core/src/testing.rs`, `docs/seguranca.md`                                                                                               |
| **H** | Proteção de comandos públicos (`get_public_branding` / `AppStatus.branding` expondo apenas `PublicBranding` sem IDs internos ou dados de auditoria) comprovada por teste unitário         | CONCLUÍDO | `4a114f8`     | `src-tauri/src/guards.rs`                                                                                                                                                          |
| **I** | Provas no SQLite: troca atômica de contato/endereço principal sem colisão de índice único, soft delete com preservação de IDs e isolamento estrito multitenant                            | CONCLUÍDO | `2c594dc`     | `crates/store-sqlite/src/lib.rs`                                                                                                                                                   |
| **J** | Consolidação documental da revisão rev2, portões 100% aprovados, links de verificação do CI e preparação do PR #7 para entrega ao usuário sem merge automático                            | CONCLUÍDO | _Este commit_ | `docs/relatorios/fase-1b-2.md`                                                                                                                                                     |

---

## 6. Métricas de Performance, Testes e Qualidade

### 6.1. Benchmark SQLite 20.000 Clientes e Virtualização DOM

- **Benchmark SQLite (`test_benchmark_20k_customers_search_and_pagination`)**:
  - Inserção em massa de 20.000 registros: `870 ms`
  - Primeira página por cursor (`page1`, 50 itens): `1 ms`
  - Segunda página por cursor composto (`page2_cursor`, 50 itens): `1 ms`
  - Busca desacentuada indexada (`search`): `11 ms` (requisito `< 500 ms`)
- **Virtualização DOM (`VirtualList` em `src/lib/components/virtual-list.spec.ts`)**:
  - Lista com 20.000 itens renderiza janela inferior a 40 elementos `[data-virtual-index]` ativos no DOM, com rolagem direta verificada até o item 10.000.

### 6.2. Resultados Consolidados dos Portões de Verificação

- **`node scripts/verify.js`** (executado antes de cada commit):
  1. `svelte-check`: `0` erros e `0` avisos.
  2. Lint & Cores: `0` cores literais em `28` componentes `.svelte`, Prettier e ESLint aprovados.
  3. Vitest (`pnpm test:unit`): `6` arquivos, `35/35` testes aprovados (`color`, `document`, `auth-and-users`, `customer-repository`, `branding-repository`, `virtual-list`).
  4. Congelamento de migrações (`scripts/test-check-migrations.js`): `6/6` cenários de prova aprovados (`0001` a `0004` protegidas; versões duplicadas e retroativas bloqueadas; nova versão sequencial permitida).
  5. `cargo fmt --check`: aprovado.
  6. `cargo clippy --all-targets -- -D warnings`: aprovado sem avisos (`#![deny(clippy::unwrap_used, clippy::expect_used)]`).
  7. `cargo test --workspace`: `51/51` testes aprovados (`9` em `eventrent_app_lib`, `17` em `eventrent_core`, `25` em `eventrent_store_sqlite`).
- **Suíte E2E Playwright (`pnpm test:e2e`)**:
  - `3/3` fluxos aprovados em Chromium headless (`smoke.spec.ts`, `branding.spec.ts`, `customers.spec.ts`).

---

## 7. Próximos Passos (Ação Exclusiva do Usuário)

1. Revisar o Pull Request #7 da branch `fase-1b2-rev` para `main` no GitHub.
2. Conferir os dois checks verdes do CI (`Check & Tests (ubuntu-latest)` e `Check & Tests (windows-latest)`).
3. Após revisão e aprovação formal, o **usuário** realiza o merge na interface do GitHub e cria a tag `v0.5.0-fase1b2`.
