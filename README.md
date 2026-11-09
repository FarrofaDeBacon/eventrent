# EventRent — Sistema de Gestão de Locação para Eventos

Sistema de gestão operacional e financeira de locação de equipamentos para empresas de eventos (audiovisual, som, luz, imagem e decoração).

Desenvolvido com arquitetura **Desktop First** baseada em **Tauri 2**, frontend **SvelteKit 2 + Svelte 5 (Runes)**, estilização com **Tailwind 4** e **Tokens OKLCH** para suporte nativo a White Label, e backend de domínio transacional em **Rust** com banco de dados **SQLite (SQLx)**.

---

## 🛠️ Stack Tecnológica

### Frontend

- **Framework**: SvelteKit 2 + Svelte 5 (Runes) + TypeScript Strict + Vite.
- **Renderização**: `@sveltejs/adapter-static` (Single Page Application desacoplada).
- **Design System & UI**: Tailwind 4, `bits-ui` (shadcn-svelte padrão), `lucide-svelte`, `svelte-sonner`, `mode-watcher`.
- **Formulários & Validação**: `sveltekit-superforms`, `formsnap`, `zod`.
- **Internacionalização**: `svelte-i18n` (padrão `pt-BR`).
- **Datas**: `date-fns`, `@internationalized/date`.
- **Testes**: `vitest` + `@testing-library/svelte` (unitários) e `playwright` (e2e).

### Backend & Persistência

- **Linguagem**: Rust (Cargo Workspace).
- **`crates/core`**: Modelos de domínio, entidades, regras de negócio (`CoreServices`), RBAC, Argon2id e traits assíncronas de persistência (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`). **Zero dependência de `sqlx` ou Tauri**.
- **`crates/store-sqlite`**: Implementação concreta das traits de persistência para SQLite via SQLx, migrações embutidas e execução da suíte de contrato (`run_store_contract_suite`).
- **`src-tauri`**: Aplicação Desktop Tauri 2; leitura pré-banco de `config.json` e comandos finos de IPC.
- **Banco de Dados**: SQLite (modo Local) com migrações automáticas embutidas (`sqlx::migrate!`) e índices únicos parciais (`WHERE deleted_at IS NULL`) compatíveis com PostgreSQL (modo Nuvem na Fase 2.5).

---

## 📋 Pré-requisitos

1. **Node.js**: v20+ (recomendado v22+ ou v24 LTS).
2. **pnpm**: v9+ ou v10+.
3. **Rust & Cargo**: v1.80+ (`rustup toolchain install stable`).
4. **Dependências de Build do Tauri**:
   - **Windows**: Microsoft C++ Build Tools e WebView2 Runtime.
   - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`.
   - **macOS**: Xcode Command Line Tools.

---

## 🚀 Como Executar o Projeto

### 1. Clonar e Instalar Dependências

```bash
git clone <url-do-repositorio>
cd eventrent
pnpm install
```

### 2. Executar em Modo de Desenvolvimento (Frontend Web)

```bash
pnpm dev
```

Acesse `http://localhost:5173` no navegador.

### 3. Executar em Modo Desktop (Tauri 2)

```bash
pnpm tauri dev
```

Abre a janela nativa do desktop com SQLite integrado no diretório de dados do app.

> **Aviso de Desenvolvimento (Fase 1B.0 — Congelamento de Migrações)**: As migrações `0001_init_tenant_and_branding.sql` e `0002_users_roles_and_audit.sql` foram atualizadas e consolidadas (renomeação da tabela `user` para `app_user`, tabela `audit_log` estritamente append-only protegida por triggers contra UPDATE/DELETE, e índices de performance). Bancos locais de desenvolvimento criados anteriormente (`eventrent.db`, `eventrent.db-wal`, `eventrent.db-shm` e `config.json` em `%APPDATA%\com.eventrent.app\` no Windows ou `~/.local/share/com.eventrent.app/` no Linux) **devem ser apagados** antes de executar a aplicação para evitar conflito de checksum no `sqlx::migrate!`. A partir da Fase 1B, as migrações 0001 e 0002 ficam permanentemente congeladas.

---

## 🧪 Como Executar os Testes e Verificações

Nenhum commit é integrado sem que todos os testes passem:

### 1. Checagem de Tipos e SvelteKit

```bash
pnpm check
```

### 2. Testes Unitários do Frontend (Vitest)

```bash
pnpm test:run
```

### 3. Testes End-to-End (Playwright)

```bash
pnpm test:e2e
```

### 4. Formatação e Linter do Rust

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
```

### 5. Testes Unitários do Backend (Rust em Memória)

```bash
cargo test --workspace
```

---

## 📦 Como Gerar o Build de Produção

### Build do Frontend Estático

```bash
pnpm build
```

Os artefatos estáticos são gerados no diretório `build/`.

### Build do Aplicativo Desktop Instalável

```bash
pnpm tauri build
```

Os executáveis/instaladores nativos (`.msi`, `.exe`, `.deb`, `.app`) são gerados em `target/release/bundle/`.

---

## 📂 Estrutura de Pastas

```
eventrent/
  AGENTS.md                         # Regras permanentes de engenharia para agentes
  README.md                         # Documentação de introdução e guia de execução
  docs/                             # Especificações e governança
    00-visao.md                     # Visão geral e objetivos de performance
    01-arquitetura.md               # Arquitetura e portas de dados
    02-modelo-dados.md              # Modelagem de tabelas e entidades
    03-regras-negocio.md            # Regras de domínio e integridade
    04-design-system.md             # Design tokens OKLCH e White Label
    05-roadmap.md                   # Fases de evolução do produto
    decisions/                      # Architecture Decision Records (ADRs)
    relatorios/                     # Relatórios de entrega de cada fase
  crates/core/                      # Domínio, CoreServices, RBAC, traits de store e suíte de contrato
  crates/store-sqlite/              # Implementação SQLite (SQLx) das traits e migrações SQL
  src-tauri/                        # App Tauri 2 e comandos finos IPC
  src/                              # Frontend SvelteKit 2
    lib/data/                       # Interfaces de repositório e transporte abstrato
    lib/ui/                         # Componentes base e design tokens
    lib/theme/                      # Utilitários de branding dinâmico
    lib/i18n/                       # Internacionalização (pt-BR)
    lib/components/                 # Shell (Sidebar, Header, CommandPalette)
    routes/                         # Rotas e páginas da aplicação
  .github/workflows/ci.yml          # Pipeline de integração contínua (Ubuntu & Windows)
```
