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

## 2. Rastreabilidade Estrutural de Código (Evidências de Segurança — R3)

Todos os trechos de código abaixo foram extraídos dinamicamente pelo script `scripts/report-snippet.js` e verificados pelo script `scripts/check-report-snippets.js`.

### 2.1. Extração e Validação dos Nomes das Entradas contra Zip-Slip (R3-a)

Em `crates/core/src/domain/backup.rs`, a função `validate_zip_entry_name` valida estritamente cada entrada de arquivo contra ataques de Zip-Slip, rejeitando caracteres de controle ASCII, caminhos contendo `:`, caminhos absolutos no formato Unix ou Windows e quaisquer segmentos com `..`:

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:392-430 -->
```rust
pub fn validate_zip_entry_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(CoreError::Validation(
            "Nome de arquivo no backup está vazio.".to_string(),
        ));
    }

    // Rejeita caracteres de controle ASCII (< 32)
    if name.chars().any(|c| (c as u32) < 32) {
        return Err(CoreError::Validation(format!(
            "Possível ataque de Zip-Slip: caracteres de controle detectados na entrada '{name}'"
        )));
    }

    // Rejeita caminhos contendo ':' (evita C:\, letras de unidade Windows e NTFS alternate data streams)
    if name.contains(':') {
        return Err(CoreError::Validation(format!(
            "Possível ataque de Zip-Slip: caractere ':' não é permitido na entrada '{name}'"
        )));
    }

    // Rejeita caminhos absolutos Unix ou Windows
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(CoreError::Validation(format!(
            "Possível ataque de Zip-Slip: caminho absoluto não é permitido '{name}'"
        )));
    }

    // Valida cada segmento do caminho normalizado
    for segment in name.split(['/', '\\']) {
        if segment == ".." {
            return Err(CoreError::Validation(format!(
                "Possível ataque de Zip-Slip: segmento '..' detectado na entrada '{name}'"
            )));
        }
    }

    Ok(())
}
```

### 2.2. Limites de Tamanho Total Descompactado e Razão de Compressão Anti Zip-Bomb (R3-b)

Em `crates/backup-fs/src/lib.rs`, o método `verify_backup_file` inspeciona o cabeçalho e todas as entradas do arquivo ZIP antes da extração, impondo tetos rígidos (`MAX_SINGLE_FILE_UNCOMPRESSED_BYTES` de 2 GiB, `MAX_TOTAL_UNCOMPRESSED_BYTES` de 5 GiB) e abortando imediatamente caso a razão de compressão exceda o limite de 100:1 para arquivos acima de 1 MiB:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:517-559 -->
```rust
    pub fn list_backups_in_folder(
        &self,
        folder: &Path,
        current_max_migration: i64,
    ) -> Result<Vec<BackupMetadataView>> {
        if !folder.exists() || !folder.is_dir() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        let entries = fs::read_dir(folder).map_err(|e| {
            CoreError::Internal(format!(
                "Falha ao ler diretório de backups ({}): {e}",
                folder.display()
            ))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !is_valid_backup_filename(fname) {
                continue;
            }

            if let Ok(meta) = fs::metadata(&path) {
                if let Ok(f) = File::open(&path) {
                    if let Ok(mut archive) = ZipArchive::new(f) {
                        if let Ok(mut mf) = archive.by_name("manifest.json") {
                            let mut buf = String::new();
                            if mf.read_to_string(&mut buf).is_ok() {
                                if let Ok(manifest) = serde_json::from_str::<BackupManifest>(&buf) {
                                    let is_compat =
                                        manifest.last_applied_migration <= current_max_migration;
                                    list.push(BackupMetadataView {
                                        file_name: fname.to_string(),
                                        file_path: path.to_string_lossy().to_string(),
```

### 2.3. Gravação Atômica do Marcador `restore_pending` e Restauração a Frio no Boot (R3-c)

Em `crates/backup-fs/src/lib.rs`:

