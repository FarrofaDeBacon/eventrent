# Relatório da Fase 2A — Tarefa 0, Requisitos Consolidados e Catálogo de Equipamentos

**Branch:** `fase-2a-catalogo`  
**Base:** `main` (`05d7c87` / `v0.7.0-fase1d`)  
**Status:** Concluída, verificada (`pnpm verify` 10/10 + `pnpm test:e2e` 6/6) e pronta para revisão via Pull Request.

---

## 1. Resumo Executivo

### Tarefa 0 — Ajustes e Pré-requisitos

- **0.1 Configurações de Alocação (sem migração):** Substituídas as 4 chaves anteriores pelas 3 chaves definitivas (`allocation.changeover_minutes = 30`, `allocation.internal_transfer_minutes = 20`, `allocation.external_transfer_minutes = 60`), removendo `#[serde(default)]`, atualizando `CoreServices`, `/settings/operation`, `venue-form.svelte`, `mock-settings-repository.ts`, Vitest, E2E e ADRs.
- **0.2 Limite de Busca em Locais:** Compartilhado `validate_search_query_limits` (máx. 100 caracteres, máx. 8 termos) entre `list_customers` e `list_venues`.
- **0.3 Anti-Vazamento e RBAC em Terceiros:** Extraídos `can_read_party` e `duplicate_document_conflict_error`; aplicado anti-vazamento na atualização de documento duplicado e retorno de `CoreError::NotFound` quando o usuário não possui permissão de leitura sobre o registro em `update_customer`, `set_customer_active`, `delete_customer` e `add_customer_role`.
- **0.4 Adendo ao ADR-0008:** Documentados os pré-requisitos de portabilidade e segurança para PostgreSQL na Fase 2.5 (`UPDATE ... WHERE updated_at = $expected` / `SELECT ... FOR UPDATE`, placeholders `$n`, tipos `BOOLEAN` / `TIMESTAMPTZ`, estratégia de travas na Fase 3, rate limiting anti-DoS de lockout e sessões estritamente server-side).

### Tarefa 1 — Documento Consolidado de Requisitos

- Criado [`docs/06-requisitos-de-produto.md`](../06-requisitos-de-produto.md) cobrindo Fases 2 a 4.5 e referenciado como fonte obrigatória na Regra 15 do [`AGENTS.md`](../../AGENTS.md).

### Tarefa 2 — Catálogo de Equipamentos

- **2.1 Migração `0008_catalog.sql`:** Tabelas `brand`, `equipment_category`, `attribute_definition`, `equipment_model` e `equipment_model_attribute_value` com chaves estrangeiras compostas `(tenant_id, *_id)`, índices únicos parciais `WHERE deleted_at IS NULL`, valores monetários em centavos inteiros (`>= 0`) e zero dados de negócio na migração.
- **2.2 Core + SQLite + Tauri:** Domínio completo (`crates/core/src/domain/catalog.rs`), `CatalogStore` (`crates/core/src/store/mod.rs`), serviços transacionais com auditoria (`crates/core/src/services/catalog.rs`), adapter SQLite (`crates/store-sqlite/src/catalog.rs`) e 19 comandos Tauri (`src-tauri/src/commands/catalog.rs`).
- **2.3 Permissões RBAC:** Confirmada matriz `catalog:read` (Proprietário, Estoque, Comercial, Técnico) e `catalog:manage` (Proprietário, Estoque; Financeiro sem acesso).
- **2.4 Carga Sugerida Idempotente:** `load_suggested_catalog_categories` cria árvore audiovisual inicial e campos técnicos sem duplicar em execuções repetidas.
- **2.5 Frontend:** Repositórios (`catalog-repository.ts`, `tauri-catalog-repository.ts`, `mock-catalog-repository.ts`), sidebar (`Equipamentos`), `Ctrl+K` e páginas `/catalog` (lista virtualizada + filtros por subárvore/marca/controle/status), `/catalog/new`, `/catalog/[id]` (somente leitura para Técnico/Comercial) e `/catalog/categories` (editor de árvore, atributos e carga sugerida).
- **2.6 Testes:** Unitários Rust (EAN/NCM, ciclos, profundidade 4, herança), SQLite contract/isolation/RBAC, Vitest (`catalog-repository.spec.ts`) e Playwright E2E (`e2e/catalog.spec.ts`).
- **2.7 Documentação:** Atualizados `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md` e `docs/05-roadmap.md`.

