# AGENTS.md — Regras Permanentes de Engenharia do EventRent

Este documento estabelece as diretrizes arquiteturais, de qualidade de código, padrões de commit e processos operacionais para qualquer agente ou engenheiro que atue no repositório **EventRent**.

---

## 1. Papel e Responsabilidades

- O **Diretor do Projeto** define a arquitetura, regras de negócio e escopo de cada fase.
- O **Engenheiro de Implementação** executa com rigor técnico absoluto. Não altere decisões de arquitetura por conta própria.
- Em caso de ambiguidade ou necessidade técnica de ajuste, registre em um **ADR** em `docs/decisions/` e aponte no relatório da fase.

---

## 2. Stack Tecnológica Obrigatória

- **Frontend**: SvelteKit 2 + Svelte 5 (runes) + TypeScript strict + Vite + `@sveltejs/adapter-static`.
- **UI & Estilização**: Tailwind 4, `bits-ui` (padrão shadcn-svelte), `lucide-svelte`, `svelte-sonner`, `mode-watcher`.
- **Formulários**: `sveltekit-superforms` + `formsnap` + `zod`.
- **i18n**: `svelte-i18n` (padrão pt-BR desde o início).
- **Datas**: `date-fns` e `@internationalized/date`.
- **Testes**: `vitest` + `@testing-library/svelte`; `playwright` para e2e.
- **Backend**: Rust (Cargo Workspace).
  - `crates/core`: Domínio, regras de negócio, serviços (`CoreServices`) e traits assíncronas de repositório (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`). **Zero** dependência de Tauri e **zero** dependência de `sqlx`.
  - `crates/store-sqlite`: Implementação SQLite das traits do `core` e migrações versionadas (`crates/store-sqlite/migrations`).
  - `crates/store-postgres` (Fase 2.5): Implementação PostgreSQL das mesmas traits do `core`.
  - `src-tauri`: Wrapper fino da aplicação Tauri 2; monta `CoreServices` com `Arc<dyn ...>` no `AppState` e expõe comandos IPC finos.
- **Banco de Dados**: SQLite local (modo Local) em `crates/store-sqlite` e PostgreSQL (modo Nuvem, Fase 2.5), validados pela suíte genérica de testes de contrato (`run_store_contract_suite`).
- **Gerenciador de Pacotes**: `pnpm`. Multiplataforma (Windows prioritário, compatível com Linux e macOS).

> **Proibição**: Nenhuma dependência externa à lista acima pode ser adicionada sem um ADR formal aprovado em `docs/decisions/`.

---

## 3. Regras de Arquitetura Inegociáveis

1. **Fronteira de Dados e Transporte Abstrato**:
   - Componentes e páginas Svelte **nunca** chamam Tauri `invoke` ou APIs externas diretamente.
   - Toda comunicação passa por interfaces tipadas em `src/lib/data/`.
   - A implementação desktop usa Tauri `invoke`. No futuro, uma implementação HTTP usará a **mesma** interface sem alterar nenhuma tela.
2. **Regras de Negócio no Backend**:
   - Lógica de domínio, validações complexas, cálculos e transações residem em `crates/core`.
   - Comandos em `src-tauri` são delegadores finos (parse de entrada -> chamada ao core -> retorno de resultado).
3. **Padrão de Tabelas e Entidades**:
   - Toda tabela possui:
     - `id`: Texto contendo UUID v7 ordenável.
     - `tenant_id`: Texto contendo ID do tenant (mesmo no desktop single-tenant inicial).
     - `created_at`: Data/hora UTC em formato ISO-8601 string.
     - `updated_at`: Data/hora UTC em formato ISO-8601 string.
     - `deleted_at`: Data/hora UTC para soft-delete (nulo se ativo).
     - `created_by`: Identificador do usuário que criou (quando aplicável).
4. **Precisão Numérica e Temporal**:
   - Dinheiro sempre em **inteiro** (centavos), nunca ponto flutuante.
   - Datas e horas sempre em **UTC** (ISO-8601).
5. **Transacionalidade e Consistência**:
   - Checagem e reserva de disponibilidade ocorrem estritamente na **mesma** transação no banco de dados.
6. **Controle Híbrido de Estoque**:
   - Equipamentos controlados por número de série (`serialized`) ou por quantidade/saldo (`quantity`).
   - Kits são compostos por itens que expandem na reserva/separação.
7. **Design System & White Label**:
   - Zero cores, raios de borda ou espaçamentos arbitrários hardcoded nos componentes.
   - Uso de tokens CSS OKLCH e variáveis semânticas que reagem ao tema (claro/escuro) e branding do tenant.
8. **Internacionalização e Acessibilidade**:
   - 100% dos textos exibidos ao usuário passam pelo `svelte-i18n`.
   - Foco visível, navegação por teclado, contraste WCAG e suporte a `prefers-reduced-motion`.
9. **Segurança e Rigor de Tipos**:
   - Sem `any` em TypeScript (`strict: true`).
   - Sem `unwrap()` em código Rust de produção. Erros tratados via tipos de erro explícitos (`Result<T, E>`).
10. **Segurança de Credenciais**:
    - Nunca comitar segredos, chaves de API, arquivos de banco local (`*.db`) ou variáveis `.env`.
11. **Unicidade com Exclusão Lógica**:
    - Usar índices únicos parciais `WHERE deleted_at IS NULL`, em migração nova; nunca `UNIQUE` simples em tabela com `deleted_at`.
    - **Exceção documentada (`tenant_setting`)**: A tabela `tenant_setting` armazena pares chave-valor de configuração por tenant e não possui `created_at` nem exclusão lógica (`deleted_at`), utilizando `UNIQUE (tenant_id, setting_key)`. Todo histórico de alterações é preservado em `audit_log` na mesma transação.
12. **Escopo de Migrações**:
    - Migrações contêm apenas estrutura, nunca dados de negócio. Dados iniciais do ambiente desktop (como o tenant padrão) são garantidos via código idempotente na inicialização.
13. **Imutabilidade e Congelamento de Migrações (Fase 1B.0+)**:
    - As migrações existentes na `main` estão permanentemente congeladas e não podem sofrer qualquer alteração.
    - Qualquer alteração ou extensão de esquema deve ser feita obrigatoriamente através de novas migrações versionadas incrementais. O CI valida e bloqueia alterações nas migrações congeladas em pull requests.
14. **Proibição de Reconstrução de Tabelas (`DROP`/`CREATE`)**:
    - Preferir sempre `ALTER TABLE ... ADD COLUMN` e novos índices incrementais em novas migrações.
    - É estritamente proibido reconstruir tabelas (`DROP TABLE` + `CREATE TABLE`) sem ADR específico aprovado e sem teste automatizado de migração que comprove preservação integral dos dados existentes.
15. **Fonte Obrigatória de Requisitos (Fases 2 a 4.5)**:
    - O documento [`docs/06-requisitos-de-produto.md`](docs/06-requisitos-de-produto.md) é fonte obrigatória de requisitos de produto e regras de domínio para todas as entregas da Fase 2 até a Fase 4.5.

---

## 4. Fluxo Git e Controle de Qualidade

- **Branches**: Cada fase possui sua branch dedicada (ex.: `fase-0-fundacao`).
- **Commits**:
  - Pequenos, atômicos e frequentes por unidade lógica concluída.
  - Formato Conventional Commits em **português**: `feat(...)`, `fix(...)`, `docs(...)`, `test(...)`, `chore(...)`, `refactor(...)`, `ci(...)`.
- **Verificação Pré-Commit Obrigatória**:
  Nenhum commit pode ser realizado sem que todos os seguintes comandos passem com sucesso:
  ```bash
  pnpm check
  pnpm test:run
  cargo fmt --check
  cargo clippy --workspace -- -D warnings
  cargo test --workspace
  ```
- **Fechamento de Fase**:
  - Criação da tag de versão (ex.: `v0.0.1-fase0`).
  - Geração do relatório obrigatório em `docs/relatorios/fase-<N>.md`.

---

## 5. Definição de Pronto Permanente (DoD em 4 Níveis)

Para que qualquer história, funcionalidade ou fase seja considerada concluída ("Pronto"), ela deve satisfazer rigorosamente os 4 níveis de garantia:

### Nível 1: Banco de Dados & Armazenamento

- [ ] **Migrações Imutáveis**: Migrações existentes congeladas. Novas migrações são puramente incrementais (`ALTER TABLE ADD COLUMN`, índices parciais) e reversíveis.
- [ ] **Isolamento Multitenant Estrito**: Toda tabela possui `tenant_id` e toda query/filtro no SQLite/Postgres escopa por tenant.
- [ ] **Chaves Compostas e Índices Parciais**: Unicidade lógica garantida via `UNIQUE (tenant_id, ...)` com `WHERE deleted_at IS NULL`.
- [ ] **Transacionalidade & Rollback**: Operações atômicas com `run_transaction` e rollback garantido em caso de falha.
- [ ] **Auditoria Completa**: Toda mutação sensível gera entrada atômica em `audit_log`, sem nunca persistir segredos, dados binários de mídia ou chaves brutas.

### Nível 2: Domínio & Regras de Negócio (Core)

- [ ] **Independência de Framework**: O crate `crates/core` permanece com zero dependência de Tauri e zero dependência de SQL direto (`sqlx`).
- [ ] **Validações Canônicas & Sanitização**: Strings, códigos, documentos e valores validados na fronteira de domínio.
- [ ] **RBAC Canônico**: Todas as permissões verificadas no Core contra `ALL_PERMISSIONS`. Nenhuma permissão fictícia ou descontinuada.
- [ ] **Revalidação de Sessão Dinâmica**: Sessões validadas contra `auth_epoch` e permissões do banco a cada comando protegido.
- [ ] **Testes de Contrato e Unidade**: Suíte `test_sqlite_store_contract_suite` e testes de domínio passando com 100% de sucesso.

### Nível 3: IPC & Contratos Tauri / Transporte

- [ ] **Registro e Guardas Universais**: Todo comando protegido registrado em `COMMAND_REGISTRY` e protegido por `require_session(&state).await?`.
- [ ] **Paridade Estrita de Tipos & DTOs**: Nenhuma perda silenciosa de retornos (proibido `Promise<void>` em comandos com retorno). Fixtures JSON versionadas e validadas por `cargo test` e Vitest (`contracts.spec.ts`).
- [ ] **Menor Privilégio de Capabilities**: Permissões do Tauri em `src-tauri/capabilities/default.json` reduzidas ao mínimo estrito (`core:default`), proibindo `opener`, `fs`, `http`, `dialog`.
- [ ] **Segurança de Mídia & Arquivos**: Limpeza preventiva de arquivos temporários, nomes restritos saneados (proibindo device names do Windows `COM1..9`, `COM¹..³`), extensão fixa `.pdf` forçada pelo backend em documentos.
- [ ] **CSP Estrita**: CSP sem scripts inline e sem `'unsafe-eval'` em `tauri.conf.json`.

### Nível 4: Interface & Experiência do Usuário (Frontend)

- [ ] **Isolamento de Mocks fora de Produção**: Repositórios mock (`mock-*.ts`) executam exclusivamente em ambiente de desenvolvimento (`import.meta.env.DEV`). Execução web em produção renderiza tela clara de requisito de aplicativo desktop.
- [ ] **Banner de Demonstração Transparente**: Modo mock exibe tarja acessível "MODO DE DEMONSTRAÇÃO: Não use dados reais".
- [ ] **Tokens Semânticos & White Label**: Zero cores hardcoded nos componentes (`check-no-hardcoded-colors.js` passando 100%).
- [ ] **Acessibilidade & Internacionalização**: 100% dos textos via `svelte-i18n`, foco navegável por teclado e contraste WCAG AA.
- [ ] **Testes E2E Automatizados**: Suíte Playwright executando em runners Ubuntu e Windows.

---

## 6. Checklist para Próximas Fases: Fase 2N — Kits e Composição

Ao iniciar a **Fase 2N (Kits e Composição de Equipamentos)**, seguir estritamente o seguinte roteiro:

1. **Modelagem de Dados (Nível 1)**:
   - [ ] Migração `0011_equipment_kits.sql` (ou sequencial seguinte) criando `equipment_kit` e `equipment_kit_item`.
   - [ ] FKs compostas referenciando `(tenant_id, parent_kit_id)` e `(tenant_id, component_model_id)`.
   - [ ] Prevenção de ciclos recursivos e profundidade máxima na composição de kits.
   - [ ] Quantidades e multiplicadores em inteiros positivos.
2. **Domínio e Serviços (Nível 2)**:
   - [ ] Trait `KitStore` no `core` e implementação correspondente no `store-sqlite`.
   - [ ] Regras de expansão de kits para reservas e ordens de separação.
   - [ ] Auditoria de montagem/desmontagem de kits.
3. **IPC e Contratos (Nível 3)**:
   - [ ] Novos comandos Tauri registrados em `COMMAND_REGISTRY` e `guards.rs`.
   - [ ] Fixtures geradas e validadas via `EVENTRENT_UPDATE_CONTRACTS=1`.
4. **Interface (Nível 4)**:
   - [ ] Tela de composição de kits com busca visual de modelos.
   - [ ] Validações dinâmicas de itens repetidos ou circulares.
   - [ ] `pnpm verify` 100% verde antes de cada commit.
