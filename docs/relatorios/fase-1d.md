# Relatório de Entrega — Fase 1D + Revisão 1D-rev: Tarefa 0, Locais de Evento, Espaços, Contatos e Configurações de Alocação

- **Branch:** `fase-1d-locais` (PR #9)
- **Commit de referência do código:** `9779bc5aee753e167360dc835bb97f641e7dc2d4`
- **Status:** Pronto para revisão / merge na `main` (sem merge e sem tag, conforme regra do projeto)

---

## 1. Correções da Rodada 1D-rev

- **R1 (`internal_transfer_minutes` no Local + padrão na empresa):** Movido de `venue_space` para `venue` como `INTEGER NULL CHECK (internal_transfer_minutes IS NULL OR (internal_transfer_minutes >= 0 AND internal_transfer_minutes <= 240))` (`NULL` = usar o padrão da empresa `allocation.default_internal_transfer_minutes`, padrão `20` min; `0` explícito = zero).
- **R2 (`venue_space` ficha técnica e `CHECK`s):** Adicionadas colunas `setup_hours_notes`, `noise_restrictions_notes` e `load_dimensions_notes` (`TEXT NULL`, até 2000 caracteres validadas no `core` e exibidas na seção recolhível), além de `CHECK (valor IS NULL OR valor > 0)` em `area_sqm` (até `1.000.000`) e `ceiling_height_m` (até `100`).
- **R3 (`tenant_setting` exceção documentada):** Documentado em `AGENTS.md` e `docs/02-modelo-dados.md` que `tenant_setting` não possui `created_at` nem exclusão lógica (`deleted_at`), mantendo todo histórico de alterações no `audit_log` na mesma transação (comprovado em `test_store_allocation_settings_idempotent_and_rbac`).
- **R4 (`get_allocation_settings` somente leitura):** `get_allocation_settings` e o método interno `get_allocation_settings_for_tenant` não gravam nada no banco quando a chave está ausente (retornam defaults em memória); valores inválidos no banco caem para o padrão em memória com `tracing::warn!`.
- **R5 (`scripts/verify.js`):** Adicionado `cargo clippy -p <crate> -- -D warnings` sem unificação de features para `eventrent-core`, `eventrent-store-sqlite` e `eventrent-app`.

---

## 2. URLs Raw Fixadas no Hash (`9779bc5aee753e167360dc835bb97f641e7dc2d4`)

- Migração `0007`: https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/crates/store-sqlite/migrations/0007_venues_and_settings.sql
- Domínio de Locais (`venue.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/crates/core/src/domain/venue.rs
- Domínio de Configurações (`settings.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/crates/core/src/domain/settings.rs
- Serviços (`services/mod.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/crates/core/src/services/mod.rs
- Store Traits (`store/mod.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/crates/core/src/store/mod.rs
- Store SQLite (`store-sqlite/src/lib.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/crates/store-sqlite/src/lib.rs
- Comandos Tauri (`commands/venues.rs`): https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/9779bc5aee753e167360dc835bb97f641e7dc2d4/src-tauri/src/commands/venues.rs

---

## 3. DDL Completo de `venue` e `venue_space` (`0007_venues_and_settings.sql`)

```sql
CREATE TABLE venue (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    street TEXT,
    number TEXT,
    complement TEXT,
    neighborhood TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    country TEXT NOT NULL DEFAULT 'BR',
    parking_notes TEXT,
    loading_dock_notes TEXT,
    access_hours TEXT,
    internal_transfer_minutes INTEGER CHECK (internal_transfer_minutes IS NULL OR (internal_transfer_minutes >= 0 AND internal_transfer_minutes <= 240)),
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);

CREATE INDEX idx_venue_tenant_sort
    ON venue (tenant_id, name_sort, id)
    WHERE deleted_at IS NULL;

CREATE TABLE venue_space (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    venue_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_normalized TEXT NOT NULL,
    capacity_people INTEGER,
    area_sqm REAL CHECK (area_sqm IS NULL OR area_sqm > 0),
    ceiling_height_m REAL CHECK (ceiling_height_m IS NULL OR ceiling_height_m > 0),
    power_available TEXT,
    loading_access TEXT,
    rigging_points TEXT,
    acoustics_notes TEXT,
    setup_hours_notes TEXT,
    noise_restrictions_notes TEXT,
    load_dimensions_notes TEXT,
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

---

## 4. Trechos Exatos do Código (`crates/core/src/services/mod.rs`)

### 4.1 `add_customer_role`

```rust
    /// Ativa um papel (`Customer` ou `Supplier`) em um cadastro de terceiro já existente.
    /// Exige `manage` SOMENTE do papel que está sendo adicionado.
    pub async fn add_customer_role(
        &self,
        session: &SessionInfo,
        id: &str,
        role: CustomerRole,
    ) -> Result<CustomerWithDetails> {
        let required_perm = match role {
            CustomerRole::Customer => PERM_CUSTOMER_MANAGE,
            CustomerRole::Supplier => PERM_SUPPLIER_MANAGE,
        };
        self.check_session_and_permission(session, required_perm)?;

        let session_clone = session.clone();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();

            Box::pin(async move {
                let mut existing = stores
                    .customers()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Cadastro não encontrado.".to_string()))?;

                let already_has_role = match role {
                    CustomerRole::Customer => existing.customer.is_customer,
                    CustomerRole::Supplier => existing.customer.is_supplier,
                };
                if already_has_role {
                    return Ok(existing);
                }

                match role {
                    CustomerRole::Customer => existing.customer.is_customer = true,
                    CustomerRole::Supplier => existing.customer.is_supplier = true,
                }
                existing.customer.updated_at = now;

                stores
                    .customers()
                    .update(
                        &existing.customer,
                        &existing.contacts,
                        &existing.addresses,
                        &existing.supplier_categories,
                    )
                    .await?;

                let masked_audit_name = if existing.customer.person_type == PersonType::Individual {
                    mask_individual_name_for_audit(&existing.customer.name)
                } else {
                    existing.customer.name.clone()
                };
                let added_label = match role {
                    CustomerRole::Customer => "Cliente",
                    CustomerRole::Supplier => "Fornecedor",
                };
                let audit_detail = format!(
                    "Papel '{added_label}' adicionado ao cadastro '{}' (id: {})",
                    masked_audit_name, existing.customer.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "customer.role_added",
                        "customer",
                        &existing.customer.id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(existing)
            })
        })
        .await
    }
```

### 4.2 `update_venue` (preservação de IDs de espaços e `expected_updated_at`)

```rust
    pub async fn update_venue(
        &self,
        session: &SessionInfo,
        input: &UpdateVenueInput,
    ) -> Result<VenueWithDetails> {
        self.check_session_and_permission(session, PERM_VENUE_MANAGE)?;

        if input.expected_updated_at.trim().is_empty() {
            return Err(CoreError::Validation(
                "expected_updated_at é obrigatório para atualização concorrente.".to_string(),
            ));
        }

        let fields = validate_venue_fields(
            &input.name,
            input.street.clone(),
            input.number.clone(),
            input.complement.clone(),
            input.neighborhood.clone(),
            input.city.clone(),
            input.state.clone(),
            input.postal_code.clone(),
            &input.country,
            input.parking_notes.clone(),
            input.loading_dock_notes.clone(),
            input.access_hours.clone(),
            input.internal_transfer_minutes,
            input.notes.clone(),
        )?;

        let mut space_inputs = input.spaces.clone();
        validate_venue_spaces(&mut space_inputs)?;

        let mut contact_inputs = input.contacts.clone();
        validate_venue_contacts(&mut contact_inputs, &fields.country)?;

        let now = Utc::now().to_rfc3339();
        let search_text = build_venue_search_text(&fields, &space_inputs, &contact_inputs);
        let name_sort = build_venue_name_sort(&fields.name);

        let session_clone = session.clone();
        let id_str = input.id.clone();
        let expected_updated_at = input.expected_updated_at.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let expected_updated_at = expected_updated_at.clone();
            let now = now.clone();
            let name = fields.name.clone();
            let street = fields.street.clone();
            let number = fields.number.clone();
            let complement = fields.complement.clone();
            let neighborhood = fields.neighborhood.clone();
            let city = fields.city.clone();
            let state = fields.state.clone();
            let postal_code = fields.postal_code.clone();
            let country = fields.country.clone();
            let parking_notes = fields.parking_notes.clone();
            let loading_dock_notes = fields.loading_dock_notes.clone();
            let access_hours = fields.access_hours.clone();
            let internal_transfer_minutes = fields.internal_transfer_minutes;
            let notes = fields.notes.clone();
            let search_text = search_text.clone();
            let name_sort = name_sort.clone();
            let space_inputs = space_inputs.clone();
            let contact_inputs = contact_inputs.clone();

            Box::pin(async move {
                let existing = stores
                    .venues()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Local de evento não encontrado.".to_string())
                    })?;

                if existing.venue.updated_at != expected_updated_at {
                    return Err(CoreError::Conflict(
                        "O cadastro do local foi alterado por outra pessoa. Recarregue os dados antes de salvar."
                            .to_string(),
                    ));
                }

                let existing_space_map: std::collections::HashMap<String, &VenueSpace> = existing
                    .spaces
                    .iter()
                    .map(|s| (s.id.clone(), s))
                    .collect();
                let mut seen_space_ids = std::collections::HashSet::new();

                let mut spaces = Vec::with_capacity(space_inputs.len());
                for (idx, sp) in space_inputs.into_iter().enumerate() {
                    let (space_id, created_at, created_by) = match sp.id {
                        Some(ref raw_id) if !raw_id.trim().is_empty() => {
                            let sid = raw_id.trim().to_string();
                            if !seen_space_ids.insert(sid.clone()) {
                                return Err(CoreError::Validation(
                                    "IDs de espaço duplicados na atualização.".to_string(),
                                ));
                            }
                            let prev = existing_space_map.get(&sid).ok_or_else(|| {
                                CoreError::Validation(format!(
                                    "O espaço informado (id: {sid}) não pertence a este local."
                                ))
                            })?;
                            (sid, prev.created_at.clone(), prev.created_by.clone())
                        }
                        _ => (
                            Uuid::now_v7().to_string(),
                            now.clone(),
                            Some(session.user_id.clone()),
                        ),
                    };

                    spaces.push(VenueSpace {
                        id: space_id,
                        tenant_id: session.tenant_id.clone(),
                        venue_id: existing.venue.id.clone(),
                        name_normalized: normalize_for_search(&sp.name),
                        name: sp.name,
                        capacity_people: sp.capacity_people,
                        area_sqm: sp.area_sqm,
                        ceiling_height_m: sp.ceiling_height_m,
                        power_available: sp.power_available,
                        loading_access: sp.loading_access,
                        rigging_points: sp.rigging_points,
                        acoustics_notes: sp.acoustics_notes,
                        setup_hours_notes: sp.setup_hours_notes,
                        noise_restrictions_notes: sp.noise_restrictions_notes,
                        load_dimensions_notes: sp.load_dimensions_notes,
                        notes: sp.notes,
                        active: sp.active,
                        sort_order: if sp.sort_order == 0 {
                            idx as i64
                        } else {
                            sp.sort_order
                        },
                        created_at,
                        updated_at: now.clone(),
                        deleted_at: None,
                        created_by,
                    });
                }

                let existing_contact_map: std::collections::HashMap<String, &VenueContact> =
                    existing.contacts.iter().map(|c| (c.id.clone(), c)).collect();
                let mut seen_contact_ids = std::collections::HashSet::new();

                let mut contacts = Vec::with_capacity(contact_inputs.len());
                for ct in contact_inputs {
                    let (contact_id, created_at, created_by) = match ct.id {
                        Some(ref raw_id) if !raw_id.trim().is_empty() => {
                            let cid = raw_id.trim().to_string();
                            if !seen_contact_ids.insert(cid.clone()) {
                                return Err(CoreError::Validation(
                                    "IDs de contato duplicados na atualização.".to_string(),
                                ));
                            }
                            let prev = existing_contact_map.get(&cid).ok_or_else(|| {
                                CoreError::Validation(format!(
                                    "O contato informado (id: {cid}) não pertence a este local."
                                ))
                            })?;
                            (cid, prev.created_at.clone(), prev.created_by.clone())
                        }
                        _ => (
                            Uuid::now_v7().to_string(),
                            now.clone(),
                            Some(session.user_id.clone()),
                        ),
                    };

                    contacts.push(VenueContact {
                        id: contact_id,
                        tenant_id: session.tenant_id.clone(),
                        venue_id: existing.venue.id.clone(),
                        name: ct.name,
                        email: ct.email,
                        phone: ct.phone,
                        role: ct.role,
                        is_primary: ct.is_primary,
                        created_at,
                        updated_at: now.clone(),
                        deleted_at: None,
                        created_by,
                    });
                }

                let updated_venue = Venue {
                    id: existing.venue.id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    name,
                    street,
                    number,
                    complement,
                    neighborhood,
                    city,
                    state,
                    postal_code,
                    country,
                    parking_notes,
                    loading_dock_notes,
                    access_hours,
                    internal_transfer_minutes,
                    notes,
                    active: existing.venue.active,
                    search_text,
                    name_sort,
                    created_at: existing.venue.created_at.clone(),
                    updated_at: now,
                    deleted_at: None,
                    created_by: existing.venue.created_by.clone(),
                };

                stores
                    .venues()
                    .update(&updated_venue, &spaces, &contacts)
                    .await?;

                let detail = format!(
                    "Local de evento '{}' atualizado ({} espaço(s), {} contato(s), id: {})",
                    updated_venue.name,
                    spaces.len(),
                    contacts.len(),
                    updated_venue.id
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "venue.updated",
                        "venue",
                        &updated_venue.id,
                        &detail,
                    ))
                    .await?;

                Ok(VenueWithDetails {
                    venue: updated_venue,
                    spaces,
                    contacts,
                })
            })
        })
        .await
    }
```

### 4.3 `get_allocation_settings`, `get_allocation_settings_for_tenant` e `update_allocation_settings`

```rust
    pub async fn get_allocation_settings(
        &self,
        session: &SessionInfo,
    ) -> Result<AllocationSettings> {
        self.check_session_and_permission(session, PERM_SETTINGS_READ)?;
        self.get_allocation_settings_for_tenant(&session.tenant_id)
            .await
    }

    /// Leitura interna sem sessão (utilizada pelo motor de disponibilidade da Fase 3).
    /// Nunca grava no banco: quando a chave não existe ou tem valor inválido,
    /// devolve o padrão em memória (registrando aviso no log no caso inválido).
    pub async fn get_allocation_settings_for_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<AllocationSettings> {
        let rows = self
            .stores
            .settings
            .get_many(
                tenant_id,
                &[
                    SETTING_DEFAULT_SETUP_MINUTES,
                    SETTING_DEFAULT_TEARDOWN_MINUTES,
                    SETTING_DEFAULT_TURNOVER_BUFFER_MINUTES,
                    SETTING_DEFAULT_INTERNAL_TRANSFER_MINUTES,
                ],
            )
            .await?;

        let mut result = AllocationSettings::default();
        for row in rows {
            let parsed_opt = row
                .setting_value
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|v| (0..=MAX_ALLOCATION_MINUTES).contains(v));

            let Some(parsed) = parsed_opt else {
                tracing::warn!(
                    tenant_id = %tenant_id,
                    setting_key = %row.setting_key,
                    raw_value = %row.setting_value,
                    "Valor inválido em tenant_setting; usando padrão em memória."
                );
                continue;
            };

            match row.setting_key.as_str() {
                SETTING_DEFAULT_SETUP_MINUTES => result.default_setup_minutes = parsed,
                SETTING_DEFAULT_TEARDOWN_MINUTES => result.default_teardown_minutes = parsed,
                SETTING_DEFAULT_TURNOVER_BUFFER_MINUTES => {
                    result.default_turnover_buffer_minutes = parsed
                }
                SETTING_DEFAULT_INTERNAL_TRANSFER_MINUTES => {
                    result.default_internal_transfer_minutes = parsed
                }
                _ => {}
            }
        }
        Ok(result)
    }

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
                        SETTING_DEFAULT_SETUP_MINUTES,
                        input.default_setup_minutes.to_string(),
                    ),
                    (
                        SETTING_DEFAULT_TEARDOWN_MINUTES,
                        input.default_teardown_minutes.to_string(),
                    ),
                    (
                        SETTING_DEFAULT_TURNOVER_BUFFER_MINUTES,
                        input.default_turnover_buffer_minutes.to_string(),
                    ),
                    (
                        SETTING_DEFAULT_INTERNAL_TRANSFER_MINUTES,
                        input.default_internal_transfer_minutes.to_string(),
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
                    "Configurações de alocação atualizadas: montagem={}min, desmontagem={}min, giro={}min, deslocamento={}min",
                    input.default_setup_minutes,
                    input.default_teardown_minutes,
                    input.default_turnover_buffer_minutes,
                    input.default_internal_transfer_minutes
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
                    default_setup_minutes: input.default_setup_minutes,
                    default_teardown_minutes: input.default_teardown_minutes,
                    default_turnover_buffer_minutes: input.default_turnover_buffer_minutes,
                    default_internal_transfer_minutes: input.default_internal_transfer_minutes,
                })
            })
        })
        .await
    }
```

---

## 5. Trecho Alterado em `scripts/verify.js` (Clippy por Crate)

```js
  {
    name: "Clippy Linter (Workspace)",
    command: "cargo",
    args: ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
  },
  {
    name: "Clippy Linter (eventrent-core sem unificação)",
    command: "cargo",
    args: ["clippy", "-p", "eventrent-core", "--", "-D", "warnings"],
  },
  {
    name: "Clippy Linter (eventrent-store-sqlite sem unificação)",
    command: "cargo",
    args: ["clippy", "-p", "eventrent-store-sqlite", "--", "-D", "warnings"],
  },
  {
    name: "Clippy Linter (eventrent-app sem unificação)",
    command: "cargo",
    args: ["clippy", "-p", "eventrent-app", "--", "-D", "warnings"],
  },
```

---

## 6. Verificações Executadas

- `pnpm verify` (10/10 etapas): `svelte-check` (0 erros/avisos), `check-no-hardcoded-colors` (0 cores literais em 36 `.svelte`), Prettier/ESLint, Vitest (**39/39**), prova de congelamento de migrações (**6/6**), `cargo fmt --check`, Clippy workspace + Clippy por crate sem unificação de features (`eventrent-core`, `eventrent-store-sqlite`, `eventrent-app`), e testes Rust (**9** `eventrent_app_lib` + **18** `eventrent_core` + **28** `eventrent_store_sqlite` = **55/55**).
- `pnpm test:e2e` (Playwright): **5/5** (`smoke.spec.ts`, `branding.spec.ts`, `customers.spec.ts`, `suppliers.spec.ts`, `venues.spec.ts`).
