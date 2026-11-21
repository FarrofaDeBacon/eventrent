# Relatório da Fase 2S — Correções de Segurança e Confiabilidade (Auditoria Externa e Revisão da 2M)

**Branch:** `fase-2s-auditoria`  
**Base:** `main` (`b9a0e1e` / `v0.9.0-fase2m`)  
**Status:** Concluída. `pnpm verify` 10/10 verde em todos os commits; Playwright E2E habilitado para Windows e Linux no CI; sem merge e sem criação de tag.

---

## 1. Resumo Executivo

A Fase 2S consolidou correções de segurança em profundidade, estabilidade criptográfica, consistência de contratos IPC e governança arquitetural com base no relatório de auditoria externa e na revisão técnica da Fase 2M.

As intervenções foram divididas em dois blocos complementares:

1. **Bloco B (B1 a B6):** Correções de confiabilidade e mitigação de vetores no subsistema de arquivos temporários e armazenamento de mídia (`crates/media-fs`, `src-tauri/src/commands/media.rs` e `crates/core/src/domain/validation.rs`).
2. **Bloco S (S1 a S6):** Correções de segurança estrutural em contratos IPC, rotação e exibição segura de chaves de recuperação, revalidação dinâmica de sessões no banco via `auth_epoch`, CSP estrita sem scripts inline, isolamento de mocks fora da produção, auditoria da documentação com validador automatizado de permissões e definição de pronto permanente em 4 níveis.

---

## 2. Matriz de Rastreabilidade

| Requisito | Descrição                                                                                                                                                                                                                                                                              | Commit    | Teste Automatizado que Comprova                                                                                                                                           |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **B1**    | Validação antecipada da extensão `.pdf`, componente único de caminho e contenção em pasta temporária antes de qualquer gravação em disco                                                                                                                                               | `4cb5aea` | `commands::media::tests::test_write_sanitized_temp_document_fails_early_and_writes_nothing_to_disk` em `src-tauri`                                                        |
| **B2**    | Propagação de erro em `compute_sha256_hex_async` e cálculo assíncrono único de SHA-256 no upload de PDF                                                                                                                                                                                | `7f5dc6d` | `test_fs_media_store_atomic_write_dedup_and_missing_file` em `crates/media-fs` e `test_media_core_validators_magic_exif_limits_and_filenames` em `crates/core`            |
| **B3**    | Renovação de timestamp de modificação (`mtime`) com `SystemTime::now()` ao reenviar blob pré-existente para evitar coleta prematura pelo GC                                                                                                                                            | `5635a66` | `test_fs_media_store_gc_reconciliation_and_temp_cleanup` em `crates/media-fs`                                                                                             |
| **B4**    | Lista ampliada de nomes reservados do Windows (28 entradas, incluindo `COM¹..³` e `LPT¹..³`) e restrição de stem a letras e dígitos ASCII                                                                                                                                              | `5bb0c43` | `commands::media::tests::test_write_sanitized_temp_document_r1_security_cases` em `src-tauri`                                                                             |
| **B6**    | Rejeição estendida de caracteres invisíveis, formatação bidi e tags Unicode (`\u{061C}`, `\u{180E}`, `\u{00AD}`, `\u{2060}`..=`\u{2064}`, `\u{206A}`..=`\u{206F}`, `\u{3164}`, `\u{2800}`, `\u{E0000}`..=`\u{E007F}`), preservando seletores de variação emoji `\u{FE00}`..=`\u{FE0F}` | `72606eb` | `testing::unit_tests::test_media_core_validators_magic_exif_limits_and_filenames` em `crates/core`                                                                        |
| **S1**    | Recuperação de senha devolvendo `{ new_recovery_key }` exibida em modal bloqueante com confirmação digitada ("Guardei a chave"), sem persistência em storage, com regeneração autenticada em "Minha Conta"                                                                             | `93779f3` | `e2e/recovery-and-account.spec.ts` (Playwright) e `test_contract_recovery_key_rotation_and_lockout` em `crates/store-sqlite`                                              |
| **S2**    | Fixtures canônicas de contrato versionadas para todos os 62 comandos IPC do Tauri, validadas bidirecionalmente por Rust e TypeScript, com spike no ADR-0016                                                                                                                            | `ba344b7` | `contracts::tests::test_all_commands_have_up_to_date_contract_fixtures` em `src-tauri` e `contracts.spec.ts` (Vitest)                                                     |
| **S3**    | Revalidação dinâmica de sessão e permissões do usuário a cada comando protegido através da coluna `auth_epoch` (migração `0010_auth_epoch.sql`)                                                                                                                                        | `9d57830` | `tests::test_auth_epoch_invalidation_and_dynamic_permissions` em `crates/store-sqlite` e `tests::test_get_current_session_does_not_renew_last_activity_at` em `src-tauri` |
| **S4**    | CSP de produção estrita (`script-src 'self'`), migração de script inline anti-FOUC para `static/anti-fouc.js`, teste unitário de CSP em Rust e justificativa de `'unsafe-inline'` para `style-src` no ADR-0016                                                                         | `75dea17` | `guards::tests::test_tauri_csp_is_strict_and_free_of_unsafe_eval` em `src-tauri/src/guards.rs`                                                                            |
| **S5**    | Exclusão de mocks no build de produção com erro explícito fora do Tauri runtime e exibição de tarja visual de aviso em modo demonstração no navegador                                                                                                                                  | `4b2f1d1` | Guardas em `src/lib/data/mock-*.ts`, testes unitários e suíte E2E Playwright                                                                                              |
| **S6**    | Auditoria de menções a Fastify -> Axum, validador automatizado de permissões em Rust contra `ALL_PERMISSIONS`, Playwright no Windows no CI e DoD em 4 níveis no `AGENTS.md`                                                                                                            | _(atual)_ | `domain::permission::tests::test_all_permissions_in_documentation_match_canonical_catalog` em `crates/core`                                                               |

