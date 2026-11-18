# Relatório de Entrega — Fase 1D: Tarefa 0 + Locais de Evento, Espaços, Contatos e Configurações de Alocação

- **Branch:** `fase-1d-locais`
- **Commit de referência (código e docs):** `ccb2500122949df2ddde81bdccc175f54dc61b1c`
- **Status:** Pronto para revisão / PR para `main` (sem merge e sem tag, conforme regra do projeto)

---

## 1. Resumo das Entregas

### Tarefa 0 — Pendências da Fase 1C

- **0.1 `add_customer_role` & Anti-vazamento:** Operação `add_customer_role(session, id, role)` (`CustomerRole::Customer | Supplier`) exigindo apenas `manage` do papel adicionado, erro anti-vazamento em documento duplicado quando o usuário não tem leitura do registro existente e botões contextuais na UI (`Abrir cadastro` / `Adicionar papel`). Coberto por `contract_supplier_rbac_and_dual_roles` em `crates/core/src/testing.rs`.
- **0.2 Auditoria exata por papel:** `set_customer_active` e `delete_customer` em `crates/core/src/services/mod.rs` registram `"Cliente"`, `"Fornecedor"` ou `"Cliente/Fornecedor"` conforme os papéis ativos.
- **0.3 Seletor de DDI + E.164:** Componente reutilizável `src/lib/components/phone-input.svelte` com seletor de DDI (padrão `+55`) e helpers `splitPhoneDdi`/`composePhoneE164` em `src/lib/utils/document.ts`.
- **0.4 Política de migrações sem reconstrução:** Regra explícita em `AGENTS.md` e adendo na `ADR-0011` proibindo reconstruções `DROP`/`CREATE` sem ADR específico e teste automatizado de preservação de dados.

### Fase 1D — Locais de Evento, Espaços e Operação (ADR-0012)

- **1.1 Migração `0007_venues_and_settings.sql`:** Tabelas `venue`, `venue_space`, `venue_contact` e `tenant_setting`, com chaves estrangeiras compostas `(tenant_id, venue_id)`, índices únicos parciais `WHERE deleted_at IS NULL`, `CHECK (internal_transfer_minutes >= 0 AND internal_transfer_minutes <= 240)` e zero inserts iniciais na migração.
- **1.2 Permissões RBAC (`venue:read`, `venue:manage`):** Adicionadas em `crates/core/src/domain/permission.rs`, `src/lib/data/fixtures/system-roles.json`, `src/lib/data/types.ts` e `src/lib/data/mock-auth-repository.ts`.
- **1.3 Backend Core + SQLite + Tauri de Locais:** Domínio `crates/core/src/domain/venue.rs` (limites de tamanho, validação de caracteres de controle, `Debug` mascarando `email`/`phone` em `VenueContact`, normalização de busca), `VenueStore` em `crates/core/src/store/mod.rs`, serviços transacionais em `crates/core/src/services/mod.rs` (preservando IDs estáveis de espaços em `update_venue` + concorrência otimista `expected_updated_at`), `SqliteStore` e comandos Tauri em `src-tauri/src/commands/venues.rs`.
- **1.4 Configurações de Alocação (`Configurações > Operação`):** `TenantSettingStore`, `get_allocation_settings` (defaults idempotentes `30`, `20`, `60`), `update_allocation_settings` (`0..=240` min + auditoria), comandos Tauri em `src-tauri/src/commands/settings.rs` e tela `/settings/operation`.
- **1.5 Frontend de Locais:** Lista virtualizada `/venues` com busca por nome/cidade/espaço/contato, `/venues/new`, `/venues/[id]` (somente leitura para `venue:read` sem `venue:manage`), componente `src/lib/components/venue-form.svelte` com abas (Dados do Local, Espaços com ficha técnica recolhível, Contatos com `<PhoneInput>`), aviso não bloqueante de duplicidade `nome + cidade`, sidebar e paleta `Ctrl+K`.
- **1.6 Testes:** Core (`test_venue_domain_validations_and_limits`), SQLite (`test_store_venues_fk_unique_spaces_contacts_and_isolation`, `test_store_allocation_settings_idempotent_and_rbac`), Vitest (`src/lib/data/venue-repository.spec.ts`) e Playwright E2E (`e2e/venues.spec.ts`).
- **1.7 Documentação:** Atualização de `docs/02-modelo-dados.md`, `docs/03-regras-negocio.md` e adendo da `docs/decisions/0012-modelo-de-alocacao.md`.

---

## 2. URLs Raw Fixadas no Hash (`ccb2500122949df2ddde81bdccc175f54dc61b1c`)

