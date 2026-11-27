use async_trait::async_trait;
use chrono::Utc;
use eventrent_core::{
    format_patrimony_tag, CoreError, EquipmentUnit, EquipmentUnitDetails, EquipmentUnitFilter,
    EquipmentUnitListPage, InventoryStore, PhysicalCondition, Result, StockBalance, StockMovement,
    StockMovementFilter, StockMovementType, UnitOrigin, UnitStatus, Warehouse,
};
use sqlx::{QueryBuilder, Row, Sqlite};
use uuid::Uuid;

use crate::SqliteStore;

fn parse_warehouse_row(row: &sqlx::sqlite::SqliteRow) -> Warehouse {
    Warehouse {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        name: row.get("name"),
        code: row.get("code"),
        description: row.get("description"),
        address_line1: row.get("address_line1"),
        address_line2: row.get("address_line2"),
        city: row.get("city"),
        state: row.get("state"),
        postal_code: row.get("postal_code"),
        is_default: row.get::<i64, _>("is_default") != 0,
        active: row.get::<i64, _>("active") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    }
}

fn parse_equipment_unit_row(row: &sqlx::sqlite::SqliteRow) -> Result<EquipmentUnit> {
    let status_raw: String = row.get("status");
    let status = UnitStatus::parse(&status_raw)?;
    let condition_raw: String = row.get("physical_condition");
    let physical_condition = PhysicalCondition::parse(&condition_raw)?;
    let origin_raw: String = row.get("origin");
    let origin = UnitOrigin::parse(&origin_raw)?;

    Ok(EquipmentUnit {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        equipment_model_id: row.get("equipment_model_id"),
        warehouse_id: row.get("warehouse_id"),
        patrimony_tag: row.get("patrimony_tag"),
        serial_number: row.get("serial_number"),
        status,
        physical_condition,
        notes: row.get("notes"),
        acquisition_date: row.get("acquisition_date"),
        warranty_expires_at: row.get("warranty_expires_at"),
        acquisition_cost_cents: row.get("acquisition_cost_cents"),
        supplier_id: row.get("supplier_id"),
        invoice_number: row.get("invoice_number"),
        origin,
        return_by: row.get("return_by"),
        usage_hours: row.get("usage_hours"),
        location_note: row.get("location_note"),
        active: row.get::<i64, _>("active") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    })
}

fn parse_stock_balance_row(row: &sqlx::sqlite::SqliteRow) -> StockBalance {
    StockBalance {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        warehouse_id: row.get("warehouse_id"),
        equipment_model_id: row.get("equipment_model_id"),
        quantity_on_hand: row.get("quantity_on_hand"),
        quantity_maintenance: row.get("quantity_maintenance"),
        min_quantity: row.get("min_quantity"),
        updated_at: row.get("updated_at"),
    }
}

fn parse_stock_movement_row(row: &sqlx::sqlite::SqliteRow) -> Result<StockMovement> {
    let m_type_raw: String = row.get("movement_type");
    let movement_type = StockMovementType::parse(&m_type_raw)?;

    Ok(StockMovement {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        equipment_model_id: row.get("equipment_model_id"),
        equipment_unit_id: row.get("equipment_unit_id"),
        from_warehouse_id: row.get("from_warehouse_id"),
        to_warehouse_id: row.get("to_warehouse_id"),
        movement_type,
        quantity: row.get("quantity"),
        reason: row.get("reason"),
        reference_id: row.get("reference_id"),
        performed_by_user_id: row.get("performed_by_user_id"),
        created_at: row.get("created_at"),
    })
}

#[async_trait]
impl InventoryStore for SqliteStore {
    // ==========================================
    // 1. WAREHOUSES
    // ==========================================

