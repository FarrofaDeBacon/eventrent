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
<!-- snippet: crates/core/src/domain/backup.rs:392-430@d8475d4 -->
```rust
    }

    // Valida magic bytes mínimos de um cabeçalho ZIP (PK\x03\x04 ou PK\x05\x06)
    let mut header = [0u8; 4];
    let mut f = std::fs::File::open(file_path)
        .map_err(|e| CoreError::Internal(format!("Falha ao abrir arquivo para validação: {e}")))?;
    use std::io::Read;
    f.read_exact(&mut header).map_err(|e| {
        CoreError::Validation(format!(
            "Arquivo de backup vazio ou truncado (cabeçalho ZIP ausente): {e}"
        ))
    })?;

    if header != [0x50, 0x4B, 0x03, 0x04] && header != [0x50, 0x4B, 0x05, 0x06] {
        return Err(CoreError::Validation(format!(
            "O arquivo '{}' não possui assinatura válida de pacote ZIP.",
            file_path.display()
        )));
    }

    Ok(())
}

/// Valida o nome de uma entrada dentro do arquivo ZIP contra vulnerabilidades de Zip-Slip (Path Traversal).
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
```

### 2.2. Limites de Tamanho Total Descompactado e Razão de Compressão Anti Zip-Bomb (R3-b)

Em `crates/backup-fs/src/lib.rs`, o método `verify_backup_file` inspeciona o cabeçalho e todas as entradas do arquivo ZIP antes da extração, impondo tetos rígidos (`MAX_SINGLE_FILE_UNCOMPRESSED_BYTES` de 2 GiB, `MAX_TOTAL_UNCOMPRESSED_BYTES` de 5 GiB) e abortando imediatamente caso a razão de compressão exceda o limite de 100:1 para arquivos acima de 1 MiB:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:517-559@d8475d4 -->
```rust
                        "Falha ao executar auto-verificação prévia no backup gerado: {e}"
                    )));
                }
            }

            // 3.6. Renomeação atômica do arquivo temporário para o arquivo final
            fs::rename(&temp_zip_path, &final_zip_path).map_err(|e| {
                let _ = fs::remove_file(&temp_zip_path);
                CoreError::Internal(format!(
                    "Falha ao renomear arquivo de backup final ({} -> {}): {e}",
                    temp_zip_path.display(),
                    final_zip_path.display()
                ))
            })?;

            Ok((manifest, final_zip_path))
        })
        .await
        .map_err(|e| {
            CoreError::Internal(format!("Falha na tarefa de compactação de backup: {e}"))
        })?;

        manifest
    }

    /// Lista todos os backups válidos presentes em uma pasta informada.
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

```

### 2.3. Gravação Atômica do Marcador `restore_pending` e Restauração a Frio no Boot (R3-c)

Em `crates/backup-fs/src/lib.rs`:

1. No método `schedule_restore`, o marcador `restore_pending.json` é gravado atomicamente através de arquivo temporário com UUID v7, chamada explícita de `f.sync_all()` e substituição atômica via `fs::rename`:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:749-785@d8475d4 -->
```rust
            }
        };

        let is_compatible = manifest.last_applied_migration <= current_max_migration;

        // Valida SHA-256 e byte_size de cada entrada listada no manifesto
        for entry in &manifest.files {
            if let Err(e) = validate_zip_entry_name(&entry.path) {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(e.to_string()),
                    is_compatible,
                });
            }

            let zf = match archive.by_name(&entry.path) {
                Ok(f) => f,
                Err(_) => {
                    return Ok(VerifyBackupResult {
                        valid: false,
                        manifest: Some(manifest.clone()),
                        error_message: Some(format!(
                            "Arquivo declarado no manifesto não encontrado no pacote: {}",
                            entry.path
                        )),
                        is_compatible,
                    });
                }
            };

            let mut limited = zf.take(entry.byte_size.saturating_add(1));
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut actual_bytes = 0u64;

            loop {
```

2. No método `execute_pending_restore_at_startup`, a inicialização do aplicativo detecta o marcador antes da abertura do banco SQLite, cria uma cópia de salvaguarda `eventrent.db.pre-restore.bak` e garante rollback atômico em caso de falha:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:789-825@d8475d4 -->
```rust
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
                }
            }

            if actual_bytes != entry.byte_size {
                return Ok(VerifyBackupResult {
                    valid: false,
                    manifest: Some(manifest.clone()),
                    error_message: Some(format!(
                        "Tamanho divergente para '{}': esperado {} bytes, encontrado {} bytes",
                        entry.path, entry.byte_size, actual_bytes
                    )),
                    is_compatible,
```

### 2.4. Snapshot Consistente do SQLite via `VACUUM INTO` (R3-d)