- Migração `0007`: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/store-sqlite/migrations/0007_venues_and_settings.sql
- Permissões (`permission.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/core/src/domain/permission.rs
- Domínio de Locais (`venue.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/core/src/domain/venue.rs
- Domínio de Configurações (`settings.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/core/src/domain/settings.rs
- Store Traits (`store/mod.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/core/src/store/mod.rs
- Serviços (`services/mod.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/core/src/services/mod.rs
- Store SQLite (`store-sqlite/src/lib.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/crates/store-sqlite/src/lib.rs
- Comandos Tauri (`venues.rs` / `settings.rs`):
  - https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/src-tauri/src/commands/venues.rs
  - https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/src-tauri/src/commands/settings.rs
- E2E Playwright (`e2e/venues.spec.ts`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/ccb2500122949df2ddde81bdccc175f54dc61b1c/e2e/venues.spec.ts

---

## 3. Trechos Exatos do Código

### 3.1 `SYSTEM_ROLES` em `crates/core/src/domain/permission.rs`

```rust
pub const SYSTEM_ROLES: &[SystemRoleSpec] = &[
    SystemRoleSpec {
        name: "Proprietário",
        slug: ROLE_SLUG_OWNER,
        description:
            "Acesso total à administração, configurações, usuários, financeiro e operação.",
        permissions: ALL_PERMISSIONS,
    },
    SystemRoleSpec {
        name: "Comercial",
        slug: ROLE_SLUG_COMMERCIAL,
        description: "Gestão de clientes, orçamentos, reservas e consulta ao catálogo.",
        permissions: &[
            PERM_CUSTOMER_READ,
            PERM_CUSTOMER_MANAGE,
            PERM_SUPPLIER_READ,
            PERM_VENUE_READ,
            PERM_VENUE_MANAGE,
            PERM_CATALOG_READ,
            PERM_RESERVATION_READ,
            PERM_RESERVATION_MANAGE,
            PERM_QUOTE_READ,
            PERM_QUOTE_MANAGE,
            PERM_BRANDING_READ,
        ],
    },
    SystemRoleSpec {
        name: "Estoque",
        slug: ROLE_SLUG_WAREHOUSE,
        description: "Gestão do catálogo de equipamentos, patrimônio, separação e conferência.",
        permissions: &[
            PERM_SUPPLIER_READ,
            PERM_SUPPLIER_MANAGE,
            PERM_VENUE_READ,
            PERM_VENUE_MANAGE,
            PERM_CATALOG_READ,
            PERM_CATALOG_MANAGE,
            PERM_RESERVATION_READ,
            PERM_WAREHOUSE_READ,
            PERM_WAREHOUSE_OPERATE,
            PERM_BRANDING_READ,
        ],
    },
    SystemRoleSpec {
        name: "Técnico",
        slug: ROLE_SLUG_TECHNICIAN,
        description:
            "Operação de conferência de carga, leitura QR, manutenção e consulta de eventos.",
        permissions: &[
            PERM_VENUE_READ,
            PERM_CATALOG_READ,
            PERM_RESERVATION_READ,
            PERM_WAREHOUSE_READ,
            PERM_WAREHOUSE_OPERATE,
            PERM_BRANDING_READ,
        ],
    },
    SystemRoleSpec {
        name: "Financeiro",
        slug: ROLE_SLUG_FINANCE,
        description: "Gestão de faturamento, contas a pagar/receber, contratos e auditoria.",
        permissions: &[
            PERM_CUSTOMER_READ,
            PERM_SUPPLIER_READ,
            PERM_SUPPLIER_MANAGE,
            PERM_VENUE_READ,
            PERM_QUOTE_READ,
            PERM_RESERVATION_READ,
            PERM_FINANCE_READ,
            PERM_FINANCE_MANAGE,
            PERM_AUDIT_READ,
            PERM_BRANDING_READ,
        ],
    },
];
```

### 3.2 `venue_space` e `tenant_setting` em `crates/store-sqlite/migrations/0007_venues_and_settings.sql`

```sql
CREATE TABLE venue_space (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_normalized TEXT NOT NULL,
    capacity_people INTEGER,
    area_sqm REAL,
    ceiling_height_m REAL,
    power_available TEXT,
    loading_access TEXT,
    rigging_points TEXT,
    acoustics_notes TEXT,
    internal_transfer_minutes INTEGER NOT NULL DEFAULT 0 CHECK (internal_transfer_minutes >= 0 AND internal_transfer_minutes <= 240),
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, venue_id) REFERENCES venue(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE INDEX idx_venue_space_tenant_venue
    ON venue_space (tenant_id, venue_id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_venue_space_name_active
    ON venue_space (tenant_id, venue_id, name_normalized)
    WHERE deleted_at IS NULL;
```

```sql
CREATE TABLE tenant_setting (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    setting_key TEXT NOT NULL,
    setting_value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (updated_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, setting_key)
);
```

---

## 4. Verificações Executadas

- `pnpm verify` (7/7 etapas):
  - `svelte-check`: 0 erros, 0 avisos
  - Lint de código & cores: 0 cores literais em 36 componentes `.svelte`, Prettier e ESLint 100% conformes
  - Vitest: 7 arquivos, 39 testes passando (incluindo `src/lib/data/venue-repository.spec.ts`)
  - Prova de congelamento de migrações: 6/6 cenários passando
  - `cargo fmt --check` & `cargo clippy --workspace -- -D warnings`: 0 avisos/erros
  - Rust workspace tests: 9 (`eventrent_app_lib`) + 18 (`eventrent_core`) + 28 (`eventrent_store_sqlite`) = **55/55 passando** (incluindo `contract_supplier_rbac_and_dual_roles`, `test_venue_domain_validations_and_limits`, `test_store_venues_fk_unique_spaces_contacts_and_isolation` e `test_store_allocation_settings_idempotent_and_rbac`)
- `pnpm test:e2e` (Playwright): **5/5 passando** (`smoke.spec.ts`, `branding.spec.ts`, `customers.spec.ts`, `suppliers.spec.ts`, `venues.spec.ts`).
