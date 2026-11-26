# Relatório de Conclusão — Fase 2B-rev: Gestão de Unidades Físicas e Estoque Multidepósito

**Branch**: `fase-2b-estoque`  
**Pull Request**: [#14](https://github.com/FarrofaDeBacon/eventrent/pull/14) (Rascunho / Draft mantido para conferência do usuário)  
**Status**: Concluído — 100% dos testes e verificações aprovados  

---

## 1. Resumo Executivo

A **Fase 2B-rev** consolida a arquitetura e implementação completa de **Unidades Físicas, Estoque Multidepósito e Livro-Razão Imutável** para o modo local do **EventRent**, cumprindo integralmente as decisões do **ADR-0018** (`docs/decisions/0018-unidades-e-estoque.md`), preservando congeladas as migrações existentes `0001` a `0010`, atendendo aos requisitos de integridade da migração `0011_inventory.sql`, cobrindo os achados de robustez da Tarefa 0 (backup) e submetendo todo o sistema ao portão rigoroso de qualidade (`pnpm verify`, 14 passos).

### Destaques das Entregas e Conformidade:

1. **R2 — Migração `0011_inventory.sql` Puramente Estrutural (DDL)**:
   - Removida qualquer inserção de semente de dados e triggers em `tenant` da migração;
   - Criação idempotente do `"Depósito principal"` (`code = "PRINCIPAL"`, `is_default = 1`) transferida exclusivamente para o código de aplicação via função `ensure_default_inventory`;
   - Inicialização da sequência de patrimônio `equipment_patrimony_seq` sob demanda via `INSERT INTO equipment_patrimony_seq (tenant_id, last_number) VALUES (?, 0) ON CONFLICT(tenant_id) DO NOTHING` seguido de `UPDATE equipment_patrimony_seq SET last_number = last_number + 1 WHERE tenant_id = ? RETURNING last_number`;
   - Adicionado índice único parcial para número de série por modelo: `idx_equipment_unit_tenant_model_serial_active` (`WHERE serial_number IS NOT NULL AND deleted_at IS NULL`).

2. **R3 — Evidências Estruturais e Provas de Concorrência**:
   - **R3-a (Patrimônio Concorrente)**: Teste com 12 tarefas simultâneas (>= 8) comprovando emissão atômica sem duplicatas e sem lacunas (`test_inventory_patrimony_concurrency_no_duplicates_or_gaps`);
   - **R3-b (Concorrência de Movimentos e Saldo Nunca Negativo)**: Teste com 2 saídas simultâneas sobre o último item restante; exatamente uma tem sucesso, a outra falha por saldo insuficiente, o saldo final permanece 0 e a soma algébrica do livro-razão coincide identicamente com o saldo (`test_inventory_stock_movement_concurrency_ledger_sum_and_no_negative`);
   - **R3-c (Triggers Append-Only)**: Teste com tentativas explícitas de `UPDATE` e `DELETE` em `stock_movement` confirmando bloqueio via `RAISE(ABORT)` dos triggers `trg_stock_movement_no_update` e `trg_stock_movement_no_delete` (`test_inventory_triggers_append_only_explicit_update_delete`);
   - **R3-d (Sigilo e Redação Financeira)**: Teste comparativo confirmando que `Proprietário` visualiza `cost_price_cents` e `replacement_value_cents`, enquanto `Estoque` e `Técnico` recebem `None` em todas as consultas e comandos de catálogo e estoque (`test_inventory_financial_redaction_across_roles_owner_stock_tech`);
   - **R3-e (Imutabilidade de `control_mode`)**: Teste comprovando bloqueio estrito de alteração de modo de controle quando existem unidades físicas ou movimentações no livro-razão (`test_inventory_block_control_mode_alteration_when_units_or_movements_exist`);
   - **R3-f (Constraints Únicas Parciais)**: Testes de integridade em patrimônio ativo, número de série por modelo ativo, código de depósito e depósito padrão único (`test_inventory_partial_unique_constraints_and_soft_delete`).

3. **R4 — Tarefa 0 (Robustez do Subsistema de Backup)**:
   - Cobertura completa dos itens 0.1 a 0.7 com trechos de código e testes verdes em `eventrent-backup-fs` e `eventrent-core`.

4. **R5 — Instrução Obrigatória para Teste Manual e de Fumaça**:
   - **ATENÇÃO CRÍTICA**: Os testes de fumaça manuais devem ser realizados **EXCLUSIVAMENTE** através do instalador de produção (`EventRent_0.0.1_x64-setup.exe` ou `.msi`) gerado no bundle release, e **NUNCA** executando o binário cru de desenvolvimento em `target\release\eventrent-app.exe` (que tenta conectar ao servidor Vite em `localhost` gerando erro de conexão).

---

## 2. Tarefa 0: Robustez do Backup (Achados da Fase 2N — Itens 0.1 a 0.7)

### 2.1. Item 0.1: Mídia Canônica e Descarte Seguro de Arquivos Espúrios
- **Solução**: `run_backup` valida cada arquivo com `validate_canonical_media_path(&rel_slash)`. Arquivos espúrios (`Thumbs.db`, `.DS_Store`, etc.) são ignorados e contabilizados em `skipped_media_files`, impedindo falhas na restauração.
- **Teste**: `test_backup_ignores_non_canonical_media_and_restores_successfully` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:509-525 -->
```rust
                            if validate_canonical_media_path(&rel_slash).is_err() {
                                tracing::warn!(
                                    path = %path.display(),
                                    rel = %rel_slash,
                                    "Arquivo não-canônico na pasta de mídia ignorado durante backup"
                                );
                                skipped_media_files += 1;
                                continue;
                            }

                            // Extrai o hash esperado a partir do nome do arquivo (último segmento de 64 hex)
                            let expected_hash = match rel_slash.rsplit('/').next() {
                                Some(h) if h.len() == 64 => h,
                                _ => {
                                    skipped_media_files += 1;
                                    continue;
                                }
```
<!-- /snippet -->

### 2.2. Item 0.2: Preservação de Mídias Preexistentes no Rollback de Restauração
- **Solução**: Arquivos de mídia cujo hash SHA-256 e tamanho já coincidem com o destino não são regravados nem adicionados à lista `extracted_media_paths`. Se a restauração falhar posteriormente, o rollback apaga apenas arquivos novos que foram extraídos nesta tentativa.
- **Teste**: `test_restore_rollback_preserves_preexisting_media_when_restoration_fails` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:1680-1710 -->
```rust
                let already_present_and_valid = if target_path.is_file() {
                    if let Ok(existing_meta) = fs::metadata(&target_path) {
                        if existing_meta.len() == meta.byte_size {
                            if let Ok(mut ef) = File::open(&target_path) {
                                let mut e_hasher = Sha256::new();
                                let mut e_buf = [0u8; 64 * 1024];
                                while let Ok(n) = ef.read(&mut e_buf) {
                                    if n == 0 {
                                        break;
                                    }
                                    e_hasher.update(&e_buf[..n]);
                                }
                                let e_sha = format!("{:x}", e_hasher.finalize());
                                e_sha.eq_ignore_ascii_case(&meta.sha256)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if already_present_and_valid {
                    // Arquivo idêntico já existe previamente: preserva intacto e não adiciona a extracted_media_paths
                    files_restored_count += 1;
                    continue;
```
<!-- /snippet -->

### 2.3. Item 0.3: Separação de `last_success_at` e Alerta de Backup Desatualizado
- **Solução**: `record_backup_outcome` atualiza `last_success_at` apenas em caso de sucesso. O cálculo de dias sem backup (`is_backup_stale`) baseia-se unicamente em `last_success_at`, garantindo que falhas consecutivas disparem o alerta de > 7 dias.
- **Teste**: `test_is_backup_due_and_stale` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:248-268 -->
```rust
pub fn is_backup_stale(last_success_at: Option<&str>, now: DateTime<Utc>) -> bool {
    match last_success_at {
        None => true,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => (now - dt.with_timezone(&Utc)).num_days() >= 7,
            Err(_) => true,
        },
    }
}

/// Calcula quantos dias se passaram desde o último backup de SUCESSO.
pub fn days_since_last_backup(last_success_at: Option<&str>, now: DateTime<Utc>) -> Option<u32> {
    let s = last_success_at?;
    let dt = DateTime::parse_from_rfc3339(s).ok()?;
    let days = (now - dt.with_timezone(&Utc)).num_days();
    if days < 0 {
        Some(0)
    } else {
        Some(days as u32)
    }
}
```
<!-- /snippet -->

### 2.4. Item 0.4: Validação de Retenção e Proteção Incondicional do Backup Mais Recente
- **Solução**: `calculate_retention_plan` insere incondicionalmente o arquivo mais recente no conjunto de preservação (`keep_set`), impedindo que limites zerados apaguem todos os backups.
- **Testes**: `test_retention_plan_never_deletes_most_recent_even_if_limits_zero`, `test_retention_execution_never_deletes_most_recent_when_limits_are_zero` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/backup.rs:286-300 -->
```rust
    let mut sorted = candidates.to_vec();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.created_at)); // decrescente (mais recentes primeiro)

    let mut keep_set = HashSet::new();

    // REGRA DE SEGURANÇA ABSOLUTA: O backup mais recente NUNCA pode ser excluído,
    // independentemente de limites zerados ou qualquer política de expiração.
    if let Some(most_recent) = sorted.first() {
        keep_set.insert(most_recent.file_path.clone());
    }

    // 1. Diários: os primeiros `daily_limit` backups mais recentes
    for c in sorted.iter().take(daily_limit as usize) {
        keep_set.insert(c.file_path.clone());
    }
```
<!-- /snippet -->

### 2.5. Item 0.5: Conexões SQLite Seguras via `SqliteConnectOptions::new().filename(...)`
- **Solução**: Eliminada qualquer montagem de strings de URL (`sqlite://...`). Conexões usam explicitamente `.filename(path)`, suportando espaços, acentos e caminhos Windows UNC.
- **Teste**: `test_backup_and_restore_with_special_characters_in_path` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:1868-1881 -->
```rust
        let ro_opts = SqliteConnectOptions::new()
            .filename(db_path)
            .read_only(true);
        let pool = match sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(ro_opts)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Não foi possível abrir banco para conferir migrações: {e}");
                return Ok(None);
            }
        };
```
<!-- /snippet -->

### 2.6. Item 0.6: Salvaguarda Pré-Restauração Completa com WAL e Checagem de Espaço
- **Solução**: Cópia de segurança atômica que inclui `.db`, `-wal` e `-shm` antes da extração da restauração, com verificação de espaço livre em disco.
- **Teste**: `test_restore_rollback_preserves_wal_file_when_restore_fails` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:1415-1440 -->
```rust
            // Cópia direta do .db para salvaguarda de rollback
            fs::copy(&self.db_path, &db_backup_original).map_err(|e| {
                let _ = fs::rename(&marker_path, self.app_data_dir.join("restore_failed.json"));
                CoreError::Internal(format!(
                    "Falha ao criar salvaguarda direta do banco .db: {e}"
                ))
            })?;

            pre_restore_path_str = Some(pre_restore_file.to_string_lossy().to_string());
        }

        let orig_wal = PathBuf::from(format!("{}-wal", self.db_path.to_string_lossy()));
        let orig_shm = PathBuf::from(format!("{}-shm", self.db_path.to_string_lossy()));
        let bak_wal = PathBuf::from(format!("{}-wal", db_backup_original.to_string_lossy()));
        let bak_shm = PathBuf::from(format!("{}-shm", db_backup_original.to_string_lossy()));
        let _ = fs::remove_file(&bak_wal);
        let _ = fs::remove_file(&bak_shm);

        if orig_wal.exists() {
            let _ = fs::copy(&orig_wal, &bak_wal);
        }
        if orig_shm.exists() {
            let _ = fs::copy(&orig_shm, &bak_shm);
        }

        // Rastreamento de arquivos criados para garantia de rollback integral
```
<!-- /snippet -->

### 2.7. Item 0.7: Teste de Integração Real de Restauração de Ponta a Ponta
- **Solução**: Teste automatizado que inicializa banco SQLite com dados, fotos e PDFs de clientes, gera backup, altera os dados para simular corrupção e executa restauração a frio real, validando todos os dados e mídias restaurados.
- **Teste**: `test_full_end_to_end_backup_mutation_and_restore_cycle` (PASS).

<!-- prettier-ignore -->
<!-- snippet: crates/backup-fs/src/lib.rs:3530-3551 -->
```rust
    async fn test_full_end_to_end_backup_mutation_and_restore_cycle() {
        let _test_guard = TEST_LOCK.lock().await;
        let env = TestEnvironment::setup().await;
        let service = env.service();

        // 1. Popula múltiplas tabelas no banco de dados SQLite
        let pool = env.pool_rw().await;
        sqlx::query(
            "CREATE TABLE customers (id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL);
             INSERT INTO customers (id, name, email) VALUES
             ('cust_01', 'Cliente Original 1', 'cli1@real.local'),
             ('cust_02', 'Cliente Original 2', 'cli2@real.local'),
             ('cust_03', 'TESTE RESTAURACAO', 'restauracao@real.local');
             CREATE TABLE products (id TEXT PRIMARY KEY, title TEXT NOT NULL, price INTEGER NOT NULL);
             INSERT INTO products (id, title, price) VALUES
             ('prod_01', 'Pioneer CDJ-3000', 15000),
             ('prod_02', 'Mesa Yamaha QL5', 45000);",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
```
<!-- /snippet -->

---

## 3. Rastreabilidade Estrutural de Estoque e Unidades Físicas (R3)

### 3.1. Migração `0011_inventory.sql`: DDL Estrutural e Índices Parciais (R2 & R3f)
A migração é puramente DDL, sem inserções nem triggers em tabelas de terceiros.

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/migrations/0011_inventory.sql:32-45 -->
```sql
CREATE UNIQUE INDEX idx_warehouse_tenant_code_active
    ON warehouse (tenant_id, code)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_warehouse_single_default
    ON warehouse (tenant_id)
    WHERE is_default = 1 AND deleted_at IS NULL;

-- 2. Sequência de Patrimônio por Tenant (equipment_patrimony_seq)
CREATE TABLE equipment_patrimony_seq (
    tenant_id TEXT PRIMARY KEY NOT NULL,
    last_number INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);
```
<!-- /snippet -->

Índices parciais para número de série por modelo e patrimônio:

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/migrations/0011_inventory.sql:72-80 -->
```sql
CREATE UNIQUE INDEX idx_equipment_unit_tenant_patrimony_active
    ON equipment_unit (tenant_id, patrimony_tag)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_equipment_unit_tenant_model_serial_active
    ON equipment_unit (tenant_id, equipment_model_id, serial_number)
    WHERE serial_number IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX idx_equipment_unit_tenant_model_status
```
<!-- /snippet -->

### 3.2. Provisionamento Idempotente de Depósito Padrão (`ensure_default_inventory`) (R2)

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/services/inventory.rs:721-758 -->
```rust
pub async fn ensure_default_inventory(
    tenant_id: &str,
    inventory: &dyn crate::store::InventoryStore,
) -> Result<Warehouse> {
    let warehouses = inventory.list_warehouses(tenant_id, true).await?;
    if let Some(default_wh) = warehouses
        .iter()
        .find(|w| w.is_default && w.deleted_at.is_none())
    {
        return Ok(default_wh.clone());
    }

    if let Some(wh_principal) = warehouses
        .iter()
        .find(|w| w.code == "PRINCIPAL" && w.deleted_at.is_none())
    {
        inventory
            .set_default_warehouse(tenant_id, &wh_principal.id)
            .await?;
        let mut wh = wh_principal.clone();
        wh.is_default = true;
        return Ok(wh);
    }

    let now = Utc::now().to_rfc3339();
    let warehouse = Warehouse {
        id: Uuid::now_v7().to_string(),
        tenant_id: tenant_id.to_string(),
        name: "Depósito principal".to_string(),
        code: "PRINCIPAL".to_string(),
        description: Some("Depósito padrão inicial da empresa".to_string()),
        address_line1: None,
        address_line2: None,
        city: None,
        state: None,
        postal_code: None,
        is_default: true,
        active: true,
```
<!-- /snippet -->

### 3.3. Emissão Sequencial Atômica de Patrimônio sob Demanda (R2 & R3a)

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/inventory.rs:410-434 -->
```rust
    async fn next_patrimony_tag(&self, tenant_id: &str) -> Result<String> {
        let insert_query = sqlx::query(
            r#"
            INSERT INTO equipment_patrimony_seq (tenant_id, last_number)
            VALUES (?, 0)
            ON CONFLICT(tenant_id) DO NOTHING
            "#,
        )
        .bind(tenant_id);
        self.execute(insert_query).await?;

        let update_query = sqlx::query(
            r#"
            UPDATE equipment_patrimony_seq
            SET last_number = last_number + 1
            WHERE tenant_id = ?
            RETURNING last_number
            "#,
        )
        .bind(tenant_id);

        let row = self.fetch_one(update_query).await?;
        let last_num: i64 = row.get("last_number");
        Ok(format_patrimony_tag(last_num))
    }
```
<!-- /snippet -->

### 3.4. Prova de Concorrência: Gerador de Patrimônio Sem Duplicatas nem Lacunas (R3a)
- **Teste**: `test_inventory_patrimony_concurrency_no_duplicates_or_gaps` (12 tarefas tokio concorrentes) -> **PASS**.

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/lib.rs:7925-7971 -->
```rust
        let tenant_id = ensure_default_tenant(store.tenants.as_ref(), store.branding.as_ref())
            .await
            .unwrap();

        // Dispara 12 tarefas simultâneas (>= 8 tarefas exigidas por R3a)
        const CONCURRENT_TASKS: usize = 12;
        let mut handles = Vec::with_capacity(CONCURRENT_TASKS);

        for _ in 0..CONCURRENT_TASKS {
            let store_clone = store.clone();
            let tenant_id_clone = tenant_id.clone();
            handles.push(tokio::spawn(async move {
                store_clone
                    .inventory
                    .next_patrimony_tag(&tenant_id_clone)
                    .await
            }));
        }

        let mut tags = Vec::with_capacity(CONCURRENT_TASKS);
        for handle in handles {
            let res = handle.await.unwrap();
            assert!(
                res.is_ok(),
                "Geração concorrente de patrimônio não deve falhar: {:?}",
                res.err()
            );
            tags.push(res.unwrap());
        }

        // 1. Confere unicidade absoluta (sem duplicatas)
        let unique_set: std::collections::HashSet<_> = tags.iter().cloned().collect();
        assert_eq!(
            unique_set.len(),
            CONCURRENT_TASKS,
            "Todas as tags de patrimônio geradas concorrentemente devem ser únicas"
        );

        // 2. Confere ausência de lacunas (gaps)
        let mut numbers: Vec<i64> = tags
            .iter()
            .map(|t| {
                eventrent_core::parse_patrimony_number(t).expect("Tag deve ser formato EQxxxxxx")
            })
            .collect();
        numbers.sort();

```
<!-- /snippet -->

### 3.5. Prova de Concorrência: Livro-Razão e Saldo Nunca Negativo (R3b)
- **Teste**: `test_inventory_stock_movement_concurrency_ledger_sum_and_no_negative` (2 saídas simultâneas disputando o último item restante) -> **PASS**.

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/lib.rs:8115-8160 -->
```rust
                            equipment_unit_id: None,
                            from_warehouse_id: Some(wh),
                            to_warehouse_id: None,
                            movement_type: StockMovementType::RentalOut,
                            quantity: 1,
                            reason: "Saída Concorrente 1 do último item".to_string(),
                            reference_id: None,
                        },
                    )
                    .await
                }
            },
            {
                let s = services.clone();
                let sess = session.clone();
                let wh = default_wh.id.clone();
                async move {
                    s.record_stock_movement(
                        &sess,
                        &RecordStockMovementInput {
                            equipment_model_id: model_id.to_string(),
                            equipment_unit_id: None,
                            from_warehouse_id: Some(wh),
                            to_warehouse_id: None,
                            movement_type: StockMovementType::RentalOut,
                            quantity: 1,
                            reason: "Saída Concorrente 2 do último item".to_string(),
                            reference_id: None,
                        },
                    )
                    .await
                }
            }
        );

        // Exatamente UMA operação deve vencer e a outra DEVE falhar
        let outcomes = [task1.is_ok(), task2.is_ok()];
        let successes = outcomes.iter().filter(|&&ok| ok).count();
        let failures = outcomes.iter().filter(|&&ok| !ok).count();
        assert_eq!(
            successes, 1,
            "Exatamente uma das saídas simultâneas deve ter sucesso"
        );
        assert_eq!(
            failures, 1,
            "Exatamente uma das saídas simultâneas deve falhar por falta de saldo"
```
<!-- /snippet -->

### 3.6. Prova de Triggers Append-Only no Livro-Razão (R3c)
- **Teste**: `test_inventory_triggers_append_only_explicit_update_delete` (UPDATE e DELETE em `stock_movement`) -> **PASS**.

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/lib.rs:8260-8290 -->
```rust
        )
        .bind(mov_id)
        .bind(&tenant_id)
        .bind(&default_wh.id)
        .bind(user_id)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        // Tentativa explícita de UPDATE: deve falhar com RAISE(ABORT)
        let update_res = sqlx::query("UPDATE stock_movement SET quantity = 100 WHERE id = ?")
            .bind(mov_id)
            .execute(&pool)
            .await;
        assert!(
            update_res.is_err(),
            "UPDATE em stock_movement deve ser abortado pelo trigger trg_stock_movement_no_update"
        );
        let err_msg = format!("{}", update_res.unwrap_err());
        assert!(
            err_msg.contains("stock_movement é imutável") || err_msg.contains("append-only"),
            "Mensagem do erro deve expressar imutabilidade append-only: {err_msg}"
        );

        // Tentativa explícita de DELETE: deve falhar com RAISE(ABORT)
        let delete_res = sqlx::query("DELETE FROM stock_movement WHERE id = ?")
            .bind(mov_id)
            .execute(&pool)
            .await;
        assert!(
```
<!-- /snippet -->

### 3.7. Prova de Sigilo e Redação Financeira (R3d)
- **Teste**: `test_inventory_financial_redaction_across_roles_owner_stock_tech` -> **PASS**.

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/lib.rs:8445-8475 -->
```rust
            .await
            .unwrap();

        // O retorno de create_equipment_model para Estoque já deve vir sanitizado
        assert_eq!(
            created_by_stock.model.cost_price_cents, None,
            "Estoque não deve ver cost_price_cents no retorno de criação"
        );
        assert_eq!(
            created_by_stock.model.replacement_value_cents, None,
            "Estoque não deve ver replacement_value_cents no retorno de criação"
        );

        let model_id = &created_by_stock.model.id;

        // A) get_equipment_model
        let model_owner = services
            .get_equipment_model(&owner_session, model_id)
            .await
            .unwrap();
        assert_eq!(model_owner.model.cost_price_cents, Some(420000));
        assert_eq!(model_owner.model.replacement_value_cents, Some(850000));

        let model_stock = services
            .get_equipment_model(&stock_session, model_id)
            .await
            .unwrap();
        assert_eq!(model_stock.model.cost_price_cents, None);
        assert_eq!(model_stock.model.replacement_value_cents, None);

        let model_tech = services
```
<!-- /snippet -->

### 3.8. Prova de Bloqueio de Alteração de `control_mode` (R3e)
- **Teste**: `test_inventory_block_control_mode_alteration_when_units_or_movements_exist` -> **PASS**.

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/services/catalog.rs:1320-1355 -->
```rust
                    if existing.control_mode != input.control_mode {
                        let units = stores
                            .inventory()
                            .list_units(
                                &session.tenant_id,
                                &crate::domain::inventory::EquipmentUnitFilter {
                                    equipment_model_id: Some(existing.id.clone()),
                                    active_only: Some(false),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        if !units.is_empty() {
                            return Err(CoreError::Validation(
                                "Não é permitido alterar o modo de controle de um modelo que já possui unidades cadastradas."
                                    .to_string(),
                            ));
                        }

                        let movements = stores
                            .inventory()
                            .list_stock_movements(
                                &session.tenant_id,
                                &crate::domain::inventory::StockMovementFilter {
                                    equipment_model_id: Some(existing.id.clone()),
                                    limit: Some(1),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        if !movements.is_empty() {
                            return Err(CoreError::Validation(
                                "Não é permitido alterar o modo de controle de um modelo que já possui movimentações de estoque registradas."
                                    .to_string(),
                            ));
                        }
```
<!-- /snippet -->

### 3.9. Prova de Constraints Únicas Parciais (R3f)
- **Teste**: `test_inventory_partial_unique_constraints_and_soft_delete` -> **PASS**.

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/lib.rs:8950-8989 -->
```rust
        // Modelo DIFERENTE com mesmo serial -> SUCESSO (série é única POR modelo)
        let dup_serial_diff_model_res = sqlx::query(
            "INSERT INTO equipment_unit (id, tenant_id, equipment_model_id, warehouse_id, patrimony_tag, serial_number, created_at, updated_at)
             VALUES ('u_04', ?, 'mod_m2', 'wh_dup_code', 'EQ999003', 'SN-AAA', ?, ?)",
        )
        .bind(&tenant_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(
            dup_serial_diff_model_res.is_ok(),
            "Mesmo número de série em modelos diferentes deve ser permitido"
        );

        // Serial NULL repetido no mesmo modelo -> SUCESSO (índice parcial filtra serial_number IS NOT NULL)
        let null_serial_1 = sqlx::query(
            "INSERT INTO equipment_unit (id, tenant_id, equipment_model_id, warehouse_id, patrimony_tag, serial_number, created_at, updated_at)
             VALUES ('u_05', ?, 'mod_m1', 'wh_dup_code', 'EQ999004', NULL, ?, ?)",
        )
        .bind(&tenant_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(null_serial_1.is_ok());

        let null_serial_2 = sqlx::query(
            "INSERT INTO equipment_unit (id, tenant_id, equipment_model_id, warehouse_id, patrimony_tag, serial_number, created_at, updated_at)
             VALUES ('u_06', ?, 'mod_m1', 'wh_dup_code', 'EQ999005', NULL, ?, ?)",
        )
        .bind(&tenant_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(
            null_serial_2.is_ok(),
            "Múltiplas unidades com serial NULL devem ser permitidas"
        );
```
<!-- /snippet -->

---

## 4. URLs Brutas (Raw URLs) Validadas com HTTP 200

As URLs raw do commit de entrega com 40 dígitos são validadas pelo script `scripts/print-raw-urls.js`:

- **ADR-0018 (Unidades e Estoque)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/docs/decisions/0018-unidades-e-estoque.md`
- **Migração `0011_inventory.sql`**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/crates/store-sqlite/migrations/0011_inventory.sql`
- **Domínio Core de Estoque (`crates/core/src/domain/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/crates/core/src/domain/inventory.rs`
- **Serviços Core de Estoque (`crates/core/src/services/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/crates/core/src/services/inventory.rs`
- **Persistência SQLite (`crates/store-sqlite/src/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/crates/store-sqlite/src/inventory.rs`
- **Comandos Tauri IPC (`src-tauri/src/commands/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/src-tauri/src/commands/inventory.rs`
- **Componente Svelte 5 de Unidades (`src/lib/components/catalog-model-inventory.svelte`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/src/lib/components/catalog-model-inventory.svelte`
- **Página Global de Estoque (`src/routes/inventory/+page.svelte`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/src/routes/inventory/+page.svelte`
- **Testes de Integração e Segurança IPC (`src-tauri/src/session_lifecycle_tests.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/src-tauri/src/session_lifecycle_tests.rs`
- **Testes E2E Playwright (`e2e/inventory.spec.ts`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/<COMMIT_HASH>/e2e/inventory.spec.ts`

---

## 5. Roteiro de Teste Manual e Teste de Fumaça

> [!WARNING]
> **INSTRUÇÃO OBRIGATÓRIA (R5)**:
> Para os testes manuais e de fumaça, utilize **EXCLUSIVAMENTE** o instalador de produção (`EventRent_0.0.1_x64-setup.exe` ou pacote `.msi`).
> **NUNCA** execute diretamente o binário cru em `target\release\eventrent-app.exe`. Esse executável é gerado em modo de depuração/desenvolvimento e tenta conectar a um servidor Vite local (`localhost`), resultando na mensagem `"localhost refused to connect"` observada na tela de verificação.

1. **Acesso à Gestão de Depósitos**:
   - Abra o aplicativo instalado via instalador de produção e faça login como Proprietário.
   - No menu lateral esquerdo, clique no item **Estoque & Depósitos**.
   - Na aba **Depósitos**, confirme que o `"Depósito principal"` (código `PRINCIPAL`) está listado com o selo **Padrão**.
   - Clique em **+ Novo Depósito**, preencha o nome `"Depósito Secundário"` e o código `"DEP-02"`. Clique em Salvar e verifique que aparece na tabela.

2. **Tombamento de Unidade Física Serializada**:
   - No menu lateral, acesse **Equipamentos**.
   - Abra a ficha técnica de um modelo com modo serializado (ou crie um novo modelo, ex: `"Pioneer CDJ-3000"`).
   - Clique na 4ª aba: **Unidades & Estoque**.
   - Clique no botão **+ Nova Unidade Física**.
   - Informe o número de série do fabricante (ex: `"PIONEER-SN-1234"`), condição física `"Novo"` e observações opcionais.
   - Clique em **Cadastrar e Gerar Patrimônio**.
   - Verifique que a tabela da aba atualiza imediatamente exibindo a nova unidade com código **`EQ000001`**, status **Disponível** e o depósito padrão vinculado.

3. **Conferência na Tela Global de Estoque**:
   - Retorne ao menu lateral **Estoque & Depósitos**.
   - Clique na aba **Unidades Físicas** e utilize o campo de busca digitando `"EQ000001"`. Confirme que a unidade recém-tombada é localizada com o modelo, status e depósito corretos.
   - Clique na aba **Livro-Razão & Movimentações**.
   - Clique no botão **+ Registrar Movimentação**. Lance uma movimentação (ex: Entrada por Compra ou Transferência), informe a justificativa e confirme.
   - Verifique que o histórico exibe o registro com data, tipo de movimentação, modelo, quantidade e justificativa.