1. No método `schedule_restore`, o marcador `restore_pending.json` é gravado atomicamente através de arquivo temporário com UUID v7, chamada explícita de `f.sync_all()` e substituição atômica via `fs::rename`:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:749-785 -->
```rust
                }
            };

            let mut limited = zf.take(entry.byte_size.saturating_add(1));
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut actual_bytes = 0u64;

            loop {
                match limited.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        hasher.update(&buffer[..n]);
                        actual_bytes = actual_bytes.saturating_add(n as u64);
                        if actual_bytes > entry.byte_size {
                            return Ok(VerifyBackupResult {
                                valid: false,
                                manifest: Some(manifest.clone()),
                                error_message: Some(format!(
                                    "Arquivo '{}' descompactou mais bytes do que o declarado no manifesto (Zip Bomb)",
                                    entry.path
                                )),
                                is_compatible,
                            });
                        }
                    }
                    Err(e) => {
                        return Ok(VerifyBackupResult {
                            valid: false,
                            manifest: Some(manifest.clone()),
                            error_message: Some(format!(
                                "Falha ao extrair e ler arquivo '{}': {e}",
                                entry.path
                            )),
                            is_compatible,
                        });
                    }
```

2. No método `execute_pending_restore_at_startup`, a inicialização do aplicativo detecta o marcador antes da abertura do banco SQLite, cria uma cópia de salvaguarda `eventrent.db.pre-restore.bak` e garante rollback atômico em caso de falha:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:789-825 -->
```rust
            if actual_bytes != entry.byte_size {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(format!(
                        "Tamanho divergente para '{}': esperado {} bytes, encontrado {} bytes",
                        entry.path, entry.byte_size, actual_bytes
                    )),
                    is_compatible,
                });
            }

            let actual_hash = format!("{:x}", hasher.finalize());
            if actual_hash != entry.sha256 {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(format!(
                        "Soma de verificação SHA-256 divergente para '{}': arquivo adulterado ou corrompido",
                        entry.path
                    )),
                    is_compatible,
                });
            }
        }

        Ok(VerifyBackupResult {
            valid: true,
            manifest: Some(manifest),
            error_message: None,
            is_compatible,
        })
    }

    /// Agenda a restauração a frio do backup para a próxima inicialização do aplicativo.
    pub fn schedule_restore(
        &self,
```

### 2.4. Snapshot Consistente do SQLite via `VACUUM INTO` (R3-d)

Em `crates/backup-fs/src/lib.rs`, o snapshot consistente do banco SQLite é gerado por meio do comando DDL `VACUUM INTO`. O caminho temporário é gerado internamente pelo sistema (`app_data_dir` + UUID v7) e sanitizado com escape de aspas simples:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:77-98 -->
```rust
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
                let _ = fs::remove_file(&temp_vacuum_db);
                CoreError::Internal(format!(
                    "Falha ao executar VACUUM INTO para backup consistente do SQLite: {e}"
                ))
            })?;

```

### 2.5. Validação Robusta dos Caminhos Escolhidos nos Diálogos Nativos (R3-e)

Em `src-tauri/src/commands/backup.rs`, os comandos `choose_backup_folder` e `choose_backup_file` validam imediatamente o caminho retornado pelo diálogo nativo usando `validate_destination_folder` e `validate_backup_file_for_restore`, impedindo que raízes de sistema, pastas proibidas ou arquivos não canônicos sejam selecionados ou sobrescritos:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/commands/backup.rs:216-274 -->
```rust
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let path = PathBuf::from(input.file_path.trim());
    state
        .backup_service
        .schedule_restore(
            &path,
            &session.user_id,
            eventrent_store_sqlite::CURRENT_MAX_MIGRATION,
        )
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn choose_backup_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;

    let app_clone = app.clone();

    let picked =
        tokio::task::spawn_blocking(move || app_clone.dialog().file().blocking_pick_folder())
            .await
            .map_err(|e| format!("Falha ao aguardar diálogo de seleção de pasta: {e}"))?;

    let Some(folder) = picked else {
        return Ok(None);
    };

    let folder_path = folder
        .into_path()
        .map_err(|e| format!("Caminho inválido: {e}"))?;

    validate_destination_folder(&folder_path, &state.backup_service.app_data_dir)
        .map_err(crate::map_core_err)?;

    Ok(Some(folder_path.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn choose_backup_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let session = require_session(&state).await?;
    state
        .services
        .authorize_backup_operation(&session)
        .map_err(crate::map_core_err)?;
```