---

## 3. Evidências dos Requisitos Críticos

### 3.1. S1 — Recuperação de Senha e Nova Chave de Recuperação

A auditoria externa constatou que o método `recoverPasswordWithKey` no TypeScript descartava silenciosamente o retorno do comando `recover_password`, tipando-o como `Promise<void>`.

A correção implementou o contrato completo:

- Interface `AuthRepository.recoverPasswordWithKey` retorna `Promise<RecoverPasswordResult>`.
- O Tauri IPC e o mock retornam `{ new_recovery_key: string }`.
- A tela de login (`src/routes/login/+page.svelte`) intercepta o retorno e dispara o modal bloqueante `src/lib/components/recovery-key-modal.svelte`.
- O usuário deve confirmar que salvou a chave (com botões de cópia e download `.txt`) e digitar explicitamente a frase de confirmação antes de poder prosseguir.
- A chave reside exclusivamente no estado reativo em memória do componente e nunca é persistida em `localStorage`, `sessionStorage` ou logs.

### 3.2. S2 — Fixtures de Contrato e Spike de Tipagem IPC

Para eliminar qualquer drift silencioso entre os 62 comandos expostos pelo Tauri e os métodos da camada de dados do TypeScript:

- Cada comando possui uma fixture canônica em `src/lib/data/fixtures/contracts/<comando>.json`.
- O teste Rust `test_all_commands_have_up_to_date_contract_fixtures` garante que nenhum comando existe sem fixture e nenhuma fixture é órfã.
- O teste Vitest `src/lib/data/contracts.spec.ts` valida que comandos produtores de dados não retornam `void` no TypeScript e que retornos críticos contêm campos obrigatórios.
- O ADR-0016 documentou a comparação técnica entre `ts-rs`, `tauri-specta` e Fixtures + Vitest, justificando a adoção das fixtures para manter o desacoplamento hexagonal da arquitetura.

### 3.3. S3 — Revalidação por `auth_epoch` e Permissões Dinâmicas

- A migração `0010_auth_epoch.sql` adicionou a coluna `auth_epoch INTEGER NOT NULL DEFAULT 0` à tabela `app_user`.
- Qualquer evento de segurança (alteração de senha, recuperação por chave, regeneração de chave, alteração de perfil/permissões ou desativação de conta) incrementa atomicamente o contador `auth_epoch`.
- A guarda central `require_session(&state).await?` em `src-tauri` invoca `revalidate_session`, que recarrega os dados do usuário e de seu perfil no banco SQLite.
- Divergências de `auth_epoch` ou desativação da conta revogam instantaneamente a sessão com erro `Unauthorized`, impedindo uso de tokens ou estados locais obsoletos.

### 3.4. S4 — CSP Estrita e Eliminação de Scripts Inline

- `src-tauri/tauri.conf.json` recebeu a diretiva estrita:
  ```
  default-src 'self'; script-src 'self'; img-src 'self' blob: data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'; style-src 'self' 'unsafe-inline'
  ```
- O script inline anti-FOUC de tema foi extraído de `src/app.html` e alocado no arquivo estático `static/anti-fouc.js`, sendo referenciado via `<script src="%sveltekit.assets%/anti-fouc.js"></script>`.
- O teste unitário `test_tauri_csp_is_strict_and_free_of_unsafe_eval` valida em tempo de compilação que a CSP não contém `'unsafe-eval'`, não possui curingas em `script-src` e restringe fontes de script a `'self'`.

### 3.5. S6 — Validador Automatizado de Permissões da Documentação

- O teste unitário Rust `test_all_permissions_in_documentation_match_canonical_catalog` em `crates/core/src/domain/permission.rs` analisa `docs/03-regras-negocio.md` e `docs/06-requisitos-de-produto.md`.
- Ele extrai todas as ocorrências de `recurso:ação` e valida que 100% delas constam no array canônico `ALL_PERMISSIONS`, bloqueando documentação com permissões obsoletas ou fictícias.

---

## 4. Conformidade com as Regras da Fase

- **Migrações 0001 a 0009:** Intactas e rigorosamente congeladas. Prova comprovada pelo teste `scripts/test-check-migrations.js` (6/6 cenários aprovados). A única migração adicionada é a `0010_auth_epoch.sql`, contendo exclusivamente `ALTER TABLE app_user ADD COLUMN auth_epoch INTEGER NOT NULL DEFAULT 0;`.
- **Dependências:** Nenhuma crate externa nova adicionada ao Cargo workspace; nenhuma dependência npm externa nova sem ADR.
- **Commits:** Conventional Commits em português, um commit por unidade lógica com `pnpm verify` 100% verde antes de cada commit.
- **Sem Merge e Sem Tag:** O pull request permanece aberto e sem criação de tags de versão.
