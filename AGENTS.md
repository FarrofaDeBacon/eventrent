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
  - `crates/core`: Regras de negócio, domínio, repositórios e migrações SQL. **Zero** dependência de Tauri.
  - `src-tauri`: Wrapper fino da aplicação Tauri 2; comandos chamam apenas `crates/core`.
- **Banco de Dados**: SQLite via `sqlx` (com migrações versionadas em `crates/core/migrations`). SQL portável preparado para migração futura ao PostgreSQL.
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
