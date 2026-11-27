use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::catalog::ControlMode;
use crate::domain::inventory::*;
use crate::domain::permission::{PERM_INVENTORY_MANAGE, PERM_INVENTORY_READ};
use crate::domain::user::SessionInfo;
use crate::error::{CoreError, Result};
use crate::services::CoreServices;

impl CoreServices {
    // ==========================================
    // 1. DEPÓSITOS (Warehouses)
    // ==========================================

    pub async fn list_warehouses(
        &self,
        session: &SessionInfo,
        include_inactive: bool,
    ) -> Result<Vec<Warehouse>> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;
        self.stores
            .inventory
            .list_warehouses(&session.tenant_id, include_inactive)
            .await
    }

    pub async fn get_warehouse(&self, session: &SessionInfo, id: &str) -> Result<Warehouse> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;
        self.stores
            .inventory
            .find_warehouse_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Depósito '{id}' não encontrado.")))
    }

    pub async fn create_warehouse(
        &self,
        session: &SessionInfo,
        input: &CreateWarehouseInput,
    ) -> Result<Warehouse> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        validate_warehouse_input(&input.name, &input.code)?;

        let now = Utc::now().to_rfc3339();
        let code = input.code.trim().to_uppercase();
        let warehouse = Warehouse {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            name: input.name.trim().to_string(),
            code: code.clone(),
            description: input.description.as_ref().map(|d| d.trim().to_string()),
            address_line1: input.address_line1.as_ref().map(|a| a.trim().to_string()),
            address_line2: input.address_line2.as_ref().map(|a| a.trim().to_string()),
            city: input.city.as_ref().map(|c| c.trim().to_string()),
            state: input.state.as_ref().map(|s| s.trim().to_uppercase()),
            postal_code: input.postal_code.as_ref().map(|p| p.trim().to_string()),
            is_default: input.is_default.unwrap_or(false),
            active: true,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let session_clone = session.clone();
        let wh_clone = warehouse.clone();
        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let wh = wh_clone.clone();
            Box::pin(async move {
                if let Some(existing) = stores
                    .inventory()
                    .find_warehouse_by_code(&session.tenant_id, &wh.code)
                    .await?
                {
                    if existing.deleted_at.is_none() {
                        return Err(CoreError::Conflict(format!(
                            "Já existe um depósito com o código '{}'.",
                            wh.code
                        )));
                    }
                }

                if wh.is_default {
                    stores
                        .inventory()
                        .set_default_warehouse(&session.tenant_id, &wh.id)
                        .await?;
                }
                stores.inventory().create_warehouse(&wh).await?;
                Ok(())
            })
        })
        .await?;

        Ok(warehouse)
    }

    pub async fn update_warehouse(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateWarehouseInput,
    ) -> Result<Warehouse> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        validate_warehouse_input(&input.name, &input.code)?;

        let session_clone = session.clone();
        let target_id = id.to_string();
        let input_clone = input.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = target_id.clone();
            let inp = input_clone.clone();
            Box::pin(async move {
                let existing = stores
                    .inventory()
                    .find_warehouse_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound(format!("Depósito '{id}' não encontrado.")))?;

                if existing.updated_at != inp.expected_updated_at {
                    return Err(CoreError::Conflict(
                        "O depósito foi modificado por outro usuário. Recarregue os dados e tente novamente."
                            .to_string(),
                    ));
                }

                let new_code = inp.code.trim().to_uppercase();
                if new_code != existing.code {
                    if let Some(other) = stores
                        .inventory()
                        .find_warehouse_by_code(&session.tenant_id, &new_code)
                        .await?
                    {
                        if other.id != existing.id && other.deleted_at.is_none() {
                            return Err(CoreError::Conflict(format!(
                                "Já existe outro depósito com o código '{new_code}'."
                            )));
                        }
                    }
                }

                let mut updated = existing;
                updated.name = inp.name.trim().to_string();
                updated.code = new_code;
                updated.description = inp.description.as_ref().map(|d| d.trim().to_string());
                updated.address_line1 = inp.address_line1.as_ref().map(|a| a.trim().to_string());
                updated.address_line2 = inp.address_line2.as_ref().map(|a| a.trim().to_string());
                updated.city = inp.city.as_ref().map(|c| c.trim().to_string());
                updated.state = inp.state.as_ref().map(|s| s.trim().to_uppercase());
                updated.postal_code = inp.postal_code.as_ref().map(|p| p.trim().to_string());
                if let Some(act) = inp.active {
                    updated.active = act;
                }
                updated.updated_at = Utc::now().to_rfc3339();

                stores.inventory().update_warehouse(&updated).await?;
                Ok(updated)
            })
        })
        .await
    }

    pub async fn set_default_warehouse(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        let session_clone = session.clone();
        let target_id = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let wh_id = target_id.clone();
            Box::pin(async move {
                let wh = stores
                    .inventory()
                    .find_warehouse_by_id(&session.tenant_id, &wh_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound(format!("Depósito '{wh_id}' não encontrado."))
                    })?;

                if !wh.active {
                    return Err(CoreError::Validation(
                        "Um depósito inativo não pode ser definido como padrão.".to_string(),
                    ));
                }

                stores
                    .inventory()
                    .set_default_warehouse(&session.tenant_id, &wh_id)
                    .await
            })
        })
        .await
    }

    pub async fn delete_warehouse(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        let session_clone = session.clone();
        let target_id = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let wh_id = target_id.clone();
            Box::pin(async move {
                let wh = stores
                    .inventory()
                    .find_warehouse_by_id(&session.tenant_id, &wh_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound(format!("Depósito '{wh_id}' não encontrado.")))?;

                if wh.is_default {
                    return Err(CoreError::Validation(
                        "O depósito padrão do tenant não pode ser excluído.".to_string(),
                    ));
                }

                let balances = stores
                    .inventory()
                    .list_stock_balances_by_warehouse(&session.tenant_id, &wh_id)
                    .await?;
                let total_items: i64 = balances
                    .iter()
                    .map(|b| b.quantity_on_hand + b.quantity_maintenance)
                    .sum();
                if total_items > 0 {
                    return Err(CoreError::Conflict(
                        "Não é possível excluir depósito com itens em estoque. Efetue transferência prévia ou inative-o."
                            .to_string(),
                    ));
                }

                let units = stores
                    .inventory()
                    .list_units(
                        &session.tenant_id,
                        &EquipmentUnitFilter {
                            warehouse_id: Some(wh_id.clone()),
                            active_only: Some(true),
                            ..Default::default()
                        },
                    )
                    .await?;
                if !units.is_empty() {
                    return Err(CoreError::Conflict(
                        "Não é possível excluir depósito com unidades físicas associadas."
                            .to_string(),
                    ));
                }

                stores
                    .inventory()
                    .delete_warehouse(&session.tenant_id, &wh_id)
                    .await
            })
        })
        .await
    }

    // ==========================================
    // 2. UNIDADES FÍSICAS (Equipment Units)
    // ==========================================

    pub async fn list_equipment_units(
        &self,
        session: &SessionInfo,
        filter: &EquipmentUnitFilter,
    ) -> Result<EquipmentUnitListPage> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;
        let mut page = self
            .stores
            .inventory
            .list_unit_details(&session.tenant_id, filter)
            .await?;

        for item in &mut page.items {
            sanitize_unit_financials(&mut item.unit, session);
        }

        Ok(page)
    }

    pub async fn get_equipment_unit(
        &self,
        session: &SessionInfo,
        id: &str,
    ) -> Result<EquipmentUnitDetails> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;
        let mut unit = self
            .stores
            .inventory
            .find_unit_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Unidade física '{id}' não encontrada.")))?;

        let model = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, &unit.equipment_model_id)
            .await?;
        let model_name = model
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "Modelo desconhecido".to_string());

        let warehouse_name = self
            .stores
            .inventory
            .find_warehouse_by_id(&session.tenant_id, &unit.warehouse_id)
            .await?
            .map(|w| w.name)
            .unwrap_or_else(|| "Depósito desconhecido".to_string());

        sanitize_unit_financials(&mut unit, session);

        Ok(EquipmentUnitDetails {
            unit,
            model_name,
            warehouse_name,
            model_thumbnail_url: None,
        })
    }

    pub async fn create_equipment_unit(
        &self,
        session: &SessionInfo,
        input: &CreateEquipmentUnitInput,
    ) -> Result<EquipmentUnit> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        let origin = input.origin.unwrap_or(UnitOrigin::Own);
        validate_equipment_unit_input(
            &input.equipment_model_id,
            input.serial_number.as_deref(),
            input.notes.as_deref(),
            origin,
            input.supplier_id.as_deref(),
            input.return_by.as_deref(),
            input.acquisition_cost_cents,
            input.location_note.as_deref(),
            input.invoice_number.as_deref(),
        )?;

        validate_financial_mutation_permissions(
            session,
            input.acquisition_cost_cents,
            input.supplier_id.as_deref(),
            input.invoice_number.as_deref(),
        )?;

        if origin == UnitOrigin::ThirdParty {
            let sup_empty = input
                .supplier_id
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            let ret_empty = input
                .return_by
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if sup_empty || ret_empty {
                return Err(CoreError::Validation(
                    "Unidades de terceiros exigem obrigatoriamente o fornecedor e a data prevista de devolução."
                        .to_string(),
                ));
            }
        }

        let session_clone = session.clone();
        let input_clone = input.clone();

        let mut unit = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let inp = input_clone.clone();
                Box::pin(async move {
                    let model = stores
                        .catalog()
                        .find_model_by_id(&session.tenant_id, &inp.equipment_model_id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                        })?;

                    if model.deleted_at.is_some() || !model.active {
                        return Err(CoreError::Validation(
                            "Modelo inativo ou excluído não pode receber novas unidades."
                                .to_string(),
                        ));
                    }

                    if model.control_mode != ControlMode::Serialized {
                        return Err(CoreError::Validation(
                            "Apenas modelos serializados podem possuir unidades físicas."
                                .to_string(),
                        ));
                    }

                    let warehouse_id = match &inp.warehouse_id {
                        Some(wid) => {
                            let wh = stores
                                .inventory()
                                .find_warehouse_by_id(&session.tenant_id, wid)
                                .await?
                                .ok_or_else(|| {
                                    CoreError::NotFound(format!("Depósito '{wid}' não encontrado."))
                                })?;
                            if !wh.active || wh.deleted_at.is_some() {
                                return Err(CoreError::Validation(
                                    "Depósito selecionado está inativo ou excluído.".to_string(),
                                ));
                            }
                            wid.clone()
                        }
                        None => {
                            let def = stores
                                .inventory()
                                .get_default_warehouse(&session.tenant_id)
                                .await?
                                .ok_or_else(|| {
                                    CoreError::Configuration(
                                        "Nenhum depósito padrão configurado.".to_string(),
                                    )
                                })?;
                            def.id
                        }
                    };

                    if let Some(ref sid) = inp.supplier_id {
                        if !sid.trim().is_empty() {
                            let sup = stores
                                .customers()
                                .find_by_id(&session.tenant_id, sid)
                                .await?;
                            let is_sup = sup
                                .as_ref()
                                .map(|c| c.customer.is_supplier)
                                .unwrap_or(false);
                            if !is_sup {
                                return Err(CoreError::NotFound(
                                    "Fornecedor informado não encontrado ou inválido.".to_string(),
                                ));
                            }
                        }
                    }

                    let patrimony_tag = stores
                        .inventory()
                        .next_patrimony_tag(&session.tenant_id)
                        .await?;
                    let now = Utc::now().to_rfc3339();
                    let unit_id = Uuid::now_v7().to_string();

                    let unit = EquipmentUnit {
                        id: unit_id.clone(),
                        tenant_id: session.tenant_id.clone(),
                        equipment_model_id: inp.equipment_model_id.clone(),
                        warehouse_id: warehouse_id.clone(),
                        patrimony_tag: patrimony_tag.clone(),
                        serial_number: inp.serial_number.as_ref().map(|s| s.trim().to_string()),
                        status: UnitStatus::Available,
                        physical_condition: inp
                            .physical_condition
                            .unwrap_or(PhysicalCondition::Good),
                        notes: inp.notes.as_ref().map(|n| n.trim().to_string()),
                        acquisition_date: inp.acquisition_date.clone(),
                        warranty_expires_at: inp.warranty_expires_at.clone(),
                        acquisition_cost_cents: inp.acquisition_cost_cents,
                        supplier_id: inp.supplier_id.as_ref().map(|s| s.trim().to_string()),
                        invoice_number: inp.invoice_number.as_ref().map(|i| i.trim().to_string()),
                        origin,
                        return_by: inp.return_by.clone(),
                        usage_hours: inp.usage_hours.unwrap_or(0).max(0),
                        location_note: inp.location_note.as_ref().map(|l| l.trim().to_string()),
                        active: true,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        deleted_at: None,
                        created_by: Some(session.user_id.clone()),
                    };

                    let movement = StockMovement {
                        id: Uuid::now_v7().to_string(),
                        tenant_id: session.tenant_id.clone(),
                        equipment_model_id: inp.equipment_model_id.clone(),
                        equipment_unit_id: Some(unit_id),
                        from_warehouse_id: None,
                        to_warehouse_id: Some(warehouse_id),
                        movement_type: StockMovementType::PurchaseIn,
                        quantity: 1,
                        reason: format!(
                            "Entrada e tombamento de unidade patrimonial {patrimony_tag}"
                        ),
                        reference_id: None,
                        performed_by_user_id: session.user_id.clone(),
                        created_at: now,
                    };

                    stores.inventory().create_unit(&unit).await?;
                    stores.inventory().record_stock_movement(&movement).await?;

                    Ok(unit)
                })
            })
            .await?;

        sanitize_unit_financials(&mut unit, session);
        Ok(unit)
    }

    pub async fn update_equipment_unit(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateEquipmentUnitInput,
    ) -> Result<EquipmentUnit> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;

        validate_financial_mutation_permissions(
            session,
            input.acquisition_cost_cents,
            input.supplier_id.as_deref(),
            input.invoice_number.as_deref(),
        )?;

        let session_clone = session.clone();
        let target_id = id.to_string();
        let input_clone = input.clone();

        let mut unit = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let uid = target_id.clone();
                let inp = input_clone.clone();
                Box::pin(async move {
                    let existing = stores
                        .inventory()
                        .find_unit_by_id(&session.tenant_id, &uid)
                        .await?
                        .ok_or_else(|| CoreError::NotFound(format!("Unidade '{uid}' não encontrada.")))?;

                    if existing.updated_at != inp.expected_updated_at {
                        return Err(CoreError::Conflict(
                            "A unidade física foi alterada por outro usuário. Recarregue os dados e tente novamente."
                                .to_string(),
                        ));
                    }

                    let old_warehouse_id = existing.warehouse_id.clone();
                    let old_status = existing.status;
                    let target_status = inp.status.unwrap_or(old_status);

                    validate_status_transition(
                        old_status,
                        target_status,
                        inp.status_change_reason.as_deref(),
                    )?;

                    let new_warehouse_id = match &inp.warehouse_id {
                        Some(wid) => {
                            if wid != &old_warehouse_id {
                                let wh = stores
                                    .inventory()
                                    .find_warehouse_by_id(&session.tenant_id, wid)
                                    .await?
                                    .ok_or_else(|| {
                                        CoreError::NotFound(format!("Depósito '{wid}' não encontrado."))
                                    })?;
                                if !wh.active || wh.deleted_at.is_some() {
                                    return Err(CoreError::Validation(
                                        "Depósito de destino inválido ou inativo.".to_string(),
                                    ));
                                }
                            }
                            wid.clone()
                        }
                        None => old_warehouse_id.clone(),
                    };

                    let target_origin = inp.origin.unwrap_or(existing.origin);
                    let target_supplier = inp
                        .supplier_id
                        .as_ref()
                        .map(|s| s.trim().to_string())
                        .or_else(|| existing.supplier_id.clone());
                    let target_return_by = inp.return_by.clone().or_else(|| existing.return_by.clone());

                    if let Some(ref sid) = target_supplier {
                        if !sid.trim().is_empty() {
                            let sup = stores.customers().find_by_id(&session.tenant_id, sid).await?;
                            let is_sup = sup.as_ref().map(|c| c.customer.is_supplier).unwrap_or(false);
                            if !is_sup {
                                return Err(CoreError::NotFound(
                                    "Fornecedor informado não encontrado ou inválido.".to_string(),
                                ));
                            }
                        }
                    }

                    if target_origin == UnitOrigin::ThirdParty {
                        let sup_empty = target_supplier
                            .as_ref()
                            .map(|s| s.trim().is_empty())
                            .unwrap_or(true);
                        let ret_empty = target_return_by
                            .as_ref()
                            .map(|s| s.trim().is_empty())
                            .unwrap_or(true);
                        if sup_empty || ret_empty {
                            return Err(CoreError::Validation(
                                "Unidades de terceiros exigem obrigatoriamente o fornecedor e a data prevista de devolução."
                                    .to_string(),
                            ));
                        }
                    }

                    let now = Utc::now().to_rfc3339();
                    let mut updated = existing;
                    updated.warehouse_id = new_warehouse_id.clone();
                    if let Some(ref sn) = inp.serial_number {
                        updated.serial_number = Some(sn.trim().to_string());
                    }
                    updated.status = target_status;
                    if let Some(pc) = inp.physical_condition {
                        updated.physical_condition = pc;
                    }
                    if let Some(ref n) = inp.notes {
                        updated.notes = Some(n.trim().to_string());
                    }
                    if let Some(ref ad) = inp.acquisition_date {
                        updated.acquisition_date = Some(ad.clone());
                    }
                    if let Some(ref we) = inp.warranty_expires_at {
                        updated.warranty_expires_at = Some(we.clone());
                    }
                    if let Some(c) = inp.acquisition_cost_cents {
                        updated.acquisition_cost_cents = Some(c);
                    }
                    updated.supplier_id = target_supplier;
                    if let Some(ref inv) = inp.invoice_number {
                        updated.invoice_number = Some(inv.trim().to_string());
                    }
                    updated.origin = target_origin;
                    updated.return_by = target_return_by;
                    if let Some(h) = inp.usage_hours {
                        updated.usage_hours = h.max(0);
                    }
                    if let Some(ref l) = inp.location_note {
                        updated.location_note = Some(l.trim().to_string());
                    }
                    if let Some(act) = inp.active {
                        updated.active = act;
                    }
                    updated.updated_at = now.clone();

                    stores.inventory().update_unit(&updated).await?;

                    // R3: Tratamento de movimentos separados na mesma transação
                    let warehouse_changed = new_warehouse_id != old_warehouse_id;
                    let status_changed = target_status != old_status;

                    if warehouse_changed {
                        let transfer_mov = StockMovement {
                            id: Uuid::now_v7().to_string(),
                            tenant_id: session.tenant_id.clone(),
                            equipment_model_id: updated.equipment_model_id.clone(),
                            equipment_unit_id: Some(updated.id.clone()),
                            from_warehouse_id: Some(old_warehouse_id.clone()),
                            to_warehouse_id: Some(new_warehouse_id.clone()),
                            movement_type: StockMovementType::TransferOut,
                            quantity: 1,
                            reason: format!(
                                "Transferência física da unidade {}",
                                updated.patrimony_tag
                            ),
                            reference_id: None,
                            performed_by_user_id: session.user_id.clone(),
                            created_at: now.clone(),
                        };
                        stores.inventory().record_stock_movement(&transfer_mov).await?;
                    }

                    if status_changed {
                        let (m_type, default_reason) = match target_status {
                            UnitStatus::Lost => (
                                StockMovementType::LossOut,
                                format!("Extravio/Perda da unidade {}", updated.patrimony_tag),
                            ),
                            _ => (
                                StockMovementType::StatusChange,
                                format!(
                                    "Alteração de status da unidade {} de {:?} para {:?}",
                                    updated.patrimony_tag, old_status, target_status
                                ),
                            ),
                        };

                        let final_reason = inp
                            .status_change_reason
                            .as_ref()
                            .filter(|r| !r.trim().is_empty())
                            .cloned()
                            .unwrap_or(default_reason);

                        let status_mov = StockMovement {
                            id: Uuid::now_v7().to_string(),
                            tenant_id: session.tenant_id.clone(),
                            equipment_model_id: updated.equipment_model_id.clone(),
                            equipment_unit_id: Some(updated.id.clone()),
                            from_warehouse_id: Some(new_warehouse_id),
                            to_warehouse_id: None,
                            movement_type: m_type,
                            quantity: 1,
                            reason: final_reason,
                            reference_id: None,
                            performed_by_user_id: session.user_id.clone(),
                            created_at: now,
                        };
                        stores.inventory().record_stock_movement(&status_mov).await?;
                    }

                    Ok(updated)
                })
            })
            .await?;

        sanitize_unit_financials(&mut unit, session);
        Ok(unit)
    }

    pub async fn decommission_equipment_unit(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &DecommissionEquipmentUnitInput,
    ) -> Result<EquipmentUnit> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;

        let reason_trimmed = input.reason.trim();
        if reason_trimmed.is_empty() {
            return Err(CoreError::Validation(
                "Motivo obrigatório para baixa patrimonial da unidade.".to_string(),
            ));
        }

        let session_clone = session.clone();
        let target_id = id.to_string();
        let input_clone = input.clone();

        let mut unit = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let uid = target_id.clone();
                let inp = input_clone.clone();
                Box::pin(async move {
                    let existing = stores
                        .inventory()
                        .find_unit_by_id(&session.tenant_id, &uid)
                        .await?
                        .ok_or_else(|| CoreError::NotFound(format!("Unidade '{uid}' não encontrada.")))?;

                    if existing.updated_at != inp.expected_updated_at {
                        return Err(CoreError::Conflict(
                            "A unidade física foi alterada por outro usuário. Recarregue os dados e tente novamente."
                                .to_string(),
                        ));
                    }

                    if existing.status == UnitStatus::Decommissioned {
                        return Err(CoreError::Conflict(
                            "A unidade patrimonial já se encontra baixada.".to_string(),
                        ));
                    }

                    let now = Utc::now().to_rfc3339();
                    let mut updated = existing;
                    updated.status = UnitStatus::Decommissioned;
                    updated.active = false;
                    updated.updated_at = now.clone();

                    stores.inventory().update_unit(&updated).await?;

                    let decommission_mov = StockMovement {
                        id: Uuid::now_v7().to_string(),
                        tenant_id: session.tenant_id.clone(),
                        equipment_model_id: updated.equipment_model_id.clone(),
                        equipment_unit_id: Some(updated.id.clone()),
                        from_warehouse_id: Some(updated.warehouse_id.clone()),
                        to_warehouse_id: None,
                        movement_type: StockMovementType::DecommissionOut,
                        quantity: 1,
                        reason: format!(
                            "Baixa patrimonial definitiva: {}",
                            inp.reason.trim()
                        ),
                        reference_id: None,
                        performed_by_user_id: session.user_id.clone(),
                        created_at: now,
                    };
                    stores.inventory().record_stock_movement(&decommission_mov).await?;

                    Ok(updated)
                })
            })
            .await?;

        sanitize_unit_financials(&mut unit, session);
        Ok(unit)
    }

    pub async fn delete_equipment_unit(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        let session_clone = session.clone();
        let target_id = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let uid = target_id.clone();
            Box::pin(async move {
                let unit = stores
                    .inventory()
                    .find_unit_by_id(&session.tenant_id, &uid)
                    .await?
                    .ok_or_else(|| CoreError::NotFound(format!("Unidade '{uid}' não encontrada.")))?;

                // R4: Exclusão permitida apenas se criada por engano há menos de 24 horas
                let created_dt = DateTime::parse_from_rfc3339(&unit.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| CoreError::Internal(format!("Data de criação inválida: {e}")))?;
                let now = Utc::now();
                let hours_elapsed = now.signed_duration_since(created_dt).num_hours();
                if hours_elapsed >= 24 {
                    return Err(CoreError::Conflict(
                        "Unidades criadas há mais de 24 horas não podem ser excluídas; efetue a baixa patrimonial (decommission)."
                            .to_string(),
                    ));
                }

                // R4: Não pode ter movimentações além da entrada inicial de tombamento
                let mov_count = stores
                    .inventory()
                    .count_unit_movements(&session.tenant_id, &uid)
                    .await?;
                if mov_count > 1 {
                    return Err(CoreError::Conflict(
                        "Unidades com movimentações além do tombamento inicial não podem ser excluídas; efetue a baixa patrimonial (decommission)."
                            .to_string(),
                    ));
                }

                // Reverte saldo físico decorrente da entrada inicial
                let reverse_mov = StockMovement {
                    id: Uuid::now_v7().to_string(),
                    tenant_id: session.tenant_id.clone(),
                    equipment_model_id: unit.equipment_model_id.clone(),
                    equipment_unit_id: Some(unit.id.clone()),
                    from_warehouse_id: Some(unit.warehouse_id.clone()),
                    to_warehouse_id: None,
                    movement_type: StockMovementType::LossOut,
                    quantity: 1,
                    reason: format!(
                        "Exclusão por engano de cadastro recente (<24h) da unidade {}",
                        unit.patrimony_tag
                    ),
                    reference_id: None,
                    performed_by_user_id: session.user_id.clone(),
                    created_at: Utc::now().to_rfc3339(),
                };
                stores.inventory().record_stock_movement(&reverse_mov).await?;
                stores.inventory().delete_unit(&session.tenant_id, &uid).await?;

                Ok(())
            })
        })
        .await
    }

    // ==========================================
    // 3. ESTOQUE E SALDOS (Stock & Balances)
    // ==========================================

    pub async fn get_stock_balance(
        &self,
        session: &SessionInfo,
        warehouse_id: &str,
        equipment_model_id: &str,
    ) -> Result<StockBalance> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;
        let balance = self
            .stores
            .inventory
            .get_stock_balance(&session.tenant_id, warehouse_id, equipment_model_id)
            .await?;

        Ok(balance.unwrap_or_else(|| StockBalance {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            warehouse_id: warehouse_id.to_string(),
            equipment_model_id: equipment_model_id.to_string(),
            quantity_on_hand: 0,
            quantity_maintenance: 0,
            min_quantity: 0,
            updated_at: Utc::now().to_rfc3339(),
        }))
    }

    pub async fn list_stock_balances(
        &self,
        session: &SessionInfo,
        equipment_model_id: Option<&str>,
        warehouse_id: Option<&str>,
    ) -> Result<Vec<StockBalance>> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;

        // R7: Exigir ao menos modelo ou depósito
        if equipment_model_id.is_none() && warehouse_id.is_none() {
            return Err(CoreError::Validation(
                "Informe ao menos o modelo ou o depósito para consultar saldos de estoque."
                    .to_string(),
            ));
        }

        if let Some(mid) = equipment_model_id {
            self.stores
                .inventory
                .list_stock_balances_by_model(&session.tenant_id, mid)
                .await
        } else if let Some(wid) = warehouse_id {
            self.stores
                .inventory
                .list_stock_balances_by_warehouse(&session.tenant_id, wid)
                .await
        } else {
            Ok(vec![])
        }
    }

    pub async fn record_stock_movement(
        &self,
        session: &SessionInfo,
        input: &RecordStockMovementInput,
    ) -> Result<StockMovement> {
        self.check_session_and_permission(session, PERM_INVENTORY_MANAGE)?;
        validate_stock_movement_input(
            &input.equipment_model_id,
            input.quantity,
            &input.reason,
            input.from_warehouse_id.as_deref(),
            input.to_warehouse_id.as_deref(),
            input.movement_type,
        )?;

        let session_clone = session.clone();
        let input_clone = input.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let inp = input_clone.clone();
            Box::pin(async move {
                let model = stores
                    .catalog()
                    .find_model_by_id(&session.tenant_id, &inp.equipment_model_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                    })?;

                if model.deleted_at.is_some() || !model.active {
                    return Err(CoreError::Validation(
                        "Modelo inativo ou excluído não pode receber movimentações de estoque."
                            .to_string(),
                    ));
                }

                // R2: Validação de compatibilidade de control_mode
                if let Some(ref uid) = inp.equipment_unit_id {
                    if model.control_mode != ControlMode::Serialized {
                        return Err(CoreError::Validation(
                            "Movimentações com unidade física informada exigem modelo serializado."
                                .to_string(),
                        ));
                    }
                    let unit = stores
                        .inventory()
                        .find_unit_by_id(&session.tenant_id, uid)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Unidade física não encontrada.".to_string())
                        })?;
                    if unit.equipment_model_id != model.id {
                        return Err(CoreError::Validation(
                            "A unidade informada não pertence ao modelo especificado.".to_string(),
                        ));
                    }
                } else if model.control_mode == ControlMode::Serialized {
                    return Err(CoreError::Validation(
                        "Modelos serializados exigem a identificação da unidade física em suas movimentações."
                            .to_string(),
                    ));
                }

                if let Some(ref from_wh) = inp.from_warehouse_id {
                    let wh = stores
                        .inventory()
                        .find_warehouse_by_id(&session.tenant_id, from_wh)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Depósito de origem não encontrado.".to_string())
                        })?;
                    if !wh.active || wh.deleted_at.is_some() {
                        return Err(CoreError::Validation(
                            "Depósito de origem está inativo ou excluído.".to_string(),
                        ));
                    }
                }

                if let Some(ref to_wh) = inp.to_warehouse_id {
                    let wh = stores
                        .inventory()
                        .find_warehouse_by_id(&session.tenant_id, to_wh)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Depósito de destino não encontrado.".to_string())
                        })?;
                    if !wh.active || wh.deleted_at.is_some() {
                        return Err(CoreError::Validation(
                            "Depósito de destino está inativo ou excluído.".to_string(),
                        ));
                    }
                }

                let now = Utc::now().to_rfc3339();
                let movement = StockMovement {
                    id: Uuid::now_v7().to_string(),
                    tenant_id: session.tenant_id.clone(),
                    equipment_model_id: inp.equipment_model_id.clone(),
                    equipment_unit_id: inp.equipment_unit_id.clone(),
                    from_warehouse_id: inp.from_warehouse_id.clone(),
                    to_warehouse_id: inp.to_warehouse_id.clone(),
                    movement_type: inp.movement_type,
                    quantity: inp.quantity,
                    reason: inp.reason.trim().to_string(),
                    reference_id: inp.reference_id.clone(),
                    performed_by_user_id: session.user_id.clone(),
                    created_at: now,
                };

                stores.inventory().record_stock_movement(&movement).await?;
                Ok(movement)
            })
        })
        .await
    }

    pub async fn list_stock_movements(
        &self,
        session: &SessionInfo,
        filter: &StockMovementFilter,
    ) -> Result<Vec<StockMovement>> {
        self.check_session_and_permission(session, PERM_INVENTORY_READ)?;
        self.stores
            .inventory
            .list_stock_movements(&session.tenant_id, filter)
            .await
    }

    // ==========================================
    // 4. SANITIZAÇÃO DE DADOS FINANCEIROS
    // ==========================================

    pub fn sanitize_model_financials(
        &self,
        session: &SessionInfo,
        model: &mut crate::domain::catalog::EquipmentModel,
    ) {
        if !can_view_financials(session) {
            model.cost_price_cents = None;
            model.replacement_value_cents = None;
        }
    }
}

/// Garante de forma idempotente a existência do depósito padrão ("Depósito principal", code = "PRINCIPAL")
/// para um tenant.
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
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
        created_by: None,
    };

    inventory.create_warehouse(&warehouse).await?;
    Ok(warehouse)
}