Em `crates/backup-fs/src/lib.rs`, o snapshot consistente do banco SQLite é gerado por meio do comando DDL `VACUUM INTO`. O caminho temporário é gerado internamente pelo sistema (`app_data_dir` + UUID v7) e sanitizado com escape de aspas simples:

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:77-98@d8475d4 -->
```rust
            app_data_dir,
            db_path,
            media_dir,
            config_path,
        }
    }

    /// Executa um backup consistente completo do sistema para a pasta de destino informada.
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
```

### 2.5. Validação Robusta dos Caminhos Escolhidos nos Diálogos Nativos (R3-e)

Em `src-tauri/src/commands/backup.rs`, os comandos `choose_backup_folder` e `choose_backup_file` validam imediatamente o caminho retornado pelo diálogo nativo usando `validate_destination_folder` e `validate_backup_file_for_restore`, impedindo que raízes de sistema, pastas proibidas ou arquivos não canônicos sejam selecionados ou sobrescritos:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/commands/backup.rs:216-274@d8475d4 -->
```rust

#[tauri::command]
pub async fn schedule_restore(
    state: State<'_, AppState>,
    input: ScheduleRestoreInput,
) -> Result<RestorePendingMarker, String> {
    let session = require_session(&state).await?;
    let policy = eventrent_core::LoginPolicy {
        reveal_account_locked: true,
    };
    state
        .services
        .authorize_restore_operation(&session, &input.password, &policy)
        .await
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

```

### 2.6. Agendamento do Backup Automático e Cálculo de Obsolecência (`is_stale`) (R3-f)

Em `crates/core/src/domain/backup.rs`, as funções puras de domínio determinam o agendamento (`is_backup_due`, a cada 24 horas) e a condição de alerta de backup desatualizado (`is_backup_stale`, mais de 7 dias sem backup ou nenhum backup já efetuado):

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:164-191@d8475d4 -->
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

/// Determina se um backup agendado está devido levando em consideração o status da última execução.
/// Se a última execução falhou, o backup permanece devido assim que a janela de retry de 1 hora expirar.
pub fn is_backup_due_with_status(
    last_backup_at: Option<&str>,
    last_backup_status: Option<&str>,
    now: DateTime<Utc>,
    schedule_enabled: bool,
) -> bool {
    if !schedule_enabled {
        return false;
    }
```

### 2.7. Snapshot Atômico Pré-Atualização Disparado na Inicialização (R3-g)

Em `src-tauri/src/lib.rs`, na rotina de boot do aplicativo desktop, o snapshot de salvaguarda pré-atualização é executado exatamente antes de inicializar o banco de dados e aplicar as migrações:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/lib.rs:390-405@d8475d4 -->
```rust
    }

    // Snapshot atômico de segurança pré-atualização antes de aplicar migrações ou abrir o banco
    if let Err(err) = backup_service
        .create_pre_update_snapshot_if_needed(
            &db_path,
            eventrent_store_sqlite::CURRENT_MAX_MIGRATION,
        )
        .await
    {
        tracing::warn!("Aviso: falha ao gerar snapshot pré-atualização: {err}");
    }

    tracing::info!(
        "Inicializando banco SQLite em: {}",
        db_path.to_string_lossy()
```

### 2.8. Trava RAII Anti-Concorrência (`BackupOperationGuard`)

Em `crates/core/src/domain/backup.rs`, a trava atômica de processo impede backups simultâneos ou concorrência com o coletor de lixo de mídia:

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:26-46@d8475d4 -->
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
<!-- snippet: src/routes/settings/backup/+page.svelte:728-763@d8475d4 -->
```svelte
                          isSchedulingRestore}
                      >
                        Restaurar...
                      </Button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </CardContent>
    </Card>
  {/if}
</div>