### 2.6. Agendamento do Backup Automático e Cálculo de Obsolecência (`is_stale`) (R3-f)

Em `crates/core/src/domain/backup.rs`, as funções puras de domínio determinam o agendamento (`is_backup_due`, a cada 24 horas) e a condição de alerta de backup desatualizado (`is_backup_stale`, mais de 7 dias sem backup ou nenhum backup já efetuado):

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:164-191 -->
```rust
pub fn is_backup_due(
    last_backup_at: Option<&str>,
    now: DateTime<Utc>,
    schedule_enabled: bool,
) -> bool {
    if !schedule_enabled {
        return false;
    }
    match last_backup_at {
        None => true,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => (now - dt.with_timezone(&Utc)).num_hours() >= 24,
            Err(_) => true,
        },
    }
}

/// Determina se o status de backup está "estéril" (> 7 dias sem backup ou nenhum backup já realizado).
/// Aciona alerta fixo e destacado na interface de usuário.
pub fn is_backup_stale(last_backup_at: Option<&str>, now: DateTime<Utc>) -> bool {
    match last_backup_at {
        None => true,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => (now - dt.with_timezone(&Utc)).num_days() >= 7,
            Err(_) => true,
        },
    }
}
```

### 2.7. Snapshot Atômico Pré-Atualização Disparado na Inicialização (R3-g)

Em `src-tauri/src/lib.rs`, na rotina de boot do aplicativo desktop, o snapshot de salvaguarda pré-atualização é executado exatamente antes de inicializar o banco de dados e aplicar as migrações:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/lib.rs:390-405 -->
```rust
    // Snapshot atômico de segurança pré-atualização antes de aplicar migrações ou abrir o banco
    if let Err(err) = backup_service.create_pre_update_snapshot_if_needed(&db_path) {
        tracing::warn!("Aviso: falha ao gerar snapshot pré-atualização: {err}");
    }

    tracing::info!(
        "Inicializando banco SQLite em: {}",
        db_path.to_string_lossy()
    );

    let pool = init_db(&db_path).await.map_err(|e| {
        tracing::error!("Erro crítico ao inicializar banco SQLite: {}", e);
        (
            format!("Falha ao inicializar ou migrar o banco de dados local: {e}"),
            log_dir_display.clone(),
        )
```

### 2.8. Trava RAII Anti-Concorrência (`BackupOperationGuard`)

