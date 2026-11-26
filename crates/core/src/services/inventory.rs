use chrono::Utc;
use uuid::Uuid;

use crate::domain::inventory::*;
use crate::domain::permission::{PERM_FINANCE_READ, PERM_WAREHOUSE_OPERATE, PERM_WAREHOUSE_READ};
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
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
        self.stores
            .inventory
            .list_warehouses(&session.tenant_id, include_inactive)
            .await
    }

    pub async fn get_warehouse(&self, session: &SessionInfo, id: &str) -> Result<Warehouse> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
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
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        validate_warehouse_input(&input.name, &input.code)?;

        let now = Utc::now().to_rfc3339();
        let warehouse = Warehouse {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            name: input.name.trim().to_string(),
            code: input.code.trim().to_uppercase(),
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
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        validate_warehouse_input(&input.name, &input.code)?;

        let mut wh = self.get_warehouse(session, id).await?;
        wh.name = input.name.trim().to_string();
        wh.code = input.code.trim().to_uppercase();
        wh.description = input.description.as_ref().map(|d| d.trim().to_string());
        wh.address_line1 = input.address_line1.as_ref().map(|a| a.trim().to_string());
        wh.address_line2 = input.address_line2.as_ref().map(|a| a.trim().to_string());
        wh.city = input.city.as_ref().map(|c| c.trim().to_string());
        wh.state = input.state.as_ref().map(|s| s.trim().to_uppercase());
        wh.postal_code = input.postal_code.as_ref().map(|p| p.trim().to_string());
        if let Some(act) = input.active {
            wh.active = act;
        }
        wh.updated_at = Utc::now().to_rfc3339();

        let wh_clone = wh.clone();
        self.run_transaction(|stores| {
            let wh = wh_clone.clone();
            Box::pin(async move {
                stores.inventory().update_warehouse(&wh).await?;
                Ok(())
            })
        })
        .await?;

        Ok(wh)
    }

    pub async fn set_default_warehouse(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        let wh = self.get_warehouse(session, id).await?;
        if !wh.active {
            return Err(CoreError::Validation(
                "Um depósito inativo não pode ser definido como padrão.".to_string(),
            ));
        }

        let tenant_id = session.tenant_id.clone();
        let target_id = id.to_string();
        self.run_transaction(|stores| {
            let t_id = tenant_id.clone();
            let wh_id = target_id.clone();
            Box::pin(async move {
                stores
                    .inventory()
                    .set_default_warehouse(&t_id, &wh_id)
                    .await
            })
        })
        .await
    }

    pub async fn delete_warehouse(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        let wh = self.get_warehouse(session, id).await?;
        if wh.is_default {
            return Err(CoreError::Validation(
                "O depósito padrão do tenant não pode ser excluído.".to_string(),
            ));
        }

        // Verifica se há saldos ativos ou unidades alocadas no depósito
        let balances = self
            .stores
            .inventory
            .list_stock_balances_by_warehouse(&session.tenant_id, id)
            .await?;
        let total_items: i64 = balances
            .iter()
            .map(|b| b.quantity_on_hand + b.quantity_allocated + b.quantity_maintenance)
            .sum();
        if total_items > 0 {
            return Err(CoreError::Conflict(
                "Não é possível excluir depósito com itens ou unidades em estoque. Efetue transferência prévia ou inative-o.".to_string(),
            ));
        }

        let tenant_id = session.tenant_id.clone();
        let wh_id = id.to_string();
        self.run_transaction(|stores| {
            let t_id = tenant_id.clone();
            let w_id = wh_id.clone();
            Box::pin(async move { stores.inventory().delete_warehouse(&t_id, &w_id).await })
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
    ) -> Result<Vec<EquipmentUnitDetails>> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
        let units = self
            .stores
            .inventory
            .list_units(&session.tenant_id, filter)
            .await?;
        let mut result = Vec::with_capacity(units.len());

        for unit in units {
            let model_name = self
                .stores
                .catalog
                .find_model_by_id(&session.tenant_id, &unit.equipment_model_id)
                .await?
                .map(|m| m.name)
                .unwrap_or_else(|| "Modelo desconhecido".to_string());

            let warehouse_name = self
                .stores
                .inventory
                .find_warehouse_by_id(&session.tenant_id, &unit.warehouse_id)
                .await?
                .map(|w| w.name)
                .unwrap_or_else(|| "Depósito desconhecido".to_string());

            let media = self
                .stores
                .inventory
                .list_unit_media(&session.tenant_id, &unit.id)
                .await?;

            result.push(EquipmentUnitDetails {
                unit,
                model_name,
                warehouse_name,
                media,
            });
        }

        Ok(result)
    }

    pub async fn get_equipment_unit(
        &self,
        session: &SessionInfo,
        id: &str,
    ) -> Result<EquipmentUnitDetails> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
        let unit = self
            .stores
            .inventory
            .find_unit_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Unidade física '{id}' não encontrada.")))?;

        let model_name = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, &unit.equipment_model_id)
            .await?
            .map(|m| m.name)
            .unwrap_or_else(|| "Modelo desconhecido".to_string());

        let warehouse_name = self
            .stores
            .inventory
            .find_warehouse_by_id(&session.tenant_id, &unit.warehouse_id)
            .await?
            .map(|w| w.name)
            .unwrap_or_else(|| "Depósito desconhecido".to_string());

        let media = self
            .stores
            .inventory
            .list_unit_media(&session.tenant_id, &unit.id)
            .await?;

        Ok(EquipmentUnitDetails {
            unit,
            model_name,
            warehouse_name,
            media,
        })
    }

    pub async fn create_equipment_unit(
        &self,
        session: &SessionInfo,
        input: &CreateEquipmentUnitInput,
    ) -> Result<EquipmentUnit> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        validate_equipment_unit_input(
            &input.equipment_model_id,
            input.serial_number.as_deref(),
            input.notes.as_deref(),
        )?;

        // Valida se o modelo existe e se é serializado
        let model = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, &input.equipment_model_id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
            })?;

        if model.control_mode != crate::domain::catalog::ControlMode::Serialized {
            return Err(CoreError::Validation(
                "Apenas modelos configurados com modo de controle 'serialized' podem possuir unidades físicas individuais.".to_string(),
            ));
        }

        // Determina o depósito destino (informado ou padrão)
        let warehouse_id = match &input.warehouse_id {
            Some(wid) => wid.clone(),
            None => {
                let def = self
                    .stores
                    .inventory
                    .get_default_warehouse(&session.tenant_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Configuration("Nenhum depósito padrão configurado.".to_string())
                    })?;
                def.id
            }
        };

        // Garante próximo patrimônio sequencial único atômico
        let patrimony_tag = self
            .stores
            .inventory
            .next_patrimony_tag(&session.tenant_id)
            .await?;
        let now = Utc::now().to_rfc3339();
        let unit_id = Uuid::now_v7().to_string();

        let unit = EquipmentUnit {
            id: unit_id.clone(),
            tenant_id: session.tenant_id.clone(),
            equipment_model_id: input.equipment_model_id.clone(),
            warehouse_id: warehouse_id.clone(),
            patrimony_tag: patrimony_tag.clone(),
            serial_number: input.serial_number.as_ref().map(|s| s.trim().to_string()),
            status: UnitStatus::Available,
            physical_condition: input.physical_condition.unwrap_or(PhysicalCondition::Good),
            notes: input.notes.as_ref().map(|n| n.trim().to_string()),
            acquisition_date: input.acquisition_date.clone(),
            warranty_expires_at: input.warranty_expires_at.clone(),
            active: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let movement = StockMovement {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            equipment_model_id: input.equipment_model_id.clone(),
            equipment_unit_id: Some(unit_id),
            from_warehouse_id: None,
            to_warehouse_id: Some(warehouse_id),
            movement_type: StockMovementType::PurchaseIn,
            quantity: 1,
            reason: format!("Entrada e tombamento de unidade patrimonial {patrimony_tag}"),
            reference_id: None,
            performed_by_user_id: session.user_id.clone(),
            created_at: now,
        };

        let unit_clone = unit.clone();
        let mov_clone = movement.clone();
        self.run_transaction(|stores| {
            let u = unit_clone.clone();
            let m = mov_clone.clone();
            Box::pin(async move {
                stores.inventory().create_unit(&u).await?;
                stores.inventory().record_stock_movement(&m).await?;
                Ok(())
            })
        })
        .await?;

        Ok(unit)
    }

    pub async fn update_equipment_unit(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateEquipmentUnitInput,
    ) -> Result<EquipmentUnit> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        let mut unit = self
            .stores
            .inventory
            .find_unit_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Unidade '{id}' não encontrada.")))?;

        let old_warehouse_id = unit.warehouse_id.clone();
        let old_status = unit.status;

        if let Some(ref wid) = input.warehouse_id {
            unit.warehouse_id = wid.clone();
        }
        if let Some(ref sn) = input.serial_number {
            unit.serial_number = Some(sn.trim().to_string());
        }
        if let Some(st) = input.status {
            unit.status = st;
        }
        if let Some(pc) = input.physical_condition {
            unit.physical_condition = pc;
        }
        if let Some(ref n) = input.notes {
            unit.notes = Some(n.trim().to_string());
        }
        if let Some(ref ad) = input.acquisition_date {
            unit.acquisition_date = Some(ad.clone());
        }
        if let Some(ref we) = input.warranty_expires_at {
            unit.warranty_expires_at = Some(we.clone());
        }
        if let Some(act) = input.active {
            unit.active = act;
        }
        unit.updated_at = Utc::now().to_rfc3339();

        // Se houve alteração de depósito ou status, registrar movimentação
        let mut optional_movement: Option<StockMovement> = None;
        if unit.warehouse_id != old_warehouse_id {
            optional_movement = Some(StockMovement {
                id: Uuid::now_v7().to_string(),
                tenant_id: session.tenant_id.clone(),
                equipment_model_id: unit.equipment_model_id.clone(),
                equipment_unit_id: Some(unit.id.clone()),
                from_warehouse_id: Some(old_warehouse_id),
                to_warehouse_id: Some(unit.warehouse_id.clone()),
                movement_type: StockMovementType::TransferOut,
                quantity: 1,
                reason: format!("Transferência física da unidade {}", unit.patrimony_tag),
                reference_id: None,
                performed_by_user_id: session.user_id.clone(),
                created_at: Utc::now().to_rfc3339(),
            });
        } else if unit.status != old_status {
            let (m_type, reason) = match unit.status {
                UnitStatus::Maintenance => (
                    StockMovementType::MaintenanceOut,
                    format!("Envio para manutenção da unidade {}", unit.patrimony_tag),
                ),
                UnitStatus::Available if old_status == UnitStatus::Maintenance => (
                    StockMovementType::MaintenanceIn,
                    format!("Retorno de manutenção da unidade {}", unit.patrimony_tag),
                ),
                UnitStatus::Damaged => (
                    StockMovementType::LossOut,
                    format!("Registro de avaria na unidade {}", unit.patrimony_tag),
                ),
                UnitStatus::Lost => (
                    StockMovementType::LossOut,
                    format!("Extravio/Perda da unidade {}", unit.patrimony_tag),
                ),
                _ => (
                    StockMovementType::Adjustment,
                    format!("Mudança de status operacional para {:?}", unit.status),
                ),
            };

            optional_movement = Some(StockMovement {
                id: Uuid::now_v7().to_string(),
                tenant_id: session.tenant_id.clone(),
                equipment_model_id: unit.equipment_model_id.clone(),
                equipment_unit_id: Some(unit.id.clone()),
                from_warehouse_id: Some(unit.warehouse_id.clone()),
                to_warehouse_id: None,
                movement_type: m_type,
                quantity: 1,
                reason,
                reference_id: None,
                performed_by_user_id: session.user_id.clone(),
                created_at: Utc::now().to_rfc3339(),
            });
        }

        let unit_clone = unit.clone();
        self.run_transaction(|stores| {
            let u = unit_clone.clone();
            let opt_m = optional_movement.clone();
            Box::pin(async move {
                stores.inventory().update_unit(&u).await?;
                if let Some(ref m) = opt_m {
                    stores.inventory().record_stock_movement(m).await?;
                }
                Ok(())
            })
        })
        .await?;

        Ok(unit)
    }

    pub async fn delete_equipment_unit(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        let unit = self
            .stores
            .inventory
            .find_unit_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("Unidade '{id}' não encontrada.")))?;

        let tenant_id = session.tenant_id.clone();
        let u_id = id.to_string();
        let movement = StockMovement {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            equipment_model_id: unit.equipment_model_id.clone(),
            equipment_unit_id: Some(unit.id.clone()),
            from_warehouse_id: Some(unit.warehouse_id.clone()),
            to_warehouse_id: None,
            movement_type: StockMovementType::LossOut,
            quantity: 1,
            reason: format!(
                "Baixa patrimonial definitiva da unidade {}",
                unit.patrimony_tag
            ),
            reference_id: None,
            performed_by_user_id: session.user_id.clone(),
            created_at: Utc::now().to_rfc3339(),
        };

        self.run_transaction(|stores| {
            let t = tenant_id.clone();
            let u = u_id.clone();
            let m = movement.clone();
            Box::pin(async move {
                stores.inventory().delete_unit(&t, &u).await?;
                stores.inventory().record_stock_movement(&m).await?;
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
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
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
            quantity_allocated: 0,
            quantity_maintenance: 0,
            updated_at: Utc::now().to_rfc3339(),
        }))
    }

    pub async fn list_stock_balances(
        &self,
        session: &SessionInfo,
        equipment_model_id: Option<&str>,
        warehouse_id: Option<&str>,
    ) -> Result<Vec<StockBalance>> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
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
            // Se nenhum filtro, traz todos do depósito padrão
            let def_wh = self
                .stores
                .inventory
                .get_default_warehouse(&session.tenant_id)
                .await?;
            if let Some(wh) = def_wh {
                self.stores
                    .inventory
                    .list_stock_balances_by_warehouse(&session.tenant_id, &wh.id)
                    .await
            } else {
                Ok(vec![])
            }
        }
    }

    pub async fn record_stock_movement(
        &self,
        session: &SessionInfo,
        input: &RecordStockMovementInput,
    ) -> Result<StockMovement> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        validate_stock_movement_input(
            &input.equipment_model_id,
            input.quantity,
            &input.reason,
            input.from_warehouse_id.as_deref(),
            input.to_warehouse_id.as_deref(),
            input.movement_type,
        )?;

        let now = Utc::now().to_rfc3339();
        let movement = StockMovement {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            equipment_model_id: input.equipment_model_id.clone(),
            equipment_unit_id: input.equipment_unit_id.clone(),
            from_warehouse_id: input.from_warehouse_id.clone(),
            to_warehouse_id: input.to_warehouse_id.clone(),
            movement_type: input.movement_type,
            quantity: input.quantity,
            reason: input.reason.trim().to_string(),
            reference_id: input.reference_id.clone(),
            performed_by_user_id: session.user_id.clone(),
            created_at: now,
        };

        let mov_clone = movement.clone();
        self.run_transaction(|stores| {
            let m = mov_clone.clone();
            Box::pin(async move { stores.inventory().record_stock_movement(&m).await })
        })
        .await?;

        Ok(movement)
    }

    pub async fn list_stock_movements(
        &self,
        session: &SessionInfo,
        filter: &StockMovementFilter,
    ) -> Result<Vec<StockMovement>> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_READ)?;
        self.stores
            .inventory
            .list_stock_movements(&session.tenant_id, filter)
            .await
    }

    // ==========================================
    // 4. MÍDIAS DA UNIDADE FÍSICA (Unit Media)
    // ==========================================

    pub async fn attach_unit_media(
        &self,
        session: &SessionInfo,
        input: &AttachUnitMediaInput,
    ) -> Result<EquipmentUnitMedia> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        let unit = self
            .stores
            .inventory
            .find_unit_by_id(&session.tenant_id, &input.equipment_unit_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Unidade física não encontrada.".to_string()))?;

        let media = EquipmentUnitMedia {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            equipment_unit_id: unit.id,
            blob_id: input.blob_id.clone(),
            media_type: input.media_type,
            caption: input.caption.as_ref().map(|c| c.trim().to_string()),
            created_at: Utc::now().to_rfc3339(),
            created_by: Some(session.user_id.clone()),
        };

        let media_clone = media.clone();
        self.run_transaction(|stores| {
            let m = media_clone.clone();
            Box::pin(async move { stores.inventory().attach_unit_media(&m).await })
        })
        .await?;

        Ok(media)
    }

    pub async fn detach_unit_media(&self, session: &SessionInfo, media_id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_WAREHOUSE_OPERATE)?;
        let tenant_id = session.tenant_id.clone();
        let m_id = media_id.to_string();
        self.run_transaction(|stores| {
            let t = tenant_id.clone();
            let m = m_id.clone();
            Box::pin(async move { stores.inventory().detach_unit_media(&t, &m).await })
        })
        .await
    }

    // ==========================================
    // 5. SANITIZAÇÃO DE DADOS FINANCEIROS
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
