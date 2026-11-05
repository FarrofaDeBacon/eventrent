# 01 - Arquitetura de Software: EventRent

## 1. Visão Geral

A arquitetura do EventRent adota dois modos de operação escolhidos na instalação (**Modo Local** com SQLite em um único computador e **Modo Nuvem** com servidor central `axum` + PostgreSQL na Fase 2.5), mantendo o núcleo de regras de negócio (`crates/core`) e o frontend (`src/`) 100% independentes do transporte e do banco de dados.

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
       | src/lib/data/tauri-*.ts       | (Modo Local: Tauri invoke)
       | (Fase 2.5: http-*.ts via API) |
       +---------------+---------------+
                       |
=======================|=============================================
                       | IPC Tauri (ou HTTP na Fase 2.5)
                       v
+-------------------------------------------------------------------+
|                        BACKEND (RUST)                             |
|                                                                   |
|  [src-tauri] (Bin Crate - Aplicação Desktop)                      |
|    - Lê <app_data_dir>/config.json antes de abrir o banco         |
|    - Monta StoreBundle (Arc<dyn ...>) e CoreServices no AppState  |
|    - Comandos finos IPC delegando para CoreServices               |
|                                                                   |
|  [crates/core] (Lib Crate - Domínio, Serviços e Ports)            |
|    - Entidades de Domínio, Permissões e Regras de Negócio         |
|    - CoreServices (Auth Argon2id, RBAC, Lockout, Auditoria)       |
|    - Traits async: TenantStore, BrandingStore, RoleStore,         |
|      UserStore, AuditStore                                        |
|    - Suíte genérica de testes de contrato                         |
|    - Zero dependência de Tauri e zero dependência de sqlx         |
+------------------------------+------------------------------------+
                               |
             +-----------------+-----------------+
             |                                   |
             v                                   v
+-------------------------+         +-------------------------+
|  [crates/store-sqlite]  |         | [crates/store-postgres] |
|  SQLite + Migrations    |         | PostgreSQL (Fase 2.5)   |
+-------------------------+         +-------------------------+
```

## 2. Camadas do Frontend

1. **Views e Rotas (`src/routes`)**:
   - Componentes Svelte 5 utilizando Runes (`$state`, `$derived`, `$props`).
   - Nenhuma rota realiza requisição direta a IPC, HTTP ou SQLite.
2. **Camada de Acesso a Dados (`src/lib/data`)**:
   - Declara contratos através de interfaces TypeScript estritas (`BrandingRepository`, `AuthRepository`, `UserRepository`).
   - No modo Desktop utiliza `@tauri-apps/api/core` `invoke`; em navegador/testes utiliza adaptadores mock em memória; na Fase 2.5 adicionará adaptadores HTTP sem alterar nenhuma tela.
3. **Design System & Tokens (`src/lib/ui`, `src/lib/theme`)**:
   - Baseado em tokens CSS (OKLCH). Nenhuma cor arbitrária no HTML.
   - White-label gerenciado via injeção dinâmica de CSS variables no `:root`.

## 3. Camadas do Backend (Rust Workspace)

1. **`crates/core`**:
   - Crate independente de banco de dados e de interface gráfica.
   - Define as traits de persistência (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`) e concentra toda a lógica de autenticação (Argon2id, chave de recuperação, bloqueio progressivo), verificação de permissões (`resource:action`) e regras de domínio (proteção do último Proprietário, proibição de autodesativação, geração de `audit_log`).
2. **`crates/store-sqlite`**:
   - Implementa todas as traits de store do `crates/core` usando SQLite (`sqlx`) com modo WAL, `foreign_keys = ON` e abertura segura por `Path` (`SqliteConnectOptions::new().filename(path)`).
   - Contém as migrações versionadas em `crates/store-sqlite/migrations`.
3. **`src-tauri`**:
   - Lê `config.json` na pasta de dados do app antes de inicializar o banco.
   - Configura logs rotativos diários (`tracing-appender`), instância única (`tauri-plugin-single-instance`) e diálogo nativo de erro não bloqueante (`tauri-plugin-dialog`).
   - Mantém `CoreServices` e o estado de sessão no `AppState`.
