# Relatório de Conclusão — Fase 1A.1 (Segurança, Qualidade e Governança)

- **Data**: 05 de novembro de 2026
- **Branch**: `fase-1a-base`
- **Pull Request**: [#2 (Draft/Revisão)](https://github.com/FarrofaDeBacon/eventrent/pull/2)
- **Status**: Concluído — Pronto para revisão final do Diretor (merge e tag `v0.2.0-fase1a` aguardando aprovação explícita)

---

## 1. Reconhecimento de Desvio de Processo no Histórico Anterior

> [!WARNING]
> **Admissão de Desvio (Regra de Commits Pequenos)**:
> Reconhecemos formalmente que o commit [`9de63e2`](https://github.com/FarrofaDeBacon/eventrent/commit/9de63e2) na branch `fase-1a-base` foi excessivamente grande e agrupou num único commit a refatoração por traits (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`), a implementação do serviço de autenticação, o controle de acesso RBAC, a auditoria e as migrações SQL.
> Em estrito respeito às boas práticas de governança e integridade do repositório, **o histórico publicado não foi reescrito (sem `rebase -i` ou `push --force`)**. A partir da Fase 1A.1, o rigor de commits pequenos, atômicos e progressivos (Conventional Commits em português) com push imediato a cada item foi plenamente restabelecido.

---

## 2. Resumo das Correções Implementadas (Itens 1 a 7)

### Item 1 — `EVENTRENT_FORCE_DB_PATH` Estritamente Isolado em Desenvolvimento
- **Arquivo**: `src-tauri/src/lib.rs` (função `resolve_db_path`).
- **Implementação**: A variável de ambiente é lida exclusivamente sob `#[cfg(debug_assertions)]`. Em compilações de release (`cargo build --release` ou `pnpm tauri build`), o bloco condicional é completamente eliminado em tempo de compilação pelo compilador Rust, sendo impossível redirecionar o banco em produção.
- **Commit**: `42b4c6d fix(seguranca): restringe EVENTRENT_FORCE_DB_PATH a builds de desenvolvimento`.

### Item 2 — Chave de Recuperação Reforçada (80 bits, Rotação e Bloqueio)
- **Arquivos**: `crates/core/src/domain/user.rs`, `crates/core/src/services/mod.rs`, `crates/store-sqlite/migrations/0002_users_roles_and_audit.sql`, `crates/store-sqlite/src/lib.rs`.
- **Implementação**:
  1. **Entropia e Alfabeto**: Formato `ER-XXXX-XXXX-XXXX-XXXX` utilizando 32 caracteres seguros (`ABCDEFGHJKLMNPQRSTUVWXYZ23456789`), excluindo ambiguidades visuais (`0`, `O`, `1`, `I`). Cada caractere provê 5 bits, totalizando $16 \times 5 = 80$ bits de entropia gerados com `OsRng`.
  2. **Bloqueio Progressivo Persistido**: Criadas as colunas `recovery_failed_attempts` e `recovery_locked_until` na tabela `user`. A cada falha, aplica o mesmo backoff exponencial do login (5 falhas = 5 min, 6 = 10 min, 7 = 20 min, 8+ = 60 min).
  3. **Rotação Obrigatória**: Ao utilizar a chave com sucesso, a chave anterior é imediatamente invalidada no banco e uma **nova chave de 80 bits** é gerada de forma atômica e retornada na estrutura `RecoverPasswordResult { new_recovery_key }` para exibição única ao usuário.
  4. **Auditoria**: Tentativas falhas e sucessos gravam eventos em `audit_log` (`auth.recovery_failed`, `auth.recovery_locked`, `user.password_recovered`, `auth.recovery_key_rotated`).
  5. **Verificação Argon2id**: Todas as comparações são feitas exclusivamente contra o hash Argon2id da chave normalizada.

### Item 3 — Guarda Central de Permissões no Tauri e Testes Automatizados
- **Arquivos**: `src-tauri/src/guards.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/users.rs`, `src-tauri/src/commands/branding.rs`.
- **Implementação**:
  1. Criação da função central `require_session_and_permission(&state, required_permission) -> Result<SessionInfo, String>`.
  2. Todos os comandos não públicos (`users.rs`, `branding.rs`) foram refatorados para utilizar obrigatoriamente essa guarda.
  3. **Teste de Auditoria Estática**: O teste `test_command_source_files_use_central_guard` verifica que nenhum comando protegido deixa de chamar a guarda.
  4. **Auditoria do Registro**: `test_all_non_public_commands_have_required_permissions` e `test_public_commands_are_strictly_whitelisted` asseguram que apenas os comandos explicitamente públicos (setup, login, logout, recuperação e status) não exigem autenticação.
  5. **Expiração sem Sleep (Relógio Injetável)**: A função `evaluate_session_inactivity` foi testada com relógio simulado injetado (`test_session_inactivity_expiration_with_injectable_clock`), validando expiração aos 31 minutos sem necessidade de espera real.

### Item 4 — Argon2id com Parâmetros OWASP em `spawn_blocking`
- **Arquivos**: `crates/core/src/services/mod.rs`, `crates/core/src/testing.rs`.
- **Implementação**:
  1. Parâmetros configurados conforme recomendação do OWASP: $m = 19\text{ MiB}$ ($19.456\text{ KiB}$), $t = 2$ passes, $p = 1$ thread, salt criptográfico aleatório por hash.
  2. Funções `hash_secret` e `verify_secret` executam dentro de `tokio::task::spawn_blocking`, evitando qualquer bloqueio no runtime assíncrono ou congelamento na thread principal da interface.
  3. **Teste de Responsividade Assíncrona**: O teste `test_argon2id_spawn_blocking_does_not_block_async_runtime` comprova que enquanto o hash de 19 MiB é calculado, workers assíncronos no runtime Tokio continuam avançando e processando tarefas sem bloqueio perceptível.

### Item 5 — Divisão da Suíte de Contrato em Testes Granulares
- **Arquivos**: `crates/core/src/testing.rs`, `crates/store-sqlite/src/lib.rs`.
- **Implementação**: A suíte monolítica foi desmembrada em 10 funções contratuais granulares e independentes, cada uma executada por um `#[tokio::test]` próprio sobre banco SQLite em memória isolado:
  1. `test_contract_default_seeds`: idempotência de tenant padrão e marca inicial;
  2. `test_contract_system_roles`: garantia idempotente dos 5 perfis de sistema;
  3. `test_contract_initial_setup_and_recovery_key`: setup do proprietário, 80 bits de entropia e conflito em segundo setup;
  4. `test_contract_last_owner_protection`: proteção contra autodesativação e rebaixamento do último proprietário ativo;
  5. `test_contract_user_creation_and_rbac`: criação de comercial e checagem de negação `Forbidden` via RBAC;
  6. `test_contract_user_lockout`: bloqueio após 5 falhas consecutivas e rejeição com `AccountLocked`;
  7. `test_contract_password_reset_clears_lockout`: desbloqueio após redefinição administrativa de senha;
  8. `test_contract_recovery_key_rotation_and_lockout`: bloqueio progressivo na recuperação, rotação de chave e invalidação da chave antiga;
  9. `test_contract_audit_log`: persistência append-only de todas as ações de auditoria;
  10. `test_contract_unique_partial_indexes`: garantia de unicidade parcial `WHERE deleted_at IS NULL`;
  11. `test_sqlite_store_contract_suite`: teste agregador preservado para futuras execuções com `store-postgres`.

### Item 6 — Documentação de Modelo de Ameaças e Limites do SQLite
- **Arquivo**: `docs/seguranca.md`.
- **Implementação**: Registro honesto de que o banco SQLite local não é criptografado em repouso, documentando o que a autenticação protege (uso da aplicação, isolamento de papéis RBAC, força bruta online, senhas com Argon2id) e o que ela não protege (acesso físico direto ao arquivo `.db`, administrador do SO), registrando o SQLCipher como pendência futura no roadmap.

---

## 3. Resultados dos Testes Automatizados

### Backend Rust (`cargo test --workspace`)
```text
running 4 tests (eventrent-core)
test testing::unit_tests::test_lockout_backoff_progression ... ok
test testing::unit_tests::test_recovery_key_format_and_uniqueness ... ok
test testing::unit_tests::test_argon2id_spawn_blocking_does_not_block_async_runtime ... ok
test testing::unit_tests::test_argon2id_hash_and_verify ... ok
test result: ok. 4 passed; 0 failed

running 12 tests (eventrent-store-sqlite)
test tests::test_contract_default_seeds ... ok
test tests::test_contract_unique_partial_indexes ... ok
test tests::test_contract_system_roles ... ok
test tests::test_init_db_with_spaces_and_accents_in_path ... ok
test tests::test_contract_initial_setup_and_recovery_key ... ok
test tests::test_contract_last_owner_protection ... ok
test tests::test_contract_user_creation_and_rbac ... ok
test tests::test_contract_recovery_key_rotation_and_lockout ... ok
test tests::test_contract_user_lockout ... ok
test tests::test_contract_password_reset_clears_lockout ... ok
test tests::test_sqlite_store_contract_suite ... ok
test tests::test_contract_audit_log ... ok
test result: ok. 12 passed; 0 failed

running 4 tests (eventrent-app-lib / guards)
test guards::tests::test_all_non_public_commands_have_required_permissions ... ok
test guards::tests::test_public_commands_are_strictly_whitelisted ... ok
test guards::tests::test_session_inactivity_expiration_with_injectable_clock ... ok
test guards::tests::test_command_source_files_use_central_guard ... ok
test result: ok. 4 passed; 0 failed
```
**Total Backend: 20 testes Rust aprovados com sucesso.**

### Frontend (`pnpm run test:run`)
```text
Test Files  2 passed (2)
     Tests  7 passed (7)
  Duration  13.85s
```
**Total Frontend: 7 testes Vitest aprovados com sucesso.**

---

## 4. Histórico de Commits da Fase 1A.1
```text
7044094 docs(seguranca): documenta modelo de ameacas, limites do sqlite local e criptografia em repouso
0f9a754 test(store): divide suite de contrato em testes isolados por comportamento
9966bf1 feat(desktop): implementa guarda central de permissoes e auditoria de comandos
5724da4 feat(auth): reforca chave de recuperacao e executa Argon2id em spawn_blocking com parametros OWASP
e4699ec refactor(ui): alinha catalogo de permissoes e perfis com o core
42b4c6d fix(seguranca): restringe EVENTRENT_FORCE_DB_PATH a builds de desenvolvimento
```