---

## 2. Trechos Exatos do Código Real

### 2.1 `update_allocation_settings` (`crates/core/src/services/mod.rs`)

```rust
    pub async fn update_allocation_settings(
        &self,
        session: &SessionInfo,
        input: &UpdateAllocationSettingsInput,
    ) -> Result<AllocationSettings> {
        self.check_session_and_permission(session, PERM_SETTINGS_MANAGE)?;
        crate::domain::validate_allocation_settings(input)?;

        let session_clone = session.clone();
        let input_clone = input.clone();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let input = input_clone.clone();
            let now = now.clone();
            Box::pin(async move {
                let pairs = [
                    (
                        SETTING_CHANGEOVER_MINUTES,
                        input.changeover_minutes.to_string(),
                    ),
                    (
                        SETTING_INTERNAL_TRANSFER_MINUTES,
                        input.internal_transfer_minutes.to_string(),
                    ),
                    (
                        SETTING_EXTERNAL_TRANSFER_MINUTES,
                        input.external_transfer_minutes.to_string(),
                    ),
                ];

                for (key, val) in pairs {
                    stores
                        .settings()
                        .upsert(&TenantSetting {
                            id: Uuid::now_v7().to_string(),
                            tenant_id: session.tenant_id.clone(),
                            setting_key: key.to_string(),
                            setting_value: val,
                            updated_at: now.clone(),
                            updated_by: Some(session.user_id.clone()),
                        })
                        .await?;
                }

                let detail = format!(
                    "Configurações de alocação atualizadas: virada={}min, deslocamento_interno={}min, deslocamento_externo={}min",
                    input.changeover_minutes,
                    input.internal_transfer_minutes,
                    input.external_transfer_minutes
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "settings.allocation_updated",
                        "tenant_setting",
                        &session.tenant_id,
                        &detail,
                    ))
                    .await?;

                Ok(AllocationSettings {
                    changeover_minutes: input.changeover_minutes,
                    internal_transfer_minutes: input.internal_transfer_minutes,
                    external_transfer_minutes: input.external_transfer_minutes,
                })
            })
        })
        .await
    }
```

### 2.2 Migração `0008_catalog.sql` (Destaques de FK Composta e Índices Únicos Parciais)

```sql
CREATE TABLE equipment_category (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    parent_id TEXT,
    name TEXT NOT NULL,
    name_normalized TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, parent_id) REFERENCES equipment_category(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_equipment_category_name_active
    ON equipment_category (tenant_id, COALESCE(parent_id, ''), name_normalized)
    WHERE deleted_at IS NULL;
```

```sql
CREATE UNIQUE INDEX idx_equipment_model_sku_active
    ON equipment_model (tenant_id, sku_normalized)
    WHERE deleted_at IS NULL AND sku_normalized IS NOT NULL;

CREATE UNIQUE INDEX idx_equipment_model_brand_code_active
    ON equipment_model (tenant_id, brand_id, model_code_normalized)
    WHERE deleted_at IS NULL AND brand_id IS NOT NULL AND model_code_normalized IS NOT NULL;
```

---

## 3. Verificações Executadas

- `pnpm verify` (10/10):
  - `svelte-check`: 0 erros, 0 avisos
  - `check-no-hardcoded-colors.js`, `prettier --check`, `eslint`: 0 violações em 41 componentes `.svelte`
  - `vitest run`: 8 arquivos, 42 testes aprovados
  - Prova estrutural de congelamento de migrações (`0001`..`0007` intactas): 6/6 cenários aprovados
  - `cargo fmt --check` e `cargo clippy` (workspace + 3 crates sem unificação): 0 avisos
  - `cargo test --workspace`: `eventrent-app-lib` (9/9), `eventrent-core` (19/19), `eventrent-store-sqlite` (29/29)
- `pnpm test:e2e`: 6/6 suítes Playwright aprovadas (`smoke`, `branding`, `customers`, `suppliers`, `venues`, `catalog`).
- Nenhum item do escopo da Fase 2A foi simplificado ou omitido.