    async fn create_warehouse(&self, warehouse: &Warehouse) -> Result<()> {
        let query = sqlx::query(
            r#"
            INSERT INTO warehouse (
                id, tenant_id, name, code, description, address_line1, address_line2,
                city, state, postal_code, is_default, active, created_at, updated_at,
                deleted_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&warehouse.id)
        .bind(&warehouse.tenant_id)
        .bind(&warehouse.name)
        .bind(&warehouse.code)
        .bind(&warehouse.description)
        .bind(&warehouse.address_line1)
        .bind(&warehouse.address_line2)
        .bind(&warehouse.city)
        .bind(&warehouse.state)
        .bind(&warehouse.postal_code)
        .bind(if warehouse.is_default { 1i64 } else { 0i64 })
        .bind(if warehouse.active { 1i64 } else { 0i64 })
        .bind(&warehouse.created_at)
        .bind(&warehouse.updated_at)
        .bind(&warehouse.deleted_at)
        .bind(&warehouse.created_by);

        self.execute(query).await.map(|_| ())
    }

    async fn update_warehouse(&self, warehouse: &Warehouse) -> Result<()> {
        let query = sqlx::query(
            r#"
            UPDATE warehouse
            SET name = ?, code = ?, description = ?, address_line1 = ?, address_line2 = ?,
                city = ?, state = ?, postal_code = ?, active = ?, updated_at = ?
            WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(&warehouse.name)
        .bind(&warehouse.code)
        .bind(&warehouse.description)
        .bind(&warehouse.address_line1)
        .bind(&warehouse.address_line2)
        .bind(&warehouse.city)
        .bind(&warehouse.state)
        .bind(&warehouse.postal_code)
        .bind(if warehouse.active { 1i64 } else { 0i64 })
        .bind(&warehouse.updated_at)
        .bind(&warehouse.tenant_id)
        .bind(&warehouse.id);

        let rows_affected = self.execute(query).await?;
        if rows_affected.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "Depósito '{}' não encontrado para atualização.",
                warehouse.id
            )));
        }
        Ok(())
    }

    async fn find_warehouse_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<Warehouse>> {
        let query = sqlx::query(
            "SELECT * FROM warehouse WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(id);

        let maybe_row = self.fetch_optional(query).await?;
        Ok(maybe_row.map(|r| parse_warehouse_row(&r)))
    }

    async fn find_warehouse_by_code(
        &self,
        tenant_id: &str,
        code: &str,
    ) -> Result<Option<Warehouse>> {
        let query = sqlx::query(
            "SELECT * FROM warehouse WHERE tenant_id = ? AND code = ? AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(code);

        let maybe_row = self.fetch_optional(query).await?;
        Ok(maybe_row.map(|r| parse_warehouse_row(&r)))
    }

    async fn get_default_warehouse(&self, tenant_id: &str) -> Result<Option<Warehouse>> {
        let query = sqlx::query(
            "SELECT * FROM warehouse WHERE tenant_id = ? AND is_default = 1 AND deleted_at IS NULL LIMIT 1",
        )
        .bind(tenant_id);

        let maybe_row = self.fetch_optional(query).await?;
        Ok(maybe_row.map(|r| parse_warehouse_row(&r)))
    }

    async fn list_warehouses(
        &self,
        tenant_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<Warehouse>> {
        let sql = if include_inactive {
            "SELECT * FROM warehouse WHERE tenant_id = ? AND deleted_at IS NULL ORDER BY is_default DESC, name ASC"
        } else {
            "SELECT * FROM warehouse WHERE tenant_id = ? AND deleted_at IS NULL AND active = 1 ORDER BY is_default DESC, name ASC"
        };
        let query = sqlx::query(sql).bind(tenant_id);
        let rows = self.fetch_all(query).await?;
        Ok(rows.iter().map(parse_warehouse_row).collect())
    }

    async fn set_default_warehouse(&self, tenant_id: &str, id: &str) -> Result<()> {
        let q_unset = sqlx::query(
            "UPDATE warehouse SET is_default = 0 WHERE tenant_id = ? AND is_default = 1",
        )
        .bind(tenant_id);
        self.execute(q_unset).await?;

        let now = Utc::now().to_rfc3339();
        let q_set = sqlx::query(
            "UPDATE warehouse SET is_default = 1, updated_at = ? WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
        )
        .bind(&now)
        .bind(tenant_id)
        .bind(id);

        let rows = self.execute(q_set).await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "Depósito '{id}' não encontrado."
            )));
        }
        Ok(())
    }

    async fn delete_warehouse(&self, tenant_id: &str, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let query = sqlx::query(
            "UPDATE warehouse SET deleted_at = ?, is_default = 0 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
        )
        .bind(&now)
        .bind(tenant_id)
        .bind(id);

        let rows = self.execute(query).await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "Depósito '{id}' não encontrado."
            )));
        }
        Ok(())
    }

    // ==========================================
    // 2. EQUIPMENT UNITS
    // ==========================================

    async fn create_unit(&self, unit: &EquipmentUnit) -> Result<()> {
        let query = sqlx::query(
            r#"
            INSERT INTO equipment_unit (
                id, tenant_id, equipment_model_id, warehouse_id, patrimony_tag, serial_number,
                status, physical_condition, notes, acquisition_date, warranty_expires_at,
                acquisition_cost_cents, supplier_id, invoice_number, origin, return_by,
                usage_hours, location_note, active, created_at, updated_at, deleted_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&unit.id)
        .bind(&unit.tenant_id)
        .bind(&unit.equipment_model_id)
        .bind(&unit.warehouse_id)
        .bind(&unit.patrimony_tag)
        .bind(&unit.serial_number)
        .bind(unit.status.as_str())
        .bind(unit.physical_condition.as_str())
        .bind(&unit.notes)
        .bind(&unit.acquisition_date)
        .bind(&unit.warranty_expires_at)
        .bind(unit.acquisition_cost_cents)
        .bind(&unit.supplier_id)
        .bind(&unit.invoice_number)
        .bind(unit.origin.as_str())
        .bind(&unit.return_by)
        .bind(unit.usage_hours)
        .bind(&unit.location_note)
        .bind(if unit.active { 1i64 } else { 0i64 })
        .bind(&unit.created_at)
        .bind(&unit.updated_at)
        .bind(&unit.deleted_at)
        .bind(&unit.created_by);

        self.execute(query).await.map(|_| ())
    }

    async fn update_unit(&self, unit: &EquipmentUnit) -> Result<()> {
        let query = sqlx::query(
            r#"
            UPDATE equipment_unit
            SET warehouse_id = ?, serial_number = ?, status = ?, physical_condition = ?,
                notes = ?, acquisition_date = ?, warranty_expires_at = ?,
                acquisition_cost_cents = ?, supplier_id = ?, invoice_number = ?,
                origin = ?, return_by = ?, usage_hours = ?, location_note = ?,
                active = ?, updated_at = ?
            WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(&unit.warehouse_id)
        .bind(&unit.serial_number)
        .bind(unit.status.as_str())
        .bind(unit.physical_condition.as_str())
        .bind(&unit.notes)
        .bind(&unit.acquisition_date)
        .bind(&unit.warranty_expires_at)
        .bind(unit.acquisition_cost_cents)
        .bind(&unit.supplier_id)
        .bind(&unit.invoice_number)
        .bind(unit.origin.as_str())
        .bind(&unit.return_by)
        .bind(unit.usage_hours)
        .bind(&unit.location_note)
        .bind(if unit.active { 1i64 } else { 0i64 })
        .bind(&unit.updated_at)
        .bind(&unit.tenant_id)
        .bind(&unit.id);

        let rows = self.execute(query).await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "Unidade física '{}' não encontrada para atualização.",
                unit.id
            )));
        }
        Ok(())
    }

    async fn find_unit_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<EquipmentUnit>> {
        let query = sqlx::query(
            "SELECT * FROM equipment_unit WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(id);

        let maybe_row = self.fetch_optional(query).await?;
        match maybe_row {
            Some(r) => parse_equipment_unit_row(&r).map(Some),
            None => Ok(None),
        }
    }

    async fn find_unit_by_patrimony(
        &self,
        tenant_id: &str,
        tag: &str,
    ) -> Result<Option<EquipmentUnit>> {
        let query = sqlx::query(
            "SELECT * FROM equipment_unit WHERE tenant_id = ? AND patrimony_tag = ? AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(tag);

        let maybe_row = self.fetch_optional(query).await?;
        match maybe_row {
            Some(r) => parse_equipment_unit_row(&r).map(Some),
            None => Ok(None),
        }
    }

    async fn list_units(
        &self,
        tenant_id: &str,
        filter: &EquipmentUnitFilter,
    ) -> Result<Vec<EquipmentUnit>> {
        let mut qb = QueryBuilder::<Sqlite>::new("SELECT * FROM equipment_unit WHERE tenant_id = ");
        qb.push_bind(tenant_id);
        qb.push(" AND deleted_at IS NULL ");

        if let Some(ref mid) = filter.equipment_model_id {
            qb.push(" AND equipment_model_id = ").push_bind(mid);
        }
        if let Some(ref wid) = filter.warehouse_id {
            qb.push(" AND warehouse_id = ").push_bind(wid);
        }
        if let Some(st) = filter.status {
            qb.push(" AND status = ").push_bind(st.as_str());
        }
        if let Some(true) = filter.active_only {
            qb.push(" AND active = 1 ");
        }
        let search_term = filter
            .search
            .as_ref()
            .map(|s| format!("%{}%", s.trim().to_uppercase()));
        if let Some(ref term) = search_term {
            qb.push(" AND (patrimony_tag LIKE ").push_bind(term);
            qb.push(" OR serial_number LIKE ")
                .push_bind(term)
                .push(") ");
        }

        qb.push(" ORDER BY patrimony_tag ASC");

        let query = qb.build();
        let rows = self.fetch_all(query).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            list.push(parse_equipment_unit_row(r)?);
        }
        Ok(list)
    }

    async fn list_unit_details(
        &self,
        tenant_id: &str,
        filter: &EquipmentUnitFilter,
    ) -> Result<EquipmentUnitListPage> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 100);
        let mut qb = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT u.id, u.tenant_id, u.equipment_model_id, u.warehouse_id, u.patrimony_tag, u.serial_number,
                   u.status, u.physical_condition, u.notes, u.acquisition_date, u.warranty_expires_at,
                   u.acquisition_cost_cents, u.supplier_id, u.invoice_number, u.origin, u.return_by,
                   u.usage_hours, u.location_note, u.active, u.created_at, u.updated_at, u.deleted_at, u.created_by,
                   m.name AS model_name,
                   w.name AS warehouse_name,
                   COALESCE(ma.thumb_blob_id, ma.blob_id) AS primary_thumb_blob_id
            FROM equipment_unit u
            INNER JOIN equipment_model m ON m.tenant_id = u.tenant_id AND m.id = u.equipment_model_id
            INNER JOIN warehouse w ON w.tenant_id = u.tenant_id AND w.id = u.warehouse_id
            LEFT JOIN equipment_model_media emm
                ON emm.tenant_id = m.tenant_id AND emm.model_id = m.id
               AND emm.is_primary = 1 AND emm.deleted_at IS NULL
            LEFT JOIN media_asset ma
                ON ma.tenant_id = emm.tenant_id AND ma.id = emm.asset_id AND ma.deleted_at IS NULL
            WHERE u.tenant_id = 
            "#,
        );
        qb.push_bind(tenant_id);
        qb.push(" AND u.deleted_at IS NULL ");

        if let Some(ref mid) = filter.equipment_model_id {
            qb.push(" AND u.equipment_model_id = ").push_bind(mid);
        }
        if let Some(ref wid) = filter.warehouse_id {
            qb.push(" AND u.warehouse_id = ").push_bind(wid);
        }
        if let Some(st) = filter.status {
            qb.push(" AND u.status = ").push_bind(st.as_str());
        }
        if let Some(true) = filter.active_only {
            qb.push(" AND u.active = 1 ");
        }

        // R7: Busca com suporte a até 8 termos em 100 caracteres
        if let Some(ref s) = filter.search {
            let s_trimmed: String = s.chars().take(100).collect();
            let terms: Vec<&str> = s_trimmed.split_whitespace().take(8).collect();
            for term in terms {
                let pattern = format!("%{}%", term.to_uppercase());
                qb.push(" AND (u.patrimony_tag LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR u.serial_number LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR UPPER(m.name) LIKE ");
                qb.push_bind(pattern);
                qb.push(") ");
            }
        }

        // Paginação estável por cursor
        if let Some(ref cur) = filter.cursor {
            qb.push(" AND u.patrimony_tag > ").push_bind(cur.clone());
        }

        qb.push(" ORDER BY u.patrimony_tag ASC LIMIT ");
        qb.push_bind((limit + 1) as i64);

        let query = qb.build();
        let rows = self.fetch_all(query).await?;
        let has_more = rows.len() > limit;
        let take_count = if has_more { limit } else { rows.len() };

        let mut items = Vec::with_capacity(take_count);
        for row in rows.iter().take(take_count) {
            let unit = parse_equipment_unit_row(row)?;
            let model_name: String = row.get("model_name");
            let warehouse_name: String = row.get("warehouse_name");
            let model_thumbnail_url: Option<String> = row.get("primary_thumb_blob_id");

            items.push(EquipmentUnitDetails {
                unit,
                model_name,
                warehouse_name,
                model_thumbnail_url,
            });
        }

        let next_cursor = if has_more {
            items.last().map(|i| i.unit.patrimony_tag.clone())
        } else {
            None
        };

        Ok(EquipmentUnitListPage {
            items,
            next_cursor,
            has_more,
        })
    }

    async fn count_unit_movements(&self, tenant_id: &str, unit_id: &str) -> Result<i64> {
        let query = sqlx::query(
            "SELECT COUNT(*) AS cnt FROM stock_movement WHERE tenant_id = ? AND equipment_unit_id = ?",
        )
        .bind(tenant_id)
        .bind(unit_id);

        let row = self.fetch_one(query).await?;
        Ok(row.get("cnt"))
    }

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

    async fn delete_unit(&self, tenant_id: &str, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let query = sqlx::query(
            "UPDATE equipment_unit SET deleted_at = ?, active = 0 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
        )
        .bind(&now)
        .bind(tenant_id)
        .bind(id);

        let rows = self.execute(query).await?;
        if rows.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!(
                "Unidade '{id}' não encontrada."
            )));
        }
        Ok(())
    }

    // ==========================================
    // 3. STOCK & BALANCES
    // ==========================================

    async fn get_stock_balance(
        &self,
        tenant_id: &str,
        warehouse_id: &str,
        equipment_model_id: &str,
    ) -> Result<Option<StockBalance>> {
        let query = sqlx::query(
            "SELECT * FROM stock_balance WHERE tenant_id = ? AND warehouse_id = ? AND equipment_model_id = ?",
        )
        .bind(tenant_id)
        .bind(warehouse_id)
        .bind(equipment_model_id);

        let maybe_row = self.fetch_optional(query).await?;
        Ok(maybe_row.map(|r| parse_stock_balance_row(&r)))
    }

    async fn list_stock_balances_by_model(
        &self,
        tenant_id: &str,
        equipment_model_id: &str,
    ) -> Result<Vec<StockBalance>> {
        let query = sqlx::query(
            "SELECT * FROM stock_balance WHERE tenant_id = ? AND equipment_model_id = ?",
        )
        .bind(tenant_id)
        .bind(equipment_model_id);

        let rows = self.fetch_all(query).await?;
        Ok(rows.iter().map(parse_stock_balance_row).collect())
    }

    async fn list_stock_balances_by_warehouse(
        &self,
        tenant_id: &str,
        warehouse_id: &str,
    ) -> Result<Vec<StockBalance>> {
        let query =
            sqlx::query("SELECT * FROM stock_balance WHERE tenant_id = ? AND warehouse_id = ?")
                .bind(tenant_id)
                .bind(warehouse_id);

        let rows = self.fetch_all(query).await?;
        Ok(rows.iter().map(parse_stock_balance_row).collect())
    }

    async fn record_stock_movement(&self, movement: &StockMovement) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // 1. Grava no livro-razão imutável
        let query = sqlx::query(
            r#"
            INSERT INTO stock_movement (
                id, tenant_id, equipment_model_id, equipment_unit_id, from_warehouse_id,
                to_warehouse_id, movement_type, quantity, reason, reference_id,
                performed_by_user_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&movement.id)
        .bind(&movement.tenant_id)
        .bind(&movement.equipment_model_id)
        .bind(&movement.equipment_unit_id)
        .bind(&movement.from_warehouse_id)
        .bind(&movement.to_warehouse_id)
        .bind(movement.movement_type.as_str())
        .bind(movement.quantity)
        .bind(&movement.reason)
        .bind(&movement.reference_id)
        .bind(&movement.performed_by_user_id)
        .bind(&movement.created_at);

        self.execute(query).await?;

        // 2. Atualiza saldos agregados em stock_balance
        // A) Origem (subtração)
        if let Some(ref from_wh) = movement.from_warehouse_id {
            let sb_id = Uuid::now_v7().to_string();
            let q_upsert = sqlx::query(
                r#"
                INSERT INTO stock_balance (id, tenant_id, warehouse_id, equipment_model_id, quantity_on_hand, quantity_maintenance, min_quantity, updated_at)
                VALUES (?, ?, ?, ?, 0, 0, 0, ?)
                ON CONFLICT(tenant_id, warehouse_id, equipment_model_id) DO NOTHING
                "#,
            )
            .bind(&sb_id)
            .bind(&movement.tenant_id)
            .bind(from_wh)
            .bind(&movement.equipment_model_id)
            .bind(&now);
            self.execute(q_upsert).await?;

            let (col_adj, delta) = match movement.movement_type {
                StockMovementType::TransferOut
                | StockMovementType::LossOut
                | StockMovementType::DecommissionOut => ("quantity_on_hand", -movement.quantity),
                StockMovementType::StatusChange => ("quantity_on_hand", 0),
                StockMovementType::Adjustment => ("quantity_on_hand", -movement.quantity),
                _ => ("quantity_on_hand", -movement.quantity),
            };

            if delta != 0 {
                let q_update_from = format!(
                    "UPDATE stock_balance SET {col_adj} = {col_adj} + ({delta}), updated_at = ? WHERE tenant_id = ? AND warehouse_id = ? AND equipment_model_id = ?"
                );
                let q_adj = sqlx::query(&q_update_from)
                    .bind(&now)
                    .bind(&movement.tenant_id)
                    .bind(from_wh)
                    .bind(&movement.equipment_model_id);
                self.execute(q_adj).await?;
            }
        }

        // B) Destino (adição)
        if let Some(ref to_wh) = movement.to_warehouse_id {
            let sb_id = Uuid::now_v7().to_string();
            let q_upsert = sqlx::query(
                r#"
                INSERT INTO stock_balance (id, tenant_id, warehouse_id, equipment_model_id, quantity_on_hand, quantity_maintenance, min_quantity, updated_at)
                VALUES (?, ?, ?, ?, 0, 0, 0, ?)
                ON CONFLICT(tenant_id, warehouse_id, equipment_model_id) DO NOTHING
                "#,
            )
            .bind(&sb_id)
            .bind(&movement.tenant_id)
            .bind(to_wh)
            .bind(&movement.equipment_model_id)
            .bind(&now);
            self.execute(q_upsert).await?;

            let (col_adj, delta) = match movement.movement_type {
                StockMovementType::PurchaseIn
                | StockMovementType::ReturnIn
                | StockMovementType::TransferIn => ("quantity_on_hand", movement.quantity),
                StockMovementType::StatusChange => ("quantity_on_hand", 0),
                StockMovementType::Adjustment => ("quantity_on_hand", movement.quantity),
                _ => ("quantity_on_hand", movement.quantity),
            };

            if delta != 0 {
                let q_update_to = format!(
                    "UPDATE stock_balance SET {col_adj} = {col_adj} + ({delta}), updated_at = ? WHERE tenant_id = ? AND warehouse_id = ? AND equipment_model_id = ?"
                );
                let q_adj = sqlx::query(&q_update_to)
                    .bind(&now)
                    .bind(&movement.tenant_id)
                    .bind(to_wh)
                    .bind(&movement.equipment_model_id);
                self.execute(q_adj).await?;
            }
        }

        Ok(())
    }

    async fn list_stock_movements(
        &self,
        tenant_id: &str,
        filter: &StockMovementFilter,
    ) -> Result<Vec<StockMovement>> {
        let mut qb = QueryBuilder::<Sqlite>::new("SELECT * FROM stock_movement WHERE tenant_id = ");
        qb.push_bind(tenant_id);

        if let Some(ref mid) = filter.equipment_model_id {
            qb.push(" AND equipment_model_id = ").push_bind(mid);
        }
        if let Some(ref uid) = filter.equipment_unit_id {
            qb.push(" AND equipment_unit_id = ").push_bind(uid);
        }
        if let Some(ref wid) = filter.warehouse_id {
            qb.push(" AND (from_warehouse_id = ").push_bind(wid);
            qb.push(" OR to_warehouse_id = ").push_bind(wid).push(") ");
        }
        if let Some(mt) = filter.movement_type {
            qb.push(" AND movement_type = ").push_bind(mt.as_str());
        }

        qb.push(" ORDER BY created_at DESC");
        let limit = filter.limit.unwrap_or(100).min(500);
        qb.push(" LIMIT ").push_bind(limit as i64);

        let query = qb.build();
        let rows = self.fetch_all(query).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            list.push(parse_stock_movement_row(r)?);
        }
        Ok(list)
    }
}
