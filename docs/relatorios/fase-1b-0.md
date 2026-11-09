# Relatório de Conclusão — Fase 1B.0 (Transações nas Stores e Ajustes de Revisão)

- **Data**: 09 de novembro de 2026
- **Branch**: `fase-1b0-transacoes`
- **Pull Request**: [#4](https://github.com/FarrofaDeBacon/eventrent/pull/4)
- **Status**: Concluída e Aprovada em CI Multiplataforma (Linux Ubuntu e Windows)

---

## 1. Sumário Executivo

A Fase 1B.0 consolidou a infraestrutura transacional do EventRent e resolveu com rigor todos os apontamentos de segurança e arquitetura levantados na revisão do PR #3. As cinco stores da aplicação (`tenants`, `branding`, `roles`, `users`, `audit`) foram unificadas sob a abstração de transações gerenciada por `TransactionManager` e `run_in_tx`, garantindo atomicidade total, reversão automática via rollback (inclusive para auditoria em caso de falha), tentativas limitadas com backoff exponencial sob contenção transitória (`TransientConflict`), e persistência com commit imediato de eventos de falha de autenticação e bloqueio temporário (mitigando perda de auditoria de segurança).

As migrações fundamentais `0001_init_tenant_and_branding.sql` e `0002_users_roles_and_audit.sql` foram congeladas em definitivo, com verificação automatizada adicionada ao pipeline de Integração Contínua (CI) e documentada no `AGENTS.md`.

---

## 2. Tabela de Rastreabilidade

| Item do Prompt | Descrição                                                                                                         | Commit SHA | Arquivos Alterados                                                                                        | Teste Associado                                                       |
| :------------- | :---------------------------------------------------------------------------------------------------------------- | :--------- | :-------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------- |
| **D8**         | Aviso de limpeza do banco local e migrações editadas                                                              | `338bdba`  | `README.md`                                                                                               | Inspeção documental                                                   |
| **D7**         | Triggers físicos append-only no `audit_log` (bloqueio de UPDATE/DELETE)                                           | `e44e6ec`  | `crates/store-sqlite/migrations/0002_...`, `crates/store-sqlite/src/lib.rs`                               | `test_audit_log_triggers_abort_update_and_delete`                     |
| **D2**         | Validação de integridade e parâmetros do `DUMMY_ARGON2_HASH`                                                      | `934a4b4`  | `crates/core/src/services/mod.rs`, `crates/core/src/testing.rs`                                           | `test_dummy_argon2_hash_validity_and_params`                          |
| **D1**         | `LoginPolicy` desacoplada contra DoS e documentação de segurança                                                  | `2815cfc`  | `crates/core/src/domain/user.rs`, `crates/core/src/services/mod.rs`, `docs/seguranca.md`, `src-tauri/...` | `unit_tests::*`, testes de contrato                                   |
| **D3 & D4**    | Centralização da autorização no core (`:manage` implica `:read`) e teste estrutural da guarda                     | `926d723`  | `crates/core/src/domain/user.rs`, `src-tauri/src/guards.rs`, `src-tauri/src/commands/...`                 | `test_all_protected_commands_call_require_session_guard`              |
| **D5**         | `get_current_session` sem renovação de inatividade (relógio injetável)                                            | `7ae368e`  | `src-tauri/src/lib.rs`, `src-tauri/src/commands/auth.rs`                                                  | `test_get_current_session_does_not_renew_last_activity_at`            |
| **D6**         | Auditoria obrigatória em `ensure_system_roles` e propagação de erros                                              | `920d43c`  | `crates/core/src/services/mod.rs`, `crates/core/src/testing.rs`, `crates/store-sqlite/src/lib.rs`         | `contract_system_roles`, `contract_audit_log`                         |
| **A1**         | `TransactionManager`, `TxStoreBundle` e `run_in_tx` com retry exponencial                                         | `960ec93`  | `crates/core/src/store/mod.rs`, `crates/core/src/lib.rs`, `crates/core/src/error.rs`                      | `test_run_in_tx_retry_and_max_retries`                                |
| **A2**         | Abstração `SqliteExecutor` (Pool/Tx), `BEGIN IMMEDIATE` e mapeamento SQLx                                         | `a4ea0ad`  | `crates/store-sqlite/src/lib.rs`                                                                          | `test_c5_contract_suite_running_inside_transaction`                   |
| **A3**         | ADR-0008 atualizado para "Aceito" com detalhes de executor, SQLite e PostgreSQL                                   | `d563a9d`  | `docs/decisions/0008-transacoes-nas-stores.md`                                                            | Inspeção documental ADR                                               |
| **B**          | Serviços com transações e commits explícitos em falha de login/recuperação                                        | `3507021`  | `crates/core/src/services/mod.rs`                                                                         | `test_c1_transaction_rollback_guarantee`, `test_c2_login_failure_...` |
| **C**          | Testes de Rollback (C1), Falha Persistida (C2), Concorrência com Temp File (C3), Retry (C4) e Contrato em Tx (C5) | `c992dbc`  | `crates/core/src/testing.rs`, `crates/store-sqlite/src/lib.rs`                                            | `test_c1_*`, `test_c2_*`, `test_c3_*`, `test_c4_*`, `test_c5_*`       |
| **F**          | Congelamento no CI das migrações `0001` e `0002` e documentação em `AGENTS.md`                                    | `6c35c86`  | `.github/workflows/ci.yml`, `AGENTS.md`, `docs/02-modelo-dados.md`                                        | CI workflow step check                                                |
| **Clippy**     | Derivação de `Default` em `LoginPolicy` conforme recomendação de linter                                           | `726cebf`  | `crates/core/src/domain/user.rs`                                                                          | `cargo clippy --workspace -- -D warnings`                             |

---

## 3. Trechos de Código Requisitados pelo Diretor (Verbatim)

### B3 — Limites de Senha (8 a 128 Caracteres Unicode) e Validação de E-mail

Arquivo: `crates/core/src/services/mod.rs` (linhas 33-95)

```rust
/// Valida tamanho de senha em contagem de caracteres Unicode (RFC/OWASP: 8 a 128 caracteres).
pub fn validate_password(password: &str) -> Result<()> {
    let char_count = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&char_count) {
        return Err(CoreError::Validation(format!(
            "A senha deve ter entre {MIN_PASSWORD_LEN} e {MAX_PASSWORD_LEN} caracteres."
        )));
    }
    Ok(())
}

/// Valida formato de e-mail sem dependências externas.
///
/// Regras aplicadas:
/// 1. Comprimento total entre 5 e 254 caracteres (RFC 5321).
/// 2. Presença de exatamente um caractere '@', separando parte local e domínio.
/// 3. Parte local: não vazia, sem pontos no início/fim, sem pontos consecutivos.
/// 4. Domínio: não vazio, sem pontos ou hífens no início/fim, com pelo menos uma extensão (TLD).
/// 5. TLD (Top-Level Domain): no mínimo 2 caracteres exclusivamente alfabéticos.
pub fn validate_email(email: &str) -> Result<String> {
    let trimmed = email.trim().to_lowercase();
    if trimmed.len() < 5 || trimmed.len() > 254 {
        return Err(CoreError::Validation(
            "O e-mail deve ter entre 5 e 254 caracteres.".to_string(),
        ));
    }
    let parts: Vec<&str> = trimmed.split('@').collect();
    if parts.len() != 2 {
        return Err(CoreError::Validation(
            "Informe um e-mail válido contendo um único '@'.".to_string(),
        ));
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return Err(CoreError::Validation(
            "Parte local do e-mail inválida.".to_string(),
        ));
    }
    if domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.starts_with('-')
        || domain.ends_with('-')
        || domain.contains("..")
    {
        return Err(CoreError::Validation(
            "Domínio do e-mail inválido.".to_string(),
        ));
    }
    let domain_labels: Vec<&str> = domain.split('.').collect();
    if domain_labels.len() < 2 {
        return Err(CoreError::Validation(
            "Domínio do e-mail deve conter pelo menos uma extensão (ex: .com).".to_string(),
        ));
    }
    let tld = domain_labels.last().unwrap();
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(CoreError::Validation(
            "Extensão do e-mail inválida.".to_string(),
        ));
    }
    Ok(trimmed)
}
```

---

### B4 — Anti-Escalada de Privilégios em `create_user` e `update_user`

Arquivo: `crates/core/src/services/mod.rs`

Trecho em `create_user`:

```rust
                let role = stores
                    .roles()
                    .find_by_id(&session.tenant_id, &role_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Perfil de acesso não encontrado.".to_string())
                    })?;

                // Anti-escalada (B4): o perfil atribuído deve ter permissões contidas na sessão atual
                for perm in &role.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido atribuir um perfil com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }
```

Trecho em `update_user`:

```rust
                let new_role = stores
                    .roles()
                    .find_by_id(&session.tenant_id, &input_role_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Perfil de acesso não encontrado.".to_string())
                    })?;

                // Anti-escalada (B4):
                for perm in &new_role.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido atribuir um perfil com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }

                if target.role_slug == ROLE_SLUG_OWNER
                    && target.active
                    && new_role.slug != ROLE_SLUG_OWNER
                {
                    let active_owners = stores
                        .users()
                        .count_active_by_role_slug(&session.tenant_id, ROLE_SLUG_OWNER)
                        .await?;
                    if active_owners <= 1 {
                        return Err(CoreError::Validation(
                            "Não é possível alterar o perfil do último Proprietário ativo."
                                .to_string(),
                        ));
                    }
                }
```

---

### B6 — Implementações de `Debug` Ocultando Segredos (`[REDACTED]`)

Arquivo: `crates/core/src/domain/user.rs`

```rust
impl fmt::Debug for CreateUserInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateUserInput")
            .field("name", &self.name)
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .field("role_id", &self.role_id)
            .finish()
    }
}

impl fmt::Debug for ResetPasswordInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResetPasswordInput")
            .field("new_password", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Debug for RecoverPasswordInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoverPasswordInput")
            .field("email", &self.email)
            .field("recovery_key", &"[REDACTED]")
            .field("new_password", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Debug for RecoverPasswordResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoverPasswordResult")
            .field("new_recovery_key", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Debug for LoginInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginInput")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}
```

---

### B7 — `reset_user_password` Zerando Contadores e Bloqueio

Arquivo: `crates/core/src/services/mod.rs` (linhas 1236-1283) e query correspondente em `crates/store-sqlite/src/lib.rs` (linhas 1128-1151)

Em `CoreServices`:

```rust
    pub async fn reset_user_password(
        &self,
        session: &SessionInfo,
        user_id: &str,
        input: &ResetPasswordInput,
    ) -> Result<()> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;
        validate_password(&input.new_password)?;

        let password_hash = hash_secret(&input.new_password).await?;
        let session = session.clone();
        let user_id_str = user_id.to_string();

        self.run_transaction(|stores| {
            let session = session.clone();
            let user_id = user_id_str.clone();
            let password_hash = password_hash.clone();

            Box::pin(async move {
                let target = stores
                    .users()
                    .find_by_id(&session.tenant_id, &user_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

                let now = Utc::now().to_rfc3339();
                stores
                    .users()
                    .update_password(&session.tenant_id, &user_id, &password_hash, &now)
                    .await?;

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "user.password_reset",
                        "user",
                        &user_id,
                        &format!("Senha redefinida administrativamente para {}", target.email),
                    ))
                    .await?;

                Ok(())
            })
        })
        .await
    }
```

Na store SQLite (`crates/store-sqlite/src/lib.rs`):

```rust
    async fn update_password(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                r#"
                UPDATE app_user
                SET password_hash = ?, failed_attempts = 0, locked_until = NULL, updated_at = ?
                WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(password_hash)
            .bind(updated_at)
            .bind(tenant_id)
            .bind(user_id),
        )
        .await?;

        Ok(())
    }
```

---

### D1 — `get_public_branding` para Telas Públicas / Pré-Login

Arquivo: `src-tauri/src/commands/branding.rs` (linhas 17-31)

```rust
/// Comando público de identidade visual para telas pré-login (/login, /setup).
#[tauri::command]
pub async fn get_public_branding(
    state: State<'_, AppState>,
) -> Result<Option<PublicBranding>, String> {
    let branding = state
        .services
        .stores
        .branding
        .get_by_tenant_id(&state.default_tenant_id)
        .await
        .map_err(crate::map_core_err)?;

    Ok(branding.map(PublicBranding::from))
}
```

---

### D3 — Gravação Atômica do `config.json` (Arquivo Temporário + Rename Seguro)

Arquivo: `src-tauri/src/lib.rs` (linhas 116-151)

```rust
/// Grava o arquivo config.json de forma atômica (arquivo temporário + rename).
/// Previne corrupção de arquivo caso ocorra queda de energia ou terminação abrupta durante a escrita.
pub fn save_app_config_file(config_path: &Path, config: &AppConfig) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Falha ao serializar config.json: {e}"))?;

    let parent = config_path.parent().unwrap_or_else(|| Path::new("."));
    let temp_file = parent.join(format!(
        ".config.json.tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));

    std::fs::write(&temp_file, &serialized).map_err(|e| {
        format!(
            "Falha ao gravar arquivo temporário de configuração ({}): {e}",
            temp_file.display()
        )
    })?;

    #[cfg(windows)]
    {
        if config_path.exists() {
            let _ = std::fs::remove_file(config_path);
        }
    }

    std::fs::rename(&temp_file, config_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_file);
        format!(
            "Falha ao mover arquivo temporário para configuração final ({}): {e}",
            config_path.display()
        )
    })?;

    Ok(())
}
```

---

### A1 — `TransactionManager`, `TxStoreBundle` e `run_in_tx` com Retry por Backoff

Arquivo: `crates/core/src/store/mod.rs` (linhas 141-216)

```rust
#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>>;
}

#[async_trait]
pub trait TxStoreBundle: Send + Sync {
    fn tenants(&self) -> &dyn TenantStore;
    fn branding(&self) -> &dyn BrandingStore;
    fn roles(&self) -> &dyn RoleStore;
    fn users(&self) -> &dyn UserStore;
    fn audit(&self) -> &dyn AuditStore;

    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TransactionPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
}

impl Default for TransactionPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 25,
        }
    }
}

use std::future::Future;
use std::pin::Pin;

pub type TxBoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Executa um closure assíncrono dentro de uma transação atômica gerenciada por `TransactionManager`.
///
/// Garantias:
/// 1. Commit automático quando o closure retornar `Ok(T)`.
/// 2. Rollback automático quando o closure retornar `Err(E)` ou ocorrer pânico (RAII/Drop).
/// 3. Nova tentativa automática com backoff exponencial quando o erro for `CoreError::TransientConflict`
///    (ex.: SQLITE_BUSY / lock contention sob concorrência).
pub async fn run_in_tx<F, T>(
    manager: &dyn TransactionManager,
    policy: &TransactionPolicy,
    f: F,
) -> Result<T>
where
    F: for<'a> Fn(&'a dyn TxStoreBundle) -> TxBoxFuture<'a, T>,
{
    let mut attempt = 0;
    loop {
        let tx = manager.begin_transaction().await?;
        match f(tx.as_ref()).await {
            Ok(res) => {
                tx.commit().await?;
                return Ok(res);
            }
            Err(err) => {
                let is_transient = matches!(err, CoreError::TransientConflict(_));
                let _ = tx.rollback().await;

                if is_transient && attempt < policy.max_retries {
                    attempt += 1;
                    let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                    continue;
                }

                return Err(err);
            }
        }
    }
}
```

---

### A2 — Abstração de Executor Compartilhado no SQLite (`SqliteExecutor`)

Arquivo: `crates/store-sqlite/src/lib.rs` (linhas 90-189)

```rust
#[derive(Clone)]
pub enum SqliteExecutor {
    Pool(DbPool),
    Tx(Arc<tokio::sync::Mutex<Option<sqlx::Transaction<'static, Sqlite>>>>),
}

impl SqliteExecutor {
    pub async fn fetch_optional<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<Option<sqlx::sqlite::SqliteRow>> {
        match self {
            Self::Pool(pool) => query.fetch_optional(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.fetch_optional(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }

    pub async fn fetch_one<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::sqlite::SqliteRow> {
        match self {
            Self::Pool(pool) => query.fetch_one(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.fetch_one(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }

    pub async fn fetch_all<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        match self {
            Self::Pool(pool) => query.fetch_all(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.fetch_all(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }

    pub async fn execute<'a>(
        &self,
        query: sqlx::query::Query<'a, Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::sqlite::SqliteQueryResult> {
        match self {
            Self::Pool(pool) => query.execute(pool).await.map_err(map_sqlx_err),
            Self::Tx(tx) => {
                let mut guard = tx.lock().await;
                let tx = guard.as_mut().ok_or_else(|| {
                    CoreError::Database("Transação SQLite já foi finalizada ou abortada".into())
                })?;
                query.execute(&mut **tx).await.map_err(map_sqlx_err)
            }
        }
    }
}

#[derive(Clone)]
pub struct SqliteTxManager {
    pub pool: DbPool,
}

#[async_trait]
impl TransactionManager for SqliteTxManager {
    async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>> {
        let tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(map_sqlx_err)?;
        let tx_mutex = Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let store = SqliteStore {
            executor: SqliteExecutor::Tx(tx_mutex.clone()),
        };
        let arc = Arc::new(store);
        Ok(Box::new(SqliteTxStoreBundle {
            tx: tx_mutex,
            tenants: arc.clone(),
            branding: arc.clone(),
            roles: arc.clone(),
            users: arc.clone(),
            audit: arc,
        }))
    }
}
```

---

### B — Como Ficaram `complete_initial_setup` e `login` Após a Mudança Transacional

Arquivo: `crates/core/src/services/mod.rs`

#### `complete_initial_setup`:

```rust
    pub async fn complete_initial_setup(
        &self,
        tenant_id: &str,
        input: &InitialSetupInput,
        inactivity_timeout_minutes: u32,
    ) -> Result<InitialSetupResult> {
        if input.mode != OperationMode::Local {
            return Err(CoreError::Validation(
                "O modo Nuvem estará disponível em breve (Fase 2.5). Selecione o modo Local."
                    .to_string(),
            ));
        }

        let company_name = input.company_name.trim();
        let owner_name = input.owner_name.trim();
        let owner_email = validate_email(&input.owner_email)?;
        validate_password(&input.owner_password)?;

        if company_name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome da empresa.".to_string(),
            ));
        }
        if owner_name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do proprietário.".to_string(),
            ));
        }

        let recovery_key = generate_recovery_key();
        let password_hash = hash_secret(&input.owner_password).await?;
        let recovery_key_hash = hash_secret(&normalize_recovery_key(&recovery_key)).await?;

        let tenant_id_str = tenant_id.to_string();
        let company_name_str = company_name.to_string();
        let owner_name_str = owner_name.to_string();
        let recovery_key_clone = recovery_key.clone();

        self.run_transaction(|stores| {
            let tenant_id = tenant_id_str.clone();
            let company_name = company_name_str.clone();
            let owner_name = owner_name_str.clone();
            let owner_email = owner_email.clone();
            let password_hash = password_hash.clone();
            let recovery_key_hash = recovery_key_hash.clone();
            let recovery_key = recovery_key_clone.clone();

            Box::pin(async move {
                let existing_users = stores.users().count_by_tenant(&tenant_id).await?;
                if existing_users > 0 {
                    return Err(CoreError::Conflict(
                        "A configuração inicial já foi realizada para este ambiente.".to_string(),
                    ));
                }

                let now = Utc::now().to_rfc3339();
                stores
                    .tenants()
                    .update_name(&tenant_id, &company_name, &now)
                    .await?;

                let current_branding = stores.branding().get_by_tenant_id(&tenant_id).await?;
                let branding_input = UpdateBrandingInput {
                    company_name: company_name.clone(),
                    primary_color: current_branding
                        .as_ref()
                        .map(|b| b.primary_color.clone())
                        .unwrap_or_else(|| "#2563eb".to_string()),
                    secondary_color: current_branding
                        .as_ref()
                        .map(|b| b.secondary_color.clone())
                        .unwrap_or_else(|| "#475569".to_string()),
                    accent_color: current_branding
                        .as_ref()
                        .map(|b| b.accent_color.clone())
                        .unwrap_or_else(|| "#7c3aed".to_string()),
                    logo_url: current_branding.and_then(|b| b.logo_url),
                };
                let branding = stores.branding().save(&tenant_id, &branding_input).await?;

                ensure_system_roles(&tenant_id, stores.roles(), stores.audit()).await?;

                let owner_role = stores
                    .roles()
                    .find_by_slug(&tenant_id, ROLE_SLUG_OWNER)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Internal("Perfil Proprietário não encontrado.".to_string())
                    })?;

                let owner_id = Uuid::now_v7().to_string();
                let owner_record = UserRecord {
                    id: owner_id.clone(),
                    tenant_id: tenant_id.clone(),
                    name: owner_name,
                    email: owner_email.clone(),
                    password_hash,
                    recovery_key_hash: Some(recovery_key_hash),
                    role_id: owner_role.id.clone(),
                    active: true,
                    last_login_at: Some(now.clone()),
                    failed_attempts: 0,
                    locked_until: None,
                    recovery_failed_attempts: 0,
                    recovery_locked_until: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    deleted_at: None,
                    created_by: Some(owner_id.clone()),
                };

                let created_owner = stores.users().create(&owner_record).await?;

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&created_owner.id),
                        "tenant.setup_completed",
                        "tenant",
                        &tenant_id,
                        &format!(
                            "Configuração inicial concluída para '{company_name}' em modo local"
                        ),
                    ))
                    .await?;

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&created_owner.id),
                        "user.owner_created",
                        "user",
                        &created_owner.id,
                        &format!("Conta de proprietário criada para {owner_email}"),
                    ))
                    .await?;

                let session = SessionInfo {
                    user_id: created_owner.id,
                    tenant_id: tenant_id.clone(),
                    name: created_owner.name,
                    email: created_owner.email,
                    role_id: created_owner.role_id,
                    role_name: created_owner.role_name,
                    role_slug: created_owner.role_slug,
                    permissions: created_owner.permissions,
                    authenticated_at: now.clone(),
                    last_activity_at: now,
                    inactivity_timeout_minutes,
                };

                Ok(InitialSetupResult {
                    tenant_id,
                    recovery_key,
                    session,
                    branding,
                })
            })
        })
        .await
    }
```

#### `login`:

```rust
    pub async fn login(
        &self,
        tenant_id: &str,
        input: &LoginInput,
        policy: &LoginPolicy,
        inactivity_timeout_minutes: u32,
    ) -> Result<SessionInfo> {
        let email = input.email.trim().to_lowercase();
        if email.is_empty() || input.password.is_empty() {
            let _ = verify_secret(&input.password, DUMMY_ARGON2_HASH).await;
            return Err(CoreError::InvalidCredentials);
        }

        let maybe_user = self.stores.users.find_by_email(tenant_id, &email).await?;
        let user = match maybe_user {
            Some(u) if u.active => u,
            _ => {
                let _ = verify_secret(&input.password, DUMMY_ARGON2_HASH).await;
                return Err(CoreError::InvalidCredentials);
            }
        };

        let now = Utc::now();
        if let Some(ref locked_str) = user.locked_until {
            if let Ok(locked_dt) = DateTime::parse_from_rfc3339(locked_str) {
                if locked_dt.with_timezone(&Utc) > now {
                    if policy.reveal_account_locked {
                        return Err(CoreError::AccountLocked {
                            locked_until: locked_str.clone(),
                        });
                    } else {
                        let _ = verify_secret(&input.password, DUMMY_ARGON2_HASH).await;
                        return Err(CoreError::InvalidCredentials);
                    }
                }
            }
        }

        if !verify_secret(&input.password, &user.password_hash).await? {
            enum LoginFailureOutcome {
                Locked { locked_until: String },
                Failed,
            }

            let tenant_id_str = tenant_id.to_string();
            let user_id_str = user.id.clone();
            let email_str = email.clone();

            let outcome = self
                .run_transaction(|stores| {
                    let tenant_id = tenant_id_str.clone();
                    let user_id = user_id_str.clone();
                    let email = email_str.clone();

                    Box::pin(async move {
                        let now = Utc::now();
                        let now_str = now.to_rfc3339();
                        let new_failures = stores
                            .users()
                            .increment_login_failure(&tenant_id, &user_id, &now_str)
                            .await?;
                        let lockout_secs = calculate_lockout_seconds(new_failures);
                        let locked_until = if lockout_secs > 0 {
                            Some((now + Duration::seconds(lockout_secs)).to_rfc3339())
                        } else {
                            None
                        };

                        if let Some(ref until) = locked_until {
                            stores
                                .users()
                                .set_login_lockout(
                                    &tenant_id,
                                    &user_id,
                                    Some(until.as_str()),
                                    &now_str,
                                )
                                .await?;

                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.account_locked",
                                    "user",
                                    &user_id,
                                    &format!(
                                        "Conta bloqueada temporariamente após {new_failures} tentativas falhas"
                                    ),
                                ))
                                .await?;

                            Ok(LoginFailureOutcome::Locked {
                                locked_until: until.clone(),
                            })
                        } else {
                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.login_failed",
                                    "user",
                                    &user_id,
                                    &format!("Tentativa de login falha ({new_failures}) para {email}"),
                                ))
                                .await?;

                            Ok(LoginFailureOutcome::Failed)
                        }
                    })
                })
                .await?;

            match outcome {
                LoginFailureOutcome::Locked { locked_until } => {
                    if policy.reveal_account_locked {
                        return Err(CoreError::AccountLocked { locked_until });
                    } else {
                        return Err(CoreError::InvalidCredentials);
                    }
                }
                LoginFailureOutcome::Failed => {
                    return Err(CoreError::InvalidCredentials);
                }
            }
        }

        // Sucesso de autenticação: persiste registro de sucesso e auditoria em transação atômica
        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user.id.clone();
        let user_email_str = user.email.clone();

        let session = self
            .run_transaction(|stores| {
                let tenant_id = tenant_id_str.clone();
                let user_id = user_id_str.clone();
                let user_email = user_email_str.clone();

                Box::pin(async move {
                    let now_str = Utc::now().to_rfc3339();
                    stores
                        .users()
                        .record_login_success(&tenant_id, &user_id, &now_str)
                        .await?;

                    let user_with_role = stores
                        .users()
                        .find_with_role(&tenant_id, &user_id)
                        .await?
                        .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &tenant_id,
                            Some(&user_id),
                            "auth.login_succeeded",
                            "user",
                            &user_id,
                            &format!("Login bem-sucedido para {user_email}"),
                        ))
                        .await?;

                    let session = SessionInfo {
                        user_id: user_with_role.id,
                        tenant_id,
                        name: user_with_role.name,
                        email: user_with_role.email,
                        role_id: user_with_role.role_id,
                        role_name: user_with_role.role_name,
                        role_slug: user_with_role.role_slug,
                        permissions: user_with_role.permissions,
                        authenticated_at: now_str.clone(),
                        last_activity_at: now_str,
                        inactivity_timeout_minutes,
                    };

                    Ok(session)
                })
            })
            .await?;

        Ok(session)
    }
```

---

## 4. DUMMY_ARGON2_HASH e Parâmetros de Produção

Constante definida em `crates/core/src/services/mod.rs` (linhas 30-31):

```rust
pub const DUMMY_ARGON2_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$ZXZlbnRyZW50c2FsdDEyMw$U29tZVVua25vd25QYXNzd29yZEhhc2hEYXRhMTIzNDU";
```

### Prova Automatizada de Validade e Parâmetros OWASP:

Arquivo `crates/core/src/testing.rs` (linhas 810-834):

```rust
    #[tokio::test]
    async fn test_dummy_argon2_hash_validity_and_params() {
        use argon2::password_hash::PasswordHash;
        use crate::services::{create_owasp_argon2, verify_secret, DUMMY_ARGON2_HASH};

        // 1. O hash deve ser sintaticamente e estruturalmente válido para o parser Argon2
        let parsed = PasswordHash::new(DUMMY_ARGON2_HASH)
            .expect("DUMMY_ARGON2_HASH deve ser um hash Argon2 válido");

        // 2. O algoritmo deve ser Argon2id versão 0x13
        assert_eq!(parsed.algorithm.as_str(), "argon2id");
        assert_eq!(parsed.version, Some(argon2::Version::V0x13));

        // 3. Os parâmetros devem ser estritamente os mesmos da produção (OWASP): m=19456 KiB, t=2, p=1
        let owasp = create_owasp_argon2().expect("Falha ao criar instância OWASP Argon2");
        let expected_params = owasp.params();

        assert_eq!(parsed.params.m_cost(), expected_params.m_cost());
        assert_eq!(parsed.params.t_cost(), expected_params.t_cost());
        assert_eq!(parsed.params.p_cost(), expected_params.p_cost());

        // 4. Execução de verify_secret contra o dummy hash deve rodar com custo real completo e retornar false
        let verification = verify_secret("TentativaAleatoria#2026", DUMMY_ARGON2_HASH).await;
        assert!(verification.is_ok());
        assert!(!verification.unwrap());
    }
```

---

## 5. Prova de Execução dos Testes (`cargo test --workspace`)

Saída obtida na execução local completa:

```text
running 6 tests
test guards::tests::test_public_commands_are_strictly_whitelisted ... ok
test guards::tests::test_session_inactivity_expiration_uses_core_production_path ... ok
test guards::tests::test_command_registry_matches_generate_handler_in_lib_rs ... ok
test guards::tests::test_all_protected_commands_call_require_session_guard ... ok
test tests::test_save_app_config_file_atomic ... ok
test tests::test_get_current_session_does_not_renew_last_activity_at ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; finished in 0.01s

running 8 tests
test testing::unit_tests::test_lockout_backoff_progression ... ok
test testing::unit_tests::test_recovery_key_format_and_uniqueness ... ok
test testing::unit_tests::test_password_and_email_validation ... ok
test testing::unit_tests::test_debug_redacts_sensitive_fields ... ok
test testing::unit_tests::test_run_in_tx_retry_and_max_retries ... ok
test testing::unit_tests::test_argon2id_spawn_blocking_does_not_block_async_runtime ... ok
test testing::unit_tests::test_dummy_argon2_hash_validity_and_params ... ok
test testing::unit_tests::test_argon2id_hash_and_verify ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; finished in 1.08s

running 17 tests
test tests::test_contract_default_seeds ... ok
test tests::test_audit_log_triggers_abort_update_and_delete ... ok
test tests::test_contract_system_roles ... ok
test tests::test_contract_unique_partial_indexes ... ok
test tests::test_init_db_with_spaces_and_accents_in_path ... ok
test tests::test_c3_concurrency_with_multiple_connections_on_temp_file ... ok
test tests::test_contract_last_owner_protection ... ok
test tests::test_contract_initial_setup_and_recovery_key ... ok
test tests::test_c2_login_failure_persists_counter_and_lockout_despite_returning_err ... ok
test tests::test_contract_user_creation_and_rbac ... ok
test tests::test_contract_recovery_key_rotation_and_lockout ... ok
test tests::test_c1_transaction_rollback_guarantee ... ok
test tests::test_contract_user_lockout ... ok
test tests::test_contract_password_reset_clears_lockout ... ok
test tests::test_contract_audit_log ... ok
test tests::test_sqlite_store_contract_suite ... ok
test tests::test_c5_contract_suite_running_inside_transaction ... ok
test result: ok. 17 passed; 0 failed; 0 ignored; finished in 9.60s
```

---

## 6. Status do CI no GitHub Actions

- **Pull Request**: [#4](https://github.com/FarrofaDeBacon/eventrent/pull/4)
- **Workflows**: `quality (ubuntu-latest)` e `quality (windows-latest)` verdes e validados em menos de 4 minutos cada.
