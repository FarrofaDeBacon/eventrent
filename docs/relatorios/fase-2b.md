# Relatório de Conclusão — Fase 2B: Gestão de Unidades Físicas e Estoque Multidepósito

**Branch**: `fase-2b-estoque`  
**Pull Request**: [#14](https://github.com/FarrofaDeBacon/eventrent/pull/14) (Rascunho / Draft mantido para conferência do usuário)  
**Status**: Concluído — 100% dos testes e verificações aprovados  

---

## 1. Resumo Executivo

A **Fase 2B** entregou a arquitetura e implementação completa de **Unidades Físicas, Estoque Multidepósito e Livro-Razão Imutável** para o modo local do **EventRent**, cumprindo integralmente as decisões arquiteturais do **ADR-0018** (`docs/decisions/0018-unidades-e-estoque.md`), preservando congeladas as migrações existentes `0001` a `0010` e mantendo conformidade estrita com o design system semântico OKLCH (0 cores hardcoded) e o portão de qualidade (`pnpm verify`, 14 passos).

### Principais Entregas:

1. **Tarefa 0 (Robustez e Achados da Fase 2N)**:
   - Validação canônica de arquivos de mídia (`media/<tenant>/<2 hex>/<64 hex>`) filtrando arquivos espúrios (`Thumbs.db`, `.DS_Store`, etc.) sem quebrar o backup;
   - Preservação estrita de mídias preexistentes no rollback de restaurações parciais;
   - Separação entre `last_attempt_at` e `last_success_at`, garantindo que o alerta de "mais de 7 dias sem backup" dispare mesmo quando ocorrem falhas diárias consecutivas;
   - Validação estrita de limites numéricos de retenção e proteção incondicional da cópia mais recente;
   - Conexão SQLite padronizada via `SqliteConnectOptions::new().filename(...)` protegida contra espaços e acentos no path;
   - Salvaguarda pré-restauração completa com `-wal` e `-shm`, verificação prévia de espaço em disco e rotina de agendamento em background;
   - Teste de integração de ponta a ponta (`test_full_end_to_end_backup_mutation_and_restore_cycle`).

2. **Tarefa 1 (ADR-0018)**:
   - Registro arquitetural formal em `docs/decisions/0018-unidades-e-estoque.md` documentando o modelo híbrido de controle (`serialized` vs `quantity`), numeração sequencial atômica `EQ000001`, livro-razão append-only, saldo em mãos não-negativo, multidepósito com galpão padrão e confidencialidade financeira.

3. **Tarefa 2.1 (Migração `0011_inventory.sql`)**:
   - Criação das tabelas `warehouse`, `equipment_patrimony_seq`, `equipment_unit`, `stock_balance`, `stock_movement` e `equipment_unit_media`;
   - Triggers SQLite protegendo a imutabilidade do livro-razão (`trg_stock_movement_no_update`, `trg_stock_movement_no_delete`) e saldo não-negativo (`trg_stock_balance_no_negative`);
   - Provisionamento automático do "Galpão Principal" e inicializador de sequência via triggers.

4. **Tarefa 2.2 (Domínio Core & Repositórios Store-SQLite)**:
   - Entidades e enums em `crates/core/src/domain/inventory.rs`;
   - Serviços de aplicação em `crates/core/src/services/inventory.rs` com RBAC e método `sanitize_model_financials`;
   - Persistência e consultas em `crates/store-sqlite/src/inventory.rs`, com 34 testes SQLite e 28 testes Core aprovados.

5. **Tarefa 2.3 (Comandos Tauri IPC, Schemas Zod & Repositórios Frontend)**:
   - 17 comandos IPC registrados canonicamente em `COMMAND_REGISTRY` e `generate_handler!` (totalizando 87 comandos no aplicativo com 87 fixtures contratuais);
   - Schemas Zod em `src/lib/data/schemas/inventory.ts`;
   - Implementações `tauri-inventory-repository.ts` e `mock-inventory-repository.ts` com suíte de 157 testes no Vitest aprovada.

6. **Tarefa 2.4 (Interface Svelte 5 de Unidades e Estoque)**:
   - Componente `src/lib/components/catalog-model-inventory.svelte` integrado à 4ª aba da ficha técnica de equipamentos (`/catalog/[id]`);
   - Página global `/inventory` com 3 abas acessíveis: Depósitos, Unidades Físicas e Livro-Razão & Movimentações;
   - Conformidade total com tokens semânticos OKLCH (0 cores literais).

7. **Tarefa 2.5 (Testes dos 4 Níveis e E2E Playwright)**:
   - Teste de integração de segurança no Rust `test_inventory_permissions_enforced_at_core_level_and_financial_masking`;
   - Suíte Playwright completa em `e2e/inventory.spec.ts` cobrindo depósitos, tombamento serializado gerando `EQ000001` e lançamento no livro-razão.

8. **Tarefa 2.6 (Documentação & Relatório)**:
   - Atualização de `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md`, `docs/seguranca.md`, `docs/05-roadmap.md` e este relatório.

---

## 2. Rastreabilidade Estrutural de Código (Snippets Verificados)

Todos os trechos de código abaixo foram validados dinamicamente pelo script `scripts/check-report-snippets.js` no commit de entrega.

### 2.1. Tabela de Depósitos e Isolamento Multi-tenant (Migração 0011)

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/migrations/0011_inventory.sql:1-35@85b84e3 -->
```sql
-- Migração 0011: Unidades Físicas, Estoque Multidepósito e Rastreabilidade (Fase 2B / ADR-0018)
-- Regras:
-- - SQL portável (SQLite + PostgreSQL)
-- - Isolamento multi-tenant com chaves estrangeiras compostas (tenant_id, *_id)
-- - Índices únicos parciais com WHERE deleted_at IS NULL
-- - Livro-razão imutável append-only (stock_movement) protegido por triggers
-- - Integridade de saldo de estoque não-negativo em stock_balance

-- 1. Depósitos (warehouse)
CREATE TABLE warehouse (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    address_line1 TEXT,
    address_line2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE UNIQUE INDEX idx_warehouse_tenant_code_active
    ON warehouse (tenant_id, code)
    WHERE deleted_at IS NULL;

```
<!-- /snippet -->

### 2.2. Triggers de Imutabilidade Append-Only no Livro-Razão e Tabela de Mídias de Unidade

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/migrations/0011_inventory.sql:147-175@85b84e3 -->
```sql
-- Triggers de Imutabilidade (Append-Only) para stock_movement
CREATE TRIGGER trg_stock_movement_no_update
BEFORE UPDATE ON stock_movement
BEGIN
    SELECT RAISE(ABORT, 'Operação proibida: stock_movement é imutável (append-only)');
END;

CREATE TRIGGER trg_stock_movement_no_delete
BEFORE DELETE ON stock_movement
BEGIN
    SELECT RAISE(ABORT, 'Operação proibida: registros de stock_movement não podem ser excluídos');
END;

-- 6. Mídias Vinculadas a Unidades Físicas (equipment_unit_media)
CREATE TABLE equipment_unit_media (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    equipment_unit_id TEXT NOT NULL,
    blob_id TEXT NOT NULL,
    media_type TEXT NOT NULL CHECK (media_type IN ('damage_photo', 'inspection_photo', 'serial_photo', 'invoice_doc')),
    caption TEXT,
    created_at TEXT NOT NULL,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, equipment_unit_id) REFERENCES equipment_unit(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, blob_id) REFERENCES media_blob(tenant_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);
```
<!-- /snippet -->

### 2.3. Enums e Status do Domínio de Estoque no Core

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/domain/inventory.rs:14-41@85b84e3 -->
```rust
pub enum UnitStatus {
    Available,
    Allocated,
    InTransit,
    Maintenance,
    Damaged,
    Lost,
    Retired,
}

impl UnitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Allocated => "allocated",
            Self::InTransit => "in_transit",
            Self::Maintenance => "maintenance",
            Self::Damaged => "damaged",
            Self::Lost => "lost",
            Self::Retired => "retired",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "available" => Ok(Self::Available),
            "allocated" => Ok(Self::Allocated),
            "in_transit" => Ok(Self::InTransit),
```
<!-- /snippet -->

### 2.4. Sanitização Dinâmica de Valores Financeiros no Core para Perfis Operacionais

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/services/inventory.rs:705-717@85b84e3 -->
```rust
    // ==========================================

    pub fn sanitize_model_financials(
        &self,
        session: &SessionInfo,
        model: &mut crate::domain::catalog::EquipmentModel,
    ) {
        if !session.has_permission(PERM_FINANCE_READ) {
            model.cost_price_cents = None;
            model.replacement_value_cents = None;
        }
    }
}
```
<!-- /snippet -->

### 2.5. Emissão Sequencial Atômica de Patrimônio no SQLite

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/src/inventory.rs:410-425@85b84e3 -->
```rust
    async fn next_patrimony_tag(&self, tenant_id: &str) -> Result<String> {
        let query = sqlx::query(
            r#"
            INSERT INTO equipment_patrimony_seq (tenant_id, last_number)
            VALUES (?, 1)
            ON CONFLICT(tenant_id) DO UPDATE SET last_number = equipment_patrimony_seq.last_number + 1
            RETURNING last_number
            "#,
        )
        .bind(tenant_id);

        let row = self.fetch_one(query).await?;
        let last_num: i64 = row.get("last_number");
        Ok(format_patrimony_tag(last_num))
    }

```
<!-- /snippet -->

### 2.6. Teste de Integração IPC e Core: Validação de Sigilo Financeiro e Permissões

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/session_lifecycle_tests.rs:1145-1175@85b84e3 -->
```rust
    // Consulta pelo Proprietário -> financeiro presente
    let owner_view = app_state
        .services
        .get_equipment_model(&owner_session, &model.model.id)
        .await
        .unwrap();
    assert_eq!(owner_view.model.cost_price_cents, Some(85000));
    assert_eq!(owner_view.model.replacement_value_cents, Some(120000));

    // Consulta pelo Técnico -> financeiro sanitizado (None)
    let tech_view = app_state
        .services
        .get_equipment_model(&tech_session, &model.model.id)
        .await
        .unwrap();
    assert_eq!(
        tech_view.model.cost_price_cents, None,
        "Preço de custo deve ser mascarado para técnico sem finance:read"
    );
    assert_eq!(
        tech_view.model.replacement_value_cents, None,
        "Valor de reposição deve ser mascarado para técnico sem finance:read"
    );

    // 6. Técnico consegue tombar unidade física para este modelo
    let unit = app_state
        .services
        .create_equipment_unit(
            &tech_session,
            &eventrent_core::domain::inventory::CreateEquipmentUnitInput {
                equipment_model_id: model.model.id.clone(),
```
<!-- /snippet -->

### 2.7. Interface Svelte 5: Ações de Tombamento e Movimentação por Modo de Controle

<!-- prettier-ignore -->
<!-- snippet: src/lib/components/catalog-model-inventory.svelte:315-345@85b84e3 -->
```svelte
        {#if controlMode === "serialized"}
          <Button
            size="sm"
            onclick={() => {
              createUnitModalOpen = true;
            }}
            data-testid="btn-add-unit"
          >
            <Plus class="mr-1 h-3.5 w-3.5" />
            Nova Unidade Física
          </Button>
        {:else}
          <Button
            size="sm"
            onclick={() => {
              movementModalOpen = true;
            }}
            data-testid="btn-record-movement"
          >
            <ArrowRightLeft class="mr-1 h-3.5 w-3.5" />
            Lançar Movimentação
          </Button>
        {/if}
      {/if}
    </div>
  </div>

  {#if loading}
    <div
      class="rounded-xl border border-border bg-card p-10 text-center text-sm text-muted-foreground"
    >
```
<!-- /snippet -->

### 2.8. Teste E2E Playwright: Tombamento de Unidade e Geração de Patrimônio

<!-- prettier-ignore -->
<!-- snippet: e2e/inventory.spec.ts:105-130@85b84e3 -->
```typescript
    // 6. Navegar para a aba "Unidades & Estoque" na ficha do modelo
    const inventoryTab = page.getByTestId("tab-model-inventory");
    await expect(inventoryTab).toBeVisible();
    await inventoryTab.click();

    // 7. Tombar a primeira unidade física (deve gerar EQ000001)
    const newUnitBtn = page.getByTestId("btn-add-unit");
    await expect(newUnitBtn).toBeVisible();
    await newUnitBtn.click();

    await page.locator("#new-unit-serial").fill("PIONEER-SN-8822");
    await page
      .locator("#new-unit-notes")
      .fill("Equipamento recebido novo na caixa lacrada.");
    await page
      .getByRole("button", { name: "Cadastrar e Gerar Patrimônio" })
      .click();

    await expect(page.getByText("EQ000001")).toBeVisible();
    await expect(page.getByText("PIONEER-SN-8822")).toBeVisible();
    await expect(page.getByText("Disponível")).toBeVisible();

    // 8. Voltar para a rota global /inventory
    await inventoryNav.click();
    await expect(page).toHaveURL(/\/inventory$/);

```
<!-- /snippet -->

---

## 3. URLs Brutas (Raw URLs) Validadas com HTTP 200

Todas as URLs abaixo foram validadas pelo script `scripts/print-raw-urls.js` com o hash de 40 dígitos do commit `85b84e323fe8f9833306e4d9d8b50f6b7d606872`:

- **ADR-0018 (Unidades e Estoque)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/docs/decisions/0018-unidades-e-estoque.md`
- **Migração `0011_inventory.sql`**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/crates/store-sqlite/migrations/0011_inventory.sql`
- **Domínio Core de Estoque (`crates/core/src/domain/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/crates/core/src/domain/inventory.rs`
- **Serviços Core de Estoque (`crates/core/src/services/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/crates/core/src/services/inventory.rs`
- **Persistência SQLite (`crates/store-sqlite/src/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/crates/store-sqlite/src/inventory.rs`
- **Comandos Tauri IPC (`src-tauri/src/commands/inventory.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/src-tauri/src/commands/inventory.rs`
- **Componente Svelte 5 de Unidades (`src/lib/components/catalog-model-inventory.svelte`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/src/lib/components/catalog-model-inventory.svelte`
- **Página Global de Estoque (`src/routes/inventory/+page.svelte`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/src/routes/inventory/+page.svelte`
- **Testes de Integração e Segurança IPC (`src-tauri/src/session_lifecycle_tests.rs`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/src-tauri/src/session_lifecycle_tests.rs`
- **Testes E2E Playwright (`e2e/inventory.spec.ts`)**:  
  `https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/85b84e323fe8f9833306e4d9d8b50f6b7d606872/e2e/inventory.spec.ts`

---

## 4. Roteiro de Teste Manual e Teste de Fumaça

Este roteiro permite validar as principais funcionalidades operacionais da Fase 2B na interface do usuário:

1. **Acesso à Gestão de Depósitos**:
   - Faça login como Proprietário.
   - No menu lateral esquerdo, clique no novo item **Estoque & Depósitos**.
   - Na aba **Depósitos**, confirme que o `"Galpão Principal"` (código `PRINCIPAL`) está listado com a badge **Padrão**.
   - Clique em **+ Novo Depósito**, preencha o nome `"Galpão Secundário"` e o código `"GALP-02"`. Clique em Salvar e verifique que aparece na tabela.

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