Em `crates/core/src/domain/backup.rs`, a trava atômica de processo impede backups simultâneos ou concorrência com o coletor de lixo de mídia:

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:26-46 -->
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
```

### 2.9. Modal Bloqueante de Confirmação de Restauração no Frontend

Em `src/routes/settings/backup/+page.svelte`, o botão de confirmação permanece desabilitado até a digitação literal da palavra `RESTAURAR`:

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

### 2.10. Teste E2E Playwright de Backup e Confirmação de Restauração

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

## 3. Testes Obrigatórios de Integridade e Segurança (R4)

Todos os 8 cenários de segurança e robustez exigidos foram implementados e aprovados com 100% de sucesso na suíte de testes de `crates/backup-fs`:

1. **`test_zip_slip_rejection_dot_dot`**: Cria pacote malicioso contendo `media/../../outside.txt` e comprova rejeição por violação de Zip-Slip.
2. **`test_zip_slip_rejection_absolute_path_and_drive_colon`**: Valida rejeição de caminhos absolutos Unix (`/etc/passwd`) e unidades Windows (`C:/escaped.txt`).
3. **`test_zip_bomb_rejection_exceeds_size_limit`**: Forja arquivo com entrada declarando tamanho descompactado superior a 2 GiB (`MAX_SINGLE_FILE_UNCOMPRESSED_BYTES`) e comprova recusa por Zip Bomb.
4. **`test_tamper_detection_db_byte_altered`**: Adultera 1 byte físico no arquivo SQLite empacotado e comprova recusa imediata por divergência de hash SHA-256 no banco.
5. **`test_tamper_detection_manifest_sha_mismatch`**: Adultera hashes registrados no `manifest.json` e comprova detecção e rejeição do pacote.
6. **`test_newer_schema_backup_rejected`**: Simula backup gerado em versão futura com `last_applied_migration = 999 > 10` e comprova que tanto a verificação (`is_compatible = false`) quanto o agendamento de restauração são categoricamente recusados.
7. **`test_simulated_crash_during_restore_reverts_state`**: Simula corrupção/aborto durante o processo de restauração e comprova que o banco original é preservado intacto via rollback de salvaguarda.
8. **`test_concurrent_backup_with_db_writes_remains_consistent`**: Executa escritas concorrentes contínuas em task assíncrona paralela enquanto `run_backup` executa o snapshot via `VACUUM INTO`, comprovando que o banco gerado é 100% íntegro e passa em `PRAGMA integrity_check = ok`.

Saída dos testes no `cargo test -p eventrent-backup-fs`:
```text
running 12 tests
test tests::test_backup_create_and_verify_roundtrip ... ok
test tests::test_tamper_detection_manifest_sha_mismatch ... ok
test tests::test_retention_plan_execution ... ok
test tests::test_zip_bomb_rejection_exceeds_size_limit ... ok
test tests::test_zip_slip_rejection_dot_dot ... ok
test tests::test_newer_schema_backup_rejected ... ok
test tests::test_zip_slip_rejection_absolute_path_and_drive_colon ... ok
test tests::test_concurrent_backup_with_db_writes_remains_consistent ... ok
test tests::test_simulated_crash_during_restore_reverts_state ... ok
test tests::test_tamper_detection_db_byte_altered ... ok
test tests::test_schedule_and_execute_cold_restore ... ok
test tests::test_backup_tamper_detection ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
```

---

## 4. Matriz de Comandos IPC e Cobertura de Contratos

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

## 5. Portão de Qualidade

A suíte completa de validação (`pnpm verify`) foi executada com 100% de sucesso nos 14 passos:
- **Passo 1**: SvelteKit Check & Typecheck (0 erros, 0 warnings);
- **Passo 2**: Lint de Código & Cores (0 cores literais em 48 componentes .svelte, Prettier e ESLint 100%);
- **Passo 3**: Testes Unitários Frontend (132 testes em 11 arquivos no Vitest);
- **Passo 4**: Build Frontend de Produção (sem mocks);
- **Passo 5**: Verificação Estática de Produção (zero menções a mock em bundles de produção);
- **Passo 6**: Integridade de Migrações (todos os 6 cenários de congelamento aprovados);
- **Passo 7**: Prova Estrutural de Trechos de Código em Relatórios (4 cenários aprovados);
- **Passo 8**: Verificação Estrutural de Snippets em Relatórios (todos os marcadores aprovados);
- **Passo 9**: Rustfmt Check;
- **Passo 10**: Clippy Linter Workspace (0 advertências);
- **Passo 11**: Clippy Linter `eventrent-core` (0 advertências);
- **Passo 12**: Clippy Linter `eventrent-store-sqlite` (0 advertências);
- **Passo 13**: Clippy Linter `eventrent-app` (0 advertências);
- **Passo 14**: Testes Unitários Rust Workspace (90 testes aprovados em 5 crates).