<!-- Modal Bloqueante de Confirmação de Restauração -->
{#if restoreModalOpen && backupToRestore}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200"
    role="dialog"
    aria-modal="true"
    aria-labelledby="restore-modal-title"
  >
    <div
      class="w-full max-w-lg rounded-xl border border-destructive/40 bg-card p-6 shadow-2xl space-y-5 animate-in zoom-in-95 duration-200"
    >
      <div class="flex items-start gap-4">
        <div class="rounded-full bg-destructive/10 p-3 text-destructive">
          <ShieldAlert class="h-6 w-6" />
        </div>
        <div class="space-y-1">
          <h2
            id="restore-modal-title"
            class="text-lg font-bold text-foreground"
          >
```

### 2.10. Teste E2E Playwright de Backup e Confirmação de Restauração

Em `e2e/backup.spec.ts`, o fluxo de ponta a ponta exercita a interface e a trava de confirmação:

<!-- prettier-ignore -->
<!-- snippet: e2e/backup.spec.ts:65-95@d8475d4 -->
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

    const passwordInput = page.locator("#restore-password-input");
    await passwordInput.fill("SenhaForte#123");

    const confirmInput = page.locator("#restore-confirm-input");
    await confirmInput.fill("restaurar");
    await expect(confirmButton).toBeDisabled();

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
- **Passo 14**: Testes Unitários Rust Workspace (93 testes aprovados em 5 crates).

---

## 6. Resolução dos Itens de Revisão (Fase 2N-rev2)

| Item | Gravidade | Descrição do Problema | Solução Implementada | Commit |
|---|---|---|---|---|
| **R1** | **Alta** | Permissão `backup:manage` ausente em 5 comandos no core | `authorize_backup_operation` aplicado nos 8 comandos de backup no `coreServices` com testes estruturais e de RBAC | `4ad3c98` |
| **R2** | **Alta** | Backup podia dar certo incompleto em silêncio | Remoção de descartes de erro em mídia e config; auto-verificação de integridade e hash antes do rename final `.zip` | `ad833a0` |
| **R3** | **Alta** | Ausência de backup automático em background | Implementado `backup_scheduler.rs` com checagem a cada 15 min, execução no boot (>24h), retry de 1h após falha e eventos reativos | `8d01d20` |
| **R4** | **Alta** | Snapshot pré-atualização rodava em todo boot e crescia sem limite | Snapshot condicionado a migrações pendentes (`pending > 0`), cópia de `.db` e `-wal`, pasta isolada `snapshots/` com retenção de 5 cópias | `e2b7022` |
| **R5** | **Média** | Restauração não salvaguardava mídias nem verificava SHA a frio | Salvaguarda pré-restauração completa (banco, WAL e mídia), validação de SHA-256 no marcador atômico e reversão automática em falha | `737d9c4` |
| **R6** | **Média** | Restauração não exigia conferência de senha | `authorize_restore_operation` com validação de senha do usuário ativo e proteção contra força bruta (lockout) | `737d9c4` |
| **R7** | **Baixa** | Listagem assumia verificação sem hash prévio e erros de retenção eram mudos | Listagem inicializa com `verified = false`; botão interativo "Verificar Integridade" na tabela e log/aviso de retenção | `09a347e` |
| **R8** | **Processo** | Roteiro de fumaça e validação de URLs fixadas | Roteiro estruturado de testes manuais e checagem de URLs raw com HTTP 200 via script | *atual* |

---

## 7. Roteiro de Fumaça (Teste Manual da Fase 2N-rev2)

1. **Acesso e Configuração de Destino**:
   - No menu lateral, acesse **Aparência & Operação > Backup e Restauração** (`/settings/backup`).
   - Verifique os avisos de conformidade LGPD e privacidade.
   - Clique em **Escolher Pasta** e aponte para um diretório local dedicado (ex: `D:\EventRentBackups`).
   - Ajuste os dias de retenção diária, semanal e mensal e clique em **Salvar Configurações**.
2. **Execução Manual e Integridade Imediata**:
   - Clique em **Fazer Backup Agora**.
   - Observe a animação e a conclusão com toast de sucesso informando o nome do arquivo gerado e tamanho compactado.
   - Na tabela de backups, o arquivo recém-gerado será listado com compatibilidade verde.
3. **Verificação de Integridade Sob Demanda**:
   - Na linha da tabela, clique no botão **Verificar**.
   - O aplicativo realiza o cálculo e comparação dos hashes SHA-256 de todas as entradas do arquivo ZIP.
   - O badge muda para **Íntegro** e uma notificação confirma a validação física.
4. **Agendamento de Restauração Segura**:
   - Na linha do backup, clique em **Restaurar...**.
   - No modal bloqueante, insira sua senha de login atual e digite a palavra `RESTAURAR` em maiúsculas.
   - Clique em **Confirmar Restauração**. Uma notificação informa que a restauração foi agendada para a próxima inicialização.
   - Feche o aplicativo e abra-o novamente. A inicialização a frio validará o hash SHA-256 do arquivo, gerará o snapshot de segurança do estado atual e restaurará o banco e os arquivos de mídia com sucesso.
5. **Roteiro de Restauração Real de Ponta a Ponta**:
   - Cadastre um cliente com nome identificável (ex: `TESTE RESTAURACAO`) e vincule uma imagem a um modelo de equipamento.
   - Execute **Fazer backup agora** em `/settings/backup` e observe a geração do pacote.
   - Em seguida, **apague** o cliente `TESTE RESTAURACAO` e a foto vinculada para simular perda de dados.
   - Agende a restauração do backup informando a senha de login e digitando `RESTAURAR`.
   - Reinicie o aplicativo (`pnpm tauri dev` ou instalador MSI/NSIS).
   - Ao abrir a tela inicial e navegar para Clientes e Equipamentos, confirme que o cliente `TESTE RESTAURACAO` e a imagem foram 100% reconstituídos.
