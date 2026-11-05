# Relatório de Conclusão da Fase 1A — Fundações de Dados, Usuários, Permissões e Escolha de Modo

- **Projeto**: EventRent (Sistema de Gestão de Locação para Eventos)
- **Repositório**: `https://github.com/FarrofaDeBacon/eventrent`
- **Engenheiro de Implementação**: Antigravity
- **Diretor de Arquitetura**: Claude
- **Branch**: `fase-1a-base`
- **Base (`main`)**: `v0.1.0-fase0` (`ea5b818`)
- **Data**: Novembro de 2026

---

## 1. O que foi feito (por item do escopo, com status)

| Tarefa  | Descrição do Escopo                                                                                                                |    Status     | Detalhes                                                                                                                                                                                                                                                                                                                     |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------- | :-----------: | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **0.1** | Editar migração `0001` para substituir `UNIQUE` em linha por índices únicos parciais `WHERE deleted_at IS NULL` + aviso no README  | **Concluído** | Removidos `UNIQUE` em linha de `tenant.slug` e `tenant_branding.tenant_id` em `crates/store-sqlite/migrations/0001_init_tenant_and_branding.sql` e criados os índices parciais `idx_tenant_slug_active` e `idx_tenant_branding_tenant_active`. Aviso adicionado ao `README.md`.                                              |
| **0.2** | Teste prático do diálogo de erro de inicialização + substituição de `blocking_show()` por abordagem não-bloqueante                 | **Concluído** | `blocking_show()` foi substituído por `.show(move \|_\| { exit_handle.exit(1); })` não-bloqueante em `src-tauri/src/lib.rs`, ocultando a janela principal (`window.hide()`). Teste prático executado forçando falha via `EVENTRENT_FORCE_DB_PATH=D:\eventrent\src`, confirmando `Class=#32770 Visible=True` e `ExitCode: 1`. |
| **0.3** | Trocar nome padrão da marca de `"EventRent Audiovisual"` para `"Minha empresa"`                                                    | **Concluído** | Alterado em `crates/core/src/services/mod.rs` (`DEFAULT_COMPANY_NAME = "Minha empresa"`), `src/lib/data/types.ts`, `src/lib/i18n/locales/pt-BR.json` e todas as telas e componentes do frontend.                                                                                                                             |
| **1.1** | Definir no `crates/core` traits assíncronas de repositório sem depender de `sqlx`                                                  | **Concluído** | `crates/core` possui zero dependência de `sqlx`. Traits assíncronas (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`) definidas em `crates/core/src/store/mod.rs` recebendo e devolvendo tipos de domínio puros.                                                                                      |
| **1.2** | Criar crate `crates/store-sqlite` implementando as traits do `core` com `sqlx` e migrações                                         | **Concluído** | Criado `crates/store-sqlite` com `init_db(&Path)`, migrações `0001` e `0002` e implementação completa de `SqliteStore` para as 5 traits de persistência.                                                                                                                                                                     |
| **1.3** | Suíte de testes de contrato genérica (`run_store_contract_suite`) no `core` rodada pelo `store-sqlite`                             | **Concluído** | Criada `run_store_contract_suite` em `crates/core/src/testing.rs`, executada contra SQLite em memória no teste `tests::test_sqlite_store_contract_suite` em `crates/store-sqlite/src/lib.rs`. Pronta para rodar contra `store-postgres` na Fase 2.5 sem alterações.                                                          |
| **1.4** | Migrar repositório de branding e `ensure_default_tenant` para o modelo de traits e atualizar `src-tauri`                           | **Concluído** | `ensure_default_tenant` e `ensure_system_roles` operam em `crates/core/src/services/mod.rs` sobre traits; `src-tauri` injeta `StoreBundle` (`SqliteStore::new(pool).into_bundle()`) em `CoreServices`.                                                                                                                       |
| **1.5** | Documentar em `ADR-0006` e `ADR-0007` e atualizar `AGENTS.md` e `docs/01`, `02`, `03` e `05`                                       | **Concluído** | Criados `docs/decisions/0006-dois-modos-operacao-local-e-nuvem.md` e `docs/decisions/0007-arquitetura-repositorios-por-trait-e-stores.md`; atualizados `AGENTS.md`, `docs/01-arquitetura.md`, `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md` e `docs/05-roadmap.md`.                                                 |
| **2.1** | Guardar modo de operação em `<app_data_dir>/config.json` fora do banco, lido antes de abrir o SQLite                               | **Concluído** | `load_or_create_app_config` em `src-tauri/src/lib.rs` lê/cria `<app_data_dir>/config.json` antes de inicializar o SQLite.                                                                                                                                                                                                    |
| **2.2** | Tela de primeira execução (`/setup`) com escolha de modo (`Neste computador` ativo vs `Em mais de um computador (nuvem)` em breve) | **Concluído** | Implementada em `src/routes/setup/+page.svelte` com textos em i18n (`pt-BR.json`) e tokens semânticos OKLCH.                                                                                                                                                                                                                 |
| **2.3** | Criação da conta do Proprietário + geração única da Chave de Recuperação (`ER-XXXX-XXXX-XXXX-XXXX`, hash Argon2id no banco)        | **Concluído** | Geração criptográfica via `OsRng` em `crates/core/src/services/mod.rs`, persistência exclusiva do hash Argon2id em `user.recovery_key_hash`, exibição única com confirmação obrigatória em `/setup` e modal de recuperação de senha em `/login`.                                                                             |
| **3.1** | Migração `0002_users_roles_and_audit.sql` com índices únicos parciais `WHERE deleted_at IS NULL`                                   | **Concluído** | Tabelas `role`, `role_permission`, `user` e `audit_log` criadas em `crates/store-sqlite/migrations/0002_users_roles_and_audit.sql` com todos os campos padrão e índices únicos parciais.                                                                                                                                     |
| **3.2** | Permissões como constantes (`resource:action`) no `core` + 5 perfis de sistema idempotentes documentados em `docs/03`              | **Concluído** | 21 constantes de permissão em `crates/core/src/domain/permission.rs`; 5 perfis (`Proprietário`, `Comercial`, `Estoque`, `Técnico`, `Financeiro`) semeados por `ensure_system_roles` e documentados em `docs/03-regras-negocio.md`.                                                                                           |
| **3.3** | Hash `Argon2id`, zero vazamento de segredos em logs, bloqueio progressivo após 5 falhas e sessão com expiração por inatividade     | **Concluído** | Hashes `Argon2id`, bloqueio progressivo (`5 falhas = 5 min`, `6 = 10 min`, `7 = 20 min`, `8+ = 60 min`) e controle de inatividade de sessão em `AppState` (`touch_and_get_session`).                                                                                                                                         |
| **3.4** | Verificação de permissão no `core`, proteção do último Proprietário ativo, proibição de autodesativação e gravação em `audit_log`  | **Concluído** | Todas as regras aplicadas em `CoreServices` (`crates/core/src/services/mod.rs`) e cobertas pela suíte de contrato e testes unitários.                                                                                                                                                                                        |
| **3.5** | Telas `/login`, `/setup`, `/configuracoes/usuarios`, `/configuracoes/perfis`, menu/command palette filtrados e usuário no header   | **Concluído** | Implementadas todas as telas, modais de criação/edição/redefinição de senha, exibição do usuário logado e botão `Sair` no cabeçalho, além de filtragem por permissão na barra lateral e no `Ctrl+K`.                                                                                                                         |

---

## 2. Resultado do Teste Prático do Diálogo de Erro de Inicialização (Tarefa 0.2)

### Causa do Risco Identificado na Fase 0.2

Na implementação anterior, `blocking_show()` do `tauri-plugin-dialog` era invocado dentro do hook `.setup(|app| ...)` do Tauri, que executa na thread principal antes do início do loop de eventos nativo (`Tao`). Internamente, `blocking_show()` despacha a criação do diálogo via `app.run_on_main_thread(...)` e bloqueia a thread atual aguardando um canal (`rx.recv()`), o que pode causar deadlock quando chamado na própria thread principal antes do event loop.

### Solução Implementada em `src-tauri/src/lib.rs`

1. Quando `initialize_app(&app_handle)` retorna `Err((err_msg, log_path_display))`:
   - O erro completo é gravado no arquivo de log diário (`tracing::error!("Inicialização abortada: {}", err_msg)`).
   - A janela principal WebView2 (`main`) é imediatamente ocultada (`window.hide()`).
   - O diálogo nativo é aberto de forma **não-bloqueante** com `.show(move |_| { exit_handle.exit(1); })`, retornando `Ok(())` no `setup` para permitir que o event loop inicie, exiba a janela modal `#32770` do Windows e encerre o processo com código `1` assim que o usuário fechar o diálogo.

### Procedimento e Evidência do Teste Prático

Forçamos a falha de inicialização apontando `EVENTRENT_FORCE_DB_PATH` para um diretório existente (`D:\eventrent\src`), fazendo com que o SQLite falhe com `(code: 526) unable to open database file`:

1. **Registro gravado em `%APPDATA%\com.eventrent.app\logs\eventrent.log.2026-11-04`**:
   ```text
   2026-11-04T23:44:58.617748Z  INFO eventrent_app_lib: Inicializando banco SQLite em: D:\eventrent\src
   2026-11-04T23:44:58.618658Z ERROR eventrent_app_lib: Erro crítico ao inicializar banco SQLite: Erro de banco de dados: error returned from database: (code: 526) unable to open database file
   2026-11-04T23:44:58.618956Z ERROR eventrent_app_lib: Inicialização abortada: Falha ao inicializar ou migrar o banco de dados local: Erro de banco de dados: error returned from database: (code: 526) unable to open database file
   ```
2. **Inspeção de janelas Win32 (`EnumWindows`) e encerramento controlado**:
   ```text
   HWND=787998 Visible=True Class=#32770 Title=EventRent — Erro de Inicialização
   HWND=2494338 Visible=False Class=Tauri Window Title=EventRent — Gestão de Locação para Eventos
   HasExited: True, ExitCode: 1
   ```
   A janela principal (`Class=Tauri Window`) permaneceu oculta (`Visible=False`), a caixa de diálogo nativa do Windows (`Class=#32770`) foi exibida (`Visible=True`) sem travar a thread principal, e ao fechar o diálogo o processo encerrou limpamente com `ExitCode: 1`.

---

## 3. Verificações de Qualidade e Testes Executados

### Backend Rust (`cargo`)

- **`cargo fmt --all --check`**: Passou sem divergências.
- **`cargo clippy --workspace --all-targets -- -D warnings`**: Passou com **0 avisos e 0 erros**.
- **`cargo test --workspace`**:
  - `crates/core` (3 testes unitários):
    - `testing::unit_tests::test_argon2id_hash_and_verify` ... `ok`
    - `testing::unit_tests::test_recovery_key_format_and_uniqueness` ... `ok`
    - `testing::unit_tests::test_lockout_backoff_progression` ... `ok`
  - `crates/store-sqlite` (2 testes de integração e contrato):
    - `tests::test_init_db_with_spaces_and_accents_in_path` ... `ok`
    - `tests::test_sqlite_store_contract_suite` (executa `run_store_contract_suite` validando todas as 5 traits, idempotência de seeds, Argon2id, chave de recuperação, bloqueio após 5 falhas, permissões, proteção do último proprietário, proibição de autodesativação e `audit_log`) ... `ok`

### Frontend (`pnpm`)

- **`pnpm check`**: `svelte-check found 0 errors and 0 warnings`.
- **`pnpm lint`**: `prettier --check . && eslint .` passou sem erros.
- **`pnpm test:run` (Vitest)**: 2 suítes (`branding-repository.spec.ts` e `auth-and-users.spec.ts`), **7 testes passando**.
- **`pnpm test:e2e` (Playwright)**: `e2e/smoke.spec.ts` validando o fluxo ponta a ponta (`/` -> redirecionamento para `/setup` -> escolha de modo local -> criação do proprietário -> confirmação da chave de recuperação -> logout -> `/login` -> criação de usuário em `/configuracoes/usuarios` -> consulta em `/configuracoes/perfis` -> atualização de marca em `/configuracoes/aparencia`), **100% aprovado**.

---

## 4. Histórico de Commits (`git log --oneline`)

- `ea5b818` `Merge pull request #1 from FarrofaDeBacon/fase-0-fundacao` (tag `v0.1.0-fase0`)
- `018f8e3` `docs(arquitetura): adiciona ADR-0006 e ADR-0007 e atualiza modelagem e regras da Fase 1A`
- `9de63e2` `feat(core): desacopla sqlx por traits em store-sqlite e implementa auth, RBAC e auditoria`
- `01d8cfb` `feat(data): adiciona contratos e adaptadores de autenticacao, setup, usuarios e perfis`
- `0fa6703` `feat(ui): adiciona telas de setup inicial, login, usuarios, perfis e guards de permissao`
- `414318a` `test(e2e): valida fluxo completo de primeira execucao, login, usuarios, perfis e aparencia`

---

## 5. Desvios e Observações

- **Nenhum desvio de escopo ou dependência**: Apenas as dependências autorizadas pelo prompt (`async-trait`, `argon2`, `rand`, `serde_json`) foram adicionadas, documentadas nos ADRs `0006` e `0007`.
