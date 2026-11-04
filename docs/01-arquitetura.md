# 01 - Arquitetura de Software: EventRent

## 1. Visão Geral

A arquitetura do EventRent adota uma estratégia **Desktop First** com **Tauri 2**, projetada para transição transparente para uma aplicação Web SaaS / Online no futuro sem reescrita de código da interface ou das regras de negócio.

```
+-------------------------------------------------------------------+
|                           FRONTEND                                |
|  SvelteKit 2 + Svelte 5 (Runes) + Tailwind 4 + Bits-UI (Static SPA)|
|                                                                   |
|   +-------------------+        +------------------------------+   |
|   |  Rotas / Páginas  |  --->  |    src/lib/data (Ports)      |   |
|   +-------------------+        +------------------------------+   |
+---------------------------------------------------|---------------+
                                                    |
                       +----------------------------+
                       | Implementação de Transporte
                       v
       +-------------------------------+
       | src/lib/data/tauri_impl.ts     | (Fase atual: Tauri invoke)
       | (Futuro: http_impl.ts via API)|
       +---------------+---------------+
                       |
=======================|=============================================
                       | IPC Tauri
                       v
+-------------------------------------------------------------------+
|                        BACKEND (RUST)                             |
|                                                                   |
|  [src-tauri] (Bin Crate)                                          |
|    - Comandos finos IPC (handlers de comando)                    |
|    - Inicialização de janela, atalhos nativos, SQLite path        |
|                                                                   |
|  [crates/core] (Lib Crate - Domínio e Persistência)               |
|    - Modelos de Domínio e Entidades                               |
|    - Serviços de Negócio (reservas, cálculos, validações)         |
|    - Repositórios com SQLx e Migrações versionadas                |
|    - Zero acoplamento com Tauri                                   |
+------------------------------+------------------------------------+
                               |
                               v
                     +-------------------+
                     |  SQLite Database  | (Local Desktop)
                     | (Futuro: Postgres)|
                     +-------------------+
```

## 2. Camadas do Frontend

1. **Views e Rotas (`src/routes`)**:
   - Componentes Svelte 5 utilizando Runes (`$state`, `$derived`, `$props`).
   - Nenhuma rota realiza requisição direta a IPC, HTTP ou SQLite.
2. **Camada de Acesso a Dados (`src/lib/data`)**:
   - Declara contratos através de interfaces TypeScript estritas (ex.: `BrandingRepository`).
   - Fornece um injetor ou instância padrão do repositório.
   - Atualmente, as implementações utilizam `@tauri-apps/api/core` `invoke`. A substituição para uma implementação de rede remota (`fetch` para API REST ou gRPC-web) não requer nenhuma alteração nas páginas ou componentes.
3. **Design System & Tokens (`src/lib/ui`, `src/lib/theme`)**:
   - Baseado em tokens CSS (OKLCH). Nenhuma cor arbitrária no HTML.
   - White-label gerenciado via injeção dinâmica de CSS variables no `:root`.

## 3. Camadas do Backend (Rust Workspace)

1. **`crates/core`**:
   - Crate independente, sem referências ao Tauri. Pode ser compilado como biblioteca estática, daemon de servidor web (ex.: Axum/Actix) ou CLI de testes.
   - Gerencia conexões e pool de conexões SQLite via `sqlx`.
   - Executa migrações automáticas integradas (`sqlx::migrate!`).
2. **`src-tauri`**:
   - Camada fina responsável por expor comandos Tauri que delegam operações a instâncias gerenciadas em estado Tauri (`tauri::State`).

## 4. Portabilidade do Banco de Dados

- Utiliza SQLite com modo WAL (`Write-Ahead Logging`) e chaves estrangeiras ativadas (`PRAGMA foreign_keys = ON`).
- Dialeto SQL portável para assegurar migração suave para PostgreSQL na Fase 9.
