# Relatório de Conclusão — Fase 2N: Backup e Restauração Transacional

**Branch**: `fase-2n-backup`  
**Pull Request**: [#13](https://github.com/FarrofaDeBacon/eventrent/pull/13)  
**Status**: Concluído — 100% dos testes e verificações aprovados  

---

## 1. Resumo Executivo

A **Fase 2N** implementou a arquitetura completa de cópias de segurança (backup) transacionais e restauração segura a frio para o modo local do **EventRent**, cumprindo integralmente as diretrizes do **ADR-0017**, respeitando o congelamento das migrações SQLite existentes (`0001` a `0010`) e mantendo a política rigorosa de integridade estrutural por marcadores verificáveis via scripts automáticos.

### Principais Entregas:
1. **Tarefa 0 (Pendências da Fase 2S)**:
   - Remoção de código morto (`delete_user` e testes associados) em `crates/core/src/services/mod.rs`;
   - Teste de não-revogação da sessão em execução ao rotacionar a chave de recuperação (`sync_current_session_epoch`), garantindo que apenas sessões concorrentes sejam deslogadas;
   - Cobertura de 100% dos comandos IPC em testes de contrato de repositório (expandida de 62 para 70 comandos, com 80 testes no Vitest).
2. **Tarefa 1 (ADR-0017 & RBAC)**:
   - Registro formal da arquitetura em `docs/decisions/0017-backup-e-restauracao.md`;
   - Introdução da permissão canônica `backup:manage` atribuída por padrão ao papel do Proprietário.
3. **Tarefas 2.1 e 2.2 (Core Domain & `crates/backup-fs`)**:
   - Persistência das configurações em `tenant_setting` (chave `backup_settings`) sem alteração da base de migrações congelada;
   - Criação da crate `crates/backup-fs` responsável pela geração de snapshots SQLite atômicos via `VACUUM INTO`, validação estrutural física via `PRAGMA integrity_check`, empacotamento híbrido `.zip` com manifesto e hashes SHA-256 individuais de dados e mídias;
   - Trava de concorrência RAII (`BackupOperationGuard`) que bloqueia operações concorrentes e suspende a coleta de lixo de mídia (`run_media_gc`) durante o backup;
   - Limpeza automatizada de retenção temporal (*grandfather-father-son*: diário, semanal, mensal);
   - Algoritmo de restauração a frio executado na inicialização do aplicativo (`execute_pending_restore_at_startup`) antes de abrir o banco SQLite.
4. **Tarefa 2.3 (Tauri IPC, Schemas Zod & Repositórios)**:
   - 8 novos comandos protegidos em `src-tauri/src/commands/backup.rs` registrados em `COMMAND_REGISTRY` e `guards.rs`;
   - Schemas Zod e tipagens TypeScript estritas;
   - Port & Adapter: `tauri-backup-repository.ts` e `mock-backup-repository.ts` com guarda de instanciação em produção;
   - 70 comandos com fixtures e testes de contrato bidirecionais.
5. **Tarefa 2.4 (Interface Svelte 5 em `/settings/backup`)**:
   - Tela com runes Svelte 5, conformidade total com tokens semânticos (0 cores literais);
   - Aviso proeminente de privacidade e conformidade LGPD;
   - Banner de alerta de backup desatualizado (`is_stale`);
   - Seletor de pasta via diálogo nativo e controles de retenção;
   - Tabela de histórico de backups com badge de compatibilidade de schema;
   - Modal bloqueante de confirmação com exigência de digitação manual da palavra `RESTAURAR`.
6. **Tarefa 2.5 (Testes E2E no Playwright)**:
   - Suíte de ponta a ponta em `e2e/backup.spec.ts` cobrindo navegação, salvamento de configurações, execução de backup e acionamento do modal de restauração.
7. **Tarefa 2.6 (Documentação Atualizada)**:
   - Atualização de `docs/seguranca.md`, `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md` e `docs/05-roadmap.md`.

---

## 2. Rastreabilidade Estrutural de Código (Snippets Verificados)

Todos os trechos de código abaixo foram extraídos dinamicamente pelo script `scripts/report-snippet.js` e verificados pelo script `scripts/check-report-snippets.js`.

### 2.1. Trava RAII Anti-Concorrência (`BackupOperationGuard`)

Em `crates/core/src/domain/backup.rs`, a trava de processo impede backups simultâneos ou concorrência com o coletor de lixo:

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:26-53 -->
```rust
pub struct BackupOperationGuard;

impl BackupOperationGuard {
    pub fn try_acquire() -> Result<Self> {
        if BACKUP_OR_RESTORE_ACTIVE
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            Err(CoreError::TransientConflict(
                "Operação de backup ou restauração já está em andamento.".to_string(),
            ))
        } else {
            Ok(Self)
        }
    }
}

impl Drop for BackupOperationGuard {
    fn drop(&mut self) {
        BACKUP_OR_RESTORE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

```

### 2.2. Geração Atômica do Snapshot SQLite via `VACUUM INTO`

Em `crates/backup-fs/src/lib.rs`, o snapshot consistente do banco SQLite é gerado sem leituras sujas de WAL:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:52-85 -->
```rust
    pub async fn run_backup(
        &self,
        destination_folder: &Path,
        db_pool: &SqlitePool,
        app_version: &str,
    ) -> Result<(BackupManifest, PathBuf)> {
        let _guard = BackupOperationGuard::try_acquire()?;

        validate_destination_folder(destination_folder, &self.app_data_dir)?;

        fs::create_dir_all(destination_folder).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao acessar pasta de destino de backup ({}): {e}",
                destination_folder.display()
            ))
        })?;

        let now = Utc::now();
        let ts = now.format("%Y-%m-%d-%H%M%S").to_string();
        let vacuum_unique = uuid::Uuid::now_v7();
        let temp_vacuum_db = self
            .app_data_dir
            .join(format!("vacuum_backup_{vacuum_unique}.tmp.db"));

        // 1. Cópia consistente do banco SQLite via VACUUM INTO
        let vacuum_sql = format!(
            "VACUUM INTO '{}'",
            temp_vacuum_db.to_string_lossy().replace('\'', "''")
        );

        sqlx::query(&vacuum_sql)
            .execute(db_pool)
            .await
            .map_err(|e| {
```

### 2.3. Execução da Restauração a Frio na Inicialização do App

Em `crates/backup-fs/src/lib.rs`, a detecção de marcador e a substituição física do banco ocorrem antes da abertura do pool:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:667-705 -->
```rust
    pub fn execute_pending_restore_at_startup(
        &self,
        current_max_migration: i64,
    ) -> Result<Option<RestoreReport>> {
        let marker_path = self.app_data_dir.join(RESTORE_PENDING_FILE);
        if !marker_path.exists() {
            return Ok(None);
        }

        tracing::info!(
            marker_path = %marker_path.display(),
            "Marcador de restauração pendente detectado no boot do sistema. Iniciando restauração a frio."
        );

        let marker_raw = fs::read_to_string(&marker_path).map_err(|e| {
            CoreError::Internal(format!("Falha ao ler marcador de restauração: {e}"))
        })?;

        let marker: RestorePendingMarker = serde_json::from_str(&marker_raw).map_err(|e| {
            CoreError::Internal(format!("Formato inválido no marcador de restauração: {e}"))
        })?;

        let backup_file_path = PathBuf::from(&marker.backup_file_path);
        if !backup_file_path.exists() {
            let _ = fs::remove_file(&marker_path);
            return Err(CoreError::Validation(format!(
                "O arquivo de backup agendado não foi encontrado: {}",
                backup_file_path.display()
            )));
        }

        // Validação profunda
        let verify = self.verify_backup_file(&backup_file_path, current_max_migration)?;
        if !verify.valid {
            let _ = fs::rename(
                &marker_path,
                self.app_data_dir.join("restore_corrupted.json"),
            );
            return Err(CoreError::Validation(format!(
```

### 2.4. Comando Tauri IPC Protegido (`run_backup_now`)

Em `src-tauri/src/commands/backup.rs`, o comando valida a sessão e grava o resultado na auditoria operacional:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/commands/backup.rs:75-110 -->
```rust
pub async fn run_backup_now(state: State<'_, AppState>) -> Result<BackupMetadataView, String> {
    let session = require_session(&state).await?;

    let status = state
        .services
        .get_backup_status(&session)
        .await
        .map_err(crate::map_core_err)?;

    let dest = status.settings.destination_folder.trim().to_string();
    if dest.is_empty() {
        return Err("Nenhuma pasta de destino de backup configurada. Selecione uma pasta antes de prosseguir.".to_string());
    }

    let dest_path = PathBuf::from(&dest);

    let (manifest, backup_file) = match state
        .backup_service
        .run_backup(&dest_path, &state.db_pool, &state.app_version)
        .await
    {
        Ok(res) => res,
        Err(e) => {
            let err_str = e.to_string();
            let _ = state
                .services
                .record_backup_outcome(
                    &session.tenant_id,
                    Some(&session.user_id),
                    "failed",
                    Some(&err_str),
                    None,
                )
                .await;
            return Err(crate::map_core_err(e));
        }
```

### 2.5. Modal Bloqueante de Confirmação de Restauração no Frontend

Em `src/routes/settings/backup/+page.svelte`, o botão de confirmação permanece desabilitado até a digitação literal de `RESTAURAR`:

<!-- prettier-ignore -->
<!-- snippet: src/routes/settings/backup/+page.svelte:728-763 -->
```svelte
          class="text-xs font-medium text-foreground block"
        >
          Para prosseguir, digite exatamente <span
            class="font-bold text-destructive">RESTAURAR</span
          > no campo abaixo:
        </label>
        <Input
          id="restore-confirm-input"
          type="text"
          placeholder="RESTAURAR"
          bind:value={restoreConfirmationInput}
          class="font-mono text-center tracking-widest uppercase font-bold"
        />
      </div>

      <div class="flex justify-end gap-3 pt-2">
        <Button
          variant="outline"
          onclick={closeRestoreModal}
          disabled={isSchedulingRestore}
        >
          Cancelar
        </Button>
        <Button
          variant="destructive"
          onclick={handleConfirmRestore}
          disabled={!isRestoreConfirmed || isSchedulingRestore}
          class="flex items-center gap-2"
        >
          {#if isSchedulingRestore}
            Agendando...
          {:else}
            Confirmar e Agendar Restauração
          {/if}
        </Button>
      </div>
```

### 2.6. Teste E2E Playwright de Backup e Confirmação de Restauração

Em `e2e/backup.spec.ts`, o fluxo de ponta a ponta exercita a interface e a trava de confirmação:

<!-- prettier-ignore -->
<!-- snippet: e2e/backup.spec.ts:65-95 -->
```typescript

    // Feedback de sucesso
    await expect(page.getByText(/Backup concluído:/)).toBeVisible();

    // 6. Tabela de backups encontrados na pasta
    const restoreButtons = page.getByRole("button", { name: "Restaurar..." });
    await expect(restoreButtons.first()).toBeVisible();
    await restoreButtons.first().click();

    // 7. Modal bloqueante de confirmação com palavra-chave RESTAURAR
    await expect(
      page.getByRole("heading", { name: "Confirmar Restauração a Frio" }),
    ).toBeVisible();
    await expect(
      page.getByText(
        "A restauração substituirá integralmente todos os dados cadastrados no aplicativo",
      ),
    ).toBeVisible();

    const confirmButton = page.getByRole("button", {
      name: "Confirmar e Agendar Restauração",
    });
    await expect(confirmButton).toBeDisabled();

    const confirmInput = page.locator("#restore-confirm-input");
    await confirmInput.fill("restaurar");
    await expect(confirmButton).toBeDisabled();

    await confirmInput.fill("RESTAURAR");
    await expect(confirmButton).toBeEnabled();

```

---

## 3. Matriz de Comandos IPC e Cobertura de Contratos

Com a incorporação dos 8 comandos da Fase 2N, o catálogo do **EventRent** conta agora com **70 comandos Tauri IPC**, todos validados por fixtures JSON e cobertos por testes unitários e de contrato no Vitest:

| # | Comando Tauri | Permissão Exigida | Fixture JSON | Cobertura Vitest |
|---|---|---|---|---|
| 63 | `get_backup_status` | `backup:manage` | `get_backup_status.json` | 100% |
| 64 | `set_backup_settings` | `backup:manage` | `set_backup_settings.json` | 100% |
| 65 | `run_backup_now` | `backup:manage` | `run_backup_now.json` | 100% |
| 66 | `list_backups` | `backup:manage` | `list_backups.json` | 100% |
| 67 | `verify_backup` | `backup:manage` | `verify_backup.json` | 100% |
| 68 | `schedule_restore` | `backup:manage` | `schedule_restore.json` | 100% |
| 69 | `choose_backup_folder` | `backup:manage` | `choose_backup_folder.json` | 100% |
| 70 | `choose_backup_file` | `backup:manage` | `choose_backup_file.json` | 100% |

---

## 4. Portão de Qualidade

A suíte completa de validação (`pnpm verify`) foi executada com 100% de sucesso nos 14 passos:
- **Passo 1**: SvelteKit Check & Typecheck (0 erros, 0 warnings);
- **Passo 2**: Lint de Código & Cores (0 cores literais em 48 componentes .svelte, Prettier e ESLint 100%);
- **Passo 3**: Testes Unitários Frontend (132 testes em 11 arquivos no Vitest);
- **Passo 4**: Build Frontend de Produção (sem mocks);
- **Passo 5**: Verificação Estática de Produção (zero menções a mock em bundles de produção);
- **Passo 6**: Integridade de Migrações (todos os 6 cenários de congelamento aprovados);
- **Passo 7**: Prova Estrutural de Trechos de Código em Relatórios (4 cenários aprovados);
- **Passo 8**: Verificação Estrutural de Snippets em Relatórios (todos os marcadores aprovados);
- **Passo 9**: Testes Rust (`cargo test --workspace`) (58 testes unitários e de integração aprovados em todas as crates);
- **Passo 10**: Linter Rust (`cargo clippy --workspace --all-targets -- -D warnings`) (zero advertências);
- **Passo 11**: Auditoria de Dependências Rust (`cargo audit`);
- **Passo 12**: Contratos IPC Tauri (`cargo test -p eventrent-app --test contracts_spec`);
- **Passo 13**: Ciclo de Vida de Sessão (`cargo test -p eventrent-app --test session_lifecycle_tests`);
- **Passo 14**: Testes E2E com Playwright (todas as suítes aprovadas).
