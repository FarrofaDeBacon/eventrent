use async_trait::async_trait;
use eventrent_core::{
    normalize_for_search, normalize_unique_key, AttributeDataType, AttributeDefinition, Brand,
    CatalogCursor, CatalogListFilter, CatalogListResult, CatalogStore, ControlMode, CoreError,
    EquipmentCategory, EquipmentModel, EquipmentModelAttributeValue, EquipmentModelListItem,
    Result, VoltageOption,
};
use sqlx::{QueryBuilder, Row, Sqlite};

use crate::{map_sqlx_err, SqliteStore};

fn parse_brand_row(row: &sqlx::sqlite::SqliteRow) -> Brand {
    Brand {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        name: row.get("name"),
        name_normalized: row.get("name_normalized"),
        active: row.get::<i64, _>("active") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    }
}

fn parse_category_row(row: &sqlx::sqlite::SqliteRow) -> EquipmentCategory {
    EquipmentCategory {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        parent_id: row.get("parent_id"),
        name: row.get("name"),
        name_normalized: row.get("name_normalized"),
        description: row.get("description"),
        sort_order: row.get("sort_order"),
        active: row.get::<i64, _>("active") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    }
}

fn parse_attribute_definition_row(row: &sqlx::sqlite::SqliteRow) -> Result<AttributeDefinition> {
    let data_type_raw: String = row.get("data_type");
    let data_type = AttributeDataType::parse(&data_type_raw)?;
    let choices_json: Option<String> = row.get("choices_json");
    let choices: Vec<String> = match choices_json {
        Some(json) if !json.trim().is_empty() => serde_json::from_str(&json)
            .map_err(|e| CoreError::Internal(format!("choices_json inválido: {e}")))?,
        _ => Vec::new(),
    };

    Ok(AttributeDefinition {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        category_id: row.get("category_id"),
        field_key: row.get("field_key"),
        label: row.get("label"),
        data_type,
        unit: row.get("unit"),
        choices,
        is_required: row.get::<i64, _>("is_required") != 0,
        sort_order: row.get("sort_order"),
        active: row.get::<i64, _>("active") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    })
}

fn parse_model_row(row: &sqlx::sqlite::SqliteRow) -> Result<EquipmentModel> {
    let control_mode_raw: String = row.get("control_mode");
    let control_mode = ControlMode::parse(&control_mode_raw)?;
    let voltage_raw: Option<String> = row.get("voltage");
    let voltage = match voltage_raw {
        Some(v) => Some(VoltageOption::parse(&v)?),
        None => None,
    };

    Ok(EquipmentModel {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        category_id: row.get("category_id"),
        brand_id: row.get("brand_id"),
        name: row.get("name"),
        name_sort: row.get("name_sort"),
        model_code: row.get("model_code"),
        sku: row.get("sku"),
        ean: row.get("ean"),
        ncm: row.get("ncm"),
        description: row.get("description"),
        control_mode,
        weight_kg: row.get("weight_kg"),
        length_cm: row.get("length_cm"),
        width_cm: row.get("width_cm"),
        height_cm: row.get("height_cm"),
        power_watts: row.get("power_watts"),
        voltage,
        amperage: row.get("amperage"),
        requires_case: row.get::<i64, _>("requires_case") != 0,
        units_per_case: row.get("units_per_case"),
        daily_rate_cents: row.get("daily_rate_cents"),
        replacement_value_cents: row.get("replacement_value_cents"),
        cost_price_cents: row.get("cost_price_cents"),
        active: row.get::<i64, _>("active") != 0,
        search_text: row.get("search_text"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    })
}

fn parse_attr_value_row(row: &sqlx::sqlite::SqliteRow) -> EquipmentModelAttributeValue {
    let value_bool_raw: Option<i64> = row.get("value_bool");
    EquipmentModelAttributeValue {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        model_id: row.get("model_id"),
        definition_id: row.get("definition_id"),
        value_text: row.get("value_text"),
        value_number: row.get("value_number"),
        value_bool: value_bool_raw.map(|v| v != 0),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
        created_by: row.get("created_by"),
    }
}

#[async_trait]
impl CatalogStore for SqliteStore {
    async fn list_brands(&self, tenant_id: &str, include_inactive: bool) -> Result<Vec<Brand>> {
        let rows = if include_inactive {
            self.fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, name, name_normalized, active, created_at, updated_at, deleted_at, created_by
                     FROM brand WHERE tenant_id = ? AND deleted_at IS NULL
                     ORDER BY name_normalized ASC",
                )
                .bind(tenant_id),
            )
            .await?
        } else {
            self.fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, name, name_normalized, active, created_at, updated_at, deleted_at, created_by
                     FROM brand WHERE tenant_id = ? AND deleted_at IS NULL AND active = 1
                     ORDER BY name_normalized ASC",
                )
                .bind(tenant_id),
            )
            .await?
        };
        Ok(rows.iter().map(parse_brand_row).collect())
    }

    async fn find_brand_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<Brand>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, name, name_normalized, active, created_at, updated_at, deleted_at, created_by
                     FROM brand WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;
        Ok(row.as_ref().map(parse_brand_row))
    }

    async fn find_brand_by_name_normalized(
        &self,
        tenant_id: &str,
        name_normalized: &str,
    ) -> Result<Option<Brand>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, name, name_normalized, active, created_at, updated_at, deleted_at, created_by
                     FROM brand WHERE tenant_id = ? AND name_normalized = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(name_normalized),
            )
            .await?;
        Ok(row.as_ref().map(parse_brand_row))
    }

    async fn create_brand(&self, brand: &Brand) -> Result<()> {
        self.execute(
            sqlx::query(
                "INSERT INTO brand (id, tenant_id, name, name_normalized, active, created_at, updated_at, deleted_at, created_by)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&brand.id)
            .bind(&brand.tenant_id)
            .bind(&brand.name)
            .bind(&brand.name_normalized)
            .bind(if brand.active { 1 } else { 0 })
            .bind(&brand.created_at)
            .bind(&brand.updated_at)
            .bind(&brand.deleted_at)
            .bind(&brand.created_by),
        )
        .await?;
        Ok(())
    }

    async fn update_brand(&self, brand: &Brand) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE brand SET name = ?, name_normalized = ?, active = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(&brand.name)
            .bind(&brand.name_normalized)
            .bind(if brand.active { 1 } else { 0 })
            .bind(&brand.updated_at)
            .bind(&brand.tenant_id)
            .bind(&brand.id),
        )
        .await?;
        Ok(())
    }

    async fn soft_delete_brand(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE brand SET deleted_at = ?, updated_at = ? WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn count_models_by_brand(&self, tenant_id: &str, brand_id: &str) -> Result<i64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) AS cnt FROM equipment_model WHERE tenant_id = ? AND brand_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(brand_id),
            )
            .await?;
        Ok(row.get("cnt"))
    }

    async fn list_categories(
        &self,
        tenant_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<EquipmentCategory>> {
        let rows = if include_inactive {
            self.fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, parent_id, name, name_normalized, description, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM equipment_category WHERE tenant_id = ? AND deleted_at IS NULL
                     ORDER BY sort_order ASC, name_normalized ASC",
                )
                .bind(tenant_id),
            )
            .await?
        } else {
            self.fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, parent_id, name, name_normalized, description, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM equipment_category WHERE tenant_id = ? AND deleted_at IS NULL AND active = 1
                     ORDER BY sort_order ASC, name_normalized ASC",
                )
                .bind(tenant_id),
            )
            .await?
        };
        Ok(rows.iter().map(parse_category_row).collect())
    }

    async fn find_category_by_id(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<EquipmentCategory>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, parent_id, name, name_normalized, description, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM equipment_category WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;
        Ok(row.as_ref().map(parse_category_row))
    }

    async fn find_category_by_parent_and_name(
        &self,
        tenant_id: &str,
        parent_id: Option<&str>,
        name_normalized: &str,
    ) -> Result<Option<EquipmentCategory>> {
        let parent_key = parent_id.unwrap_or("");
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, parent_id, name, name_normalized, description, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM equipment_category
                     WHERE tenant_id = ? AND COALESCE(parent_id, '') = ? AND name_normalized = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(parent_key)
                .bind(name_normalized),
            )
            .await?;
        Ok(row.as_ref().map(parse_category_row))
    }

    async fn create_category(&self, category: &EquipmentCategory) -> Result<()> {
        self.execute(
            sqlx::query(
                "INSERT INTO equipment_category (id, tenant_id, parent_id, name, name_normalized, description, sort_order, active, created_at, updated_at, deleted_at, created_by)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&category.id)
            .bind(&category.tenant_id)
            .bind(&category.parent_id)
            .bind(&category.name)
            .bind(&category.name_normalized)
            .bind(&category.description)
            .bind(category.sort_order)
            .bind(if category.active { 1 } else { 0 })
            .bind(&category.created_at)
            .bind(&category.updated_at)
            .bind(&category.deleted_at)
            .bind(&category.created_by),
        )
        .await?;
        Ok(())
    }

    async fn update_category(&self, category: &EquipmentCategory) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE equipment_category
                 SET parent_id = ?, name = ?, name_normalized = ?, description = ?, sort_order = ?, active = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(&category.parent_id)
            .bind(&category.name)
            .bind(&category.name_normalized)
            .bind(&category.description)
            .bind(category.sort_order)
            .bind(if category.active { 1 } else { 0 })
            .bind(&category.updated_at)
            .bind(&category.tenant_id)
            .bind(&category.id),
        )
        .await?;
        Ok(())
    }

    async fn soft_delete_category(
        &self,
        tenant_id: &str,
        id: &str,
        deleted_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE equipment_category SET deleted_at = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn count_child_categories(&self, tenant_id: &str, category_id: &str) -> Result<i64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) AS cnt FROM equipment_category WHERE tenant_id = ? AND parent_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(category_id),
            )
            .await?;
        Ok(row.get("cnt"))
    }

    async fn count_models_by_category(&self, tenant_id: &str, category_id: &str) -> Result<i64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) AS cnt FROM equipment_model WHERE tenant_id = ? AND category_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(category_id),
            )
            .await?;
        Ok(row.get("cnt"))
    }

    async fn count_models_per_category(&self, tenant_id: &str) -> Result<Vec<(String, i64)>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT category_id, COUNT(*) AS cnt FROM equipment_model
                     WHERE tenant_id = ? AND deleted_at IS NULL
                     GROUP BY category_id",
                )
                .bind(tenant_id),
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("category_id"), r.get("cnt")))
            .collect())
    }

    async fn list_all_attribute_definitions(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<AttributeDefinition>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, category_id, field_key, label, data_type, unit, choices_json, is_required, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM attribute_definition
                     WHERE tenant_id = ? AND deleted_at IS NULL
                     ORDER BY sort_order ASC, label ASC",
                )
                .bind(tenant_id),
            )
            .await?;
        rows.iter().map(parse_attribute_definition_row).collect()
    }

    async fn list_attribute_definitions_by_category(
        &self,
        tenant_id: &str,
        category_id: &str,
    ) -> Result<Vec<AttributeDefinition>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, category_id, field_key, label, data_type, unit, choices_json, is_required, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM attribute_definition
                     WHERE tenant_id = ? AND category_id = ? AND deleted_at IS NULL
                     ORDER BY sort_order ASC, label ASC",
                )
                .bind(tenant_id)
                .bind(category_id),
            )
            .await?;
        rows.iter().map(parse_attribute_definition_row).collect()
    }

    async fn find_attribute_definition_by_id(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<AttributeDefinition>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, category_id, field_key, label, data_type, unit, choices_json, is_required, sort_order, active, created_at, updated_at, deleted_at, created_by
                     FROM attribute_definition
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;
        row.as_ref().map(parse_attribute_definition_row).transpose()
    }

    async fn create_attribute_definition(&self, def: &AttributeDefinition) -> Result<()> {
        let choices_json = if def.choices.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&def.choices)
                    .map_err(|e| CoreError::Internal(format!("Erro ao serializar opções: {e}")))?,
            )
        };
        self.execute(
            sqlx::query(
                "INSERT INTO attribute_definition (id, tenant_id, category_id, field_key, label, data_type, unit, choices_json, is_required, sort_order, active, created_at, updated_at, deleted_at, created_by)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&def.id)
            .bind(&def.tenant_id)
            .bind(&def.category_id)
            .bind(&def.field_key)
            .bind(&def.label)
            .bind(def.data_type.as_str())
            .bind(&def.unit)
            .bind(choices_json)
            .bind(if def.is_required { 1 } else { 0 })
            .bind(def.sort_order)
            .bind(if def.active { 1 } else { 0 })
            .bind(&def.created_at)
            .bind(&def.updated_at)
            .bind(&def.deleted_at)
            .bind(&def.created_by),
        )
        .await?;
        Ok(())
    }

    async fn update_attribute_definition(&self, def: &AttributeDefinition) -> Result<()> {
        let choices_json = if def.choices.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&def.choices)
                    .map_err(|e| CoreError::Internal(format!("Erro ao serializar opções: {e}")))?,
            )
        };
        self.execute(
            sqlx::query(
                "UPDATE attribute_definition
                 SET field_key = ?, label = ?, data_type = ?, unit = ?, choices_json = ?, is_required = ?, sort_order = ?, active = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(&def.field_key)
            .bind(&def.label)
            .bind(def.data_type.as_str())
            .bind(&def.unit)
            .bind(choices_json)
            .bind(if def.is_required { 1 } else { 0 })
            .bind(def.sort_order)
            .bind(if def.active { 1 } else { 0 })
            .bind(&def.updated_at)
            .bind(&def.tenant_id)
            .bind(&def.id),
        )
        .await?;
        Ok(())
    }

    async fn soft_delete_attribute_definition(
        &self,
        tenant_id: &str,
        id: &str,
        deleted_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE attribute_definition SET deleted_at = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn count_models_using_attribute_definition(
        &self,
        tenant_id: &str,
        definition_id: &str,
    ) -> Result<i64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(DISTINCT v.model_id) AS cnt
                     FROM equipment_model_attribute_value v
                     INNER JOIN equipment_model m ON m.tenant_id = v.tenant_id AND m.id = v.model_id
                     WHERE v.tenant_id = ? AND v.definition_id = ?
                       AND v.deleted_at IS NULL AND m.deleted_at IS NULL
                       AND (v.value_text IS NOT NULL OR v.value_number IS NOT NULL OR v.value_bool IS NOT NULL)",
                )
                .bind(tenant_id)
                .bind(definition_id),
            )
            .await?;
        Ok(row.get("cnt"))
    }

    async fn list_distinct_text_values_for_attribute_definition(
        &self,
        tenant_id: &str,
        definition_id: &str,
    ) -> Result<Vec<String>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT DISTINCT v.value_text AS val
                     FROM equipment_model_attribute_value v
                     INNER JOIN equipment_model m ON m.tenant_id = v.tenant_id AND m.id = v.model_id
                     WHERE v.tenant_id = ? AND v.definition_id = ?
                       AND v.deleted_at IS NULL AND m.deleted_at IS NULL
                       AND v.value_text IS NOT NULL
                     ORDER BY v.value_text ASC",
                )
                .bind(tenant_id)
                .bind(definition_id),
            )
            .await?;
        Ok(rows.iter().map(|r| r.get("val")).collect())
    }

    async fn list_models(
        &self,
        tenant_id: &str,
        filter: &CatalogListFilter,
        descendant_category_ids: Option<&[String]>,
    ) -> Result<CatalogListResult> {
        let limit = filter.limit.clamp(1, 200);
        let cursor = match filter.cursor.as_deref() {
            Some(c) if !c.trim().is_empty() => Some(CatalogCursor::decode(c)?),
            _ => None,
        };

        let terms: Vec<String> = filter
            .query
            .as_deref()
            .map(normalize_for_search)
            .map(|s| s.split_whitespace().map(|t| t.to_string()).collect())
            .unwrap_or_default();

        let apply_filters = |qb: &mut QueryBuilder<Sqlite>| {
            qb.push(" WHERE m.tenant_id = ");
            qb.push_bind(tenant_id.to_string());
            qb.push(" AND m.deleted_at IS NULL");

            if let Some(active) = filter.active {
                qb.push(" AND m.active = ");
                qb.push_bind(if active { 1i64 } else { 0i64 });
            }
            if let Some(ref bid) = filter.brand_id {
                if !bid.trim().is_empty() {
                    qb.push(" AND m.brand_id = ");
                    qb.push_bind(bid.clone());
                }
            }
            if let Some(mode) = filter.control_mode {
                qb.push(" AND m.control_mode = ");
                qb.push_bind(mode.as_str().to_string());
            }
            if let Some(cat_ids) = descendant_category_ids {
                if cat_ids.is_empty() {
                    qb.push(" AND 1 = 0");
                } else {
                    qb.push(" AND m.category_id IN (");
                    let mut sep = qb.separated(", ");
                    for cid in cat_ids {
                        sep.push_bind(cid.clone());
                    }
                    qb.push(")");
                }
            }
            for term in &terms {
                qb.push(" AND m.search_text LIKE ");
                qb.push_bind(format!("%{}%", term));
            }
        };

        let mut count_qb =
            QueryBuilder::<Sqlite>::new("SELECT COUNT(*) AS cnt FROM equipment_model m");
        apply_filters(&mut count_qb);

        let total_count: i64 = match &self.executor {
            crate::SqliteExecutor::Pool(pool) => {
                let row = count_qb
                    .build()
                    .fetch_one(pool)
                    .await
                    .map_err(map_sqlx_err)?;
                row.get("cnt")
            }
            crate::SqliteExecutor::Tx(tx_mutex) => {
                let mut guard = tx_mutex.lock().await;
                let tx = guard
                    .as_mut()
                    .ok_or_else(|| CoreError::Internal("Transação já finalizada.".to_string()))?;
                let row = count_qb
                    .build()
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(map_sqlx_err)?;
                row.get("cnt")
            }
        };

        let mut list_qb = QueryBuilder::<Sqlite>::new(
            "SELECT m.id, m.tenant_id, m.category_id, m.brand_id, m.name, m.name_sort,
                    m.model_code, m.sku, m.ean, m.ncm, m.description, m.control_mode,
                    m.weight_kg, m.length_cm, m.width_cm, m.height_cm, m.power_watts,
                    m.voltage, m.amperage, m.requires_case, m.units_per_case,
                    m.daily_rate_cents, m.replacement_value_cents, m.cost_price_cents,
                    m.active, m.search_text, m.created_at, m.updated_at, m.deleted_at, m.created_by,
                    c.name AS category_name, b.name AS brand_name
             FROM equipment_model m
             INNER JOIN equipment_category c ON c.tenant_id = m.tenant_id AND c.id = m.category_id
             LEFT JOIN brand b ON b.tenant_id = m.tenant_id AND b.id = m.brand_id",
        );
        apply_filters(&mut list_qb);

        if let Some(ref cur) = cursor {
            list_qb.push(" AND (m.name_sort > ");
            list_qb.push_bind(cur.name_sort.clone());
            list_qb.push(" OR (m.name_sort = ");
            list_qb.push_bind(cur.name_sort.clone());
            list_qb.push(" AND m.id > ");
            list_qb.push_bind(cur.id.clone());
            list_qb.push("))");
        }

        list_qb.push(" ORDER BY m.name_sort ASC, m.id ASC LIMIT ");
        list_qb.push_bind((limit + 1) as i64);

        let rows = match &self.executor {
            crate::SqliteExecutor::Pool(pool) => list_qb
                .build()
                .fetch_all(pool)
                .await
                .map_err(map_sqlx_err)?,
            crate::SqliteExecutor::Tx(tx_mutex) => {
                let mut guard = tx_mutex.lock().await;
                let tx = guard
                    .as_mut()
                    .ok_or_else(|| CoreError::Internal("Transação já finalizada.".to_string()))?;
                list_qb
                    .build()
                    .fetch_all(&mut **tx)
                    .await
                    .map_err(map_sqlx_err)?
            }
        };

        let has_more = rows.len() > limit;
        let slice = if has_more { &rows[..limit] } else { &rows[..] };
        let mut items = Vec::with_capacity(slice.len());
        for row in slice {
            let model = parse_model_row(row)?;
            let category_name: String = row.get("category_name");
            let brand_name: Option<String> = row.get("brand_name");
            items.push(EquipmentModelListItem {
                model,
                category_name,
                brand_name,
            });
        }

        let next_cursor = if has_more {
            items
                .last()
                .map(|last| CatalogCursor::new(&last.model.name_sort, &last.model.id).encode())
        } else {
            None
        };

        Ok(CatalogListResult {
            items,
            next_cursor,
            total_count,
        })
    }

    async fn find_model_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<EquipmentModel>> {
        let row = self
            .fetch_optional(
                sqlx::query(
                    "SELECT id, tenant_id, category_id, brand_id, name, name_sort,
                            model_code, sku, ean, ncm, description, control_mode,
                            weight_kg, length_cm, width_cm, height_cm, power_watts,
                            voltage, amperage, requires_case, units_per_case,
                            daily_rate_cents, replacement_value_cents, cost_price_cents,
                            active, search_text, created_at, updated_at, deleted_at, created_by
                     FROM equipment_model
                     WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(id),
            )
            .await?;
        row.as_ref().map(parse_model_row).transpose()
    }

    async fn list_model_attribute_values(
        &self,
        tenant_id: &str,
        model_id: &str,
    ) -> Result<Vec<EquipmentModelAttributeValue>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, model_id, definition_id, value_text, value_number, value_bool, created_at, updated_at, deleted_at, created_by
                     FROM equipment_model_attribute_value
                     WHERE tenant_id = ? AND model_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(model_id),
            )
            .await?;
        Ok(rows.iter().map(parse_attr_value_row).collect())
    }

    async fn create_model(
        &self,
        model: &EquipmentModel,
        values: &[EquipmentModelAttributeValue],
    ) -> Result<()> {
        let model_code_norm = model.model_code.as_deref().map(normalize_unique_key);
        let sku_norm = model.sku.as_deref().map(normalize_unique_key);
        self.execute(
            sqlx::query(
                "INSERT INTO equipment_model (
                    id, tenant_id, category_id, brand_id, name, name_sort,
                    model_code, model_code_normalized, sku, sku_normalized, ean, ncm, description,
                    control_mode, weight_kg, length_cm, width_cm, height_cm, power_watts,
                    voltage, amperage, requires_case, units_per_case,
                    daily_rate_cents, replacement_value_cents, cost_price_cents,
                    active, search_text, created_at, updated_at, deleted_at, created_by
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&model.id)
            .bind(&model.tenant_id)
            .bind(&model.category_id)
            .bind(&model.brand_id)
            .bind(&model.name)
            .bind(&model.name_sort)
            .bind(&model.model_code)
            .bind(model_code_norm)
            .bind(&model.sku)
            .bind(sku_norm)
            .bind(&model.ean)
            .bind(&model.ncm)
            .bind(&model.description)
            .bind(model.control_mode.as_str())
            .bind(model.weight_kg)
            .bind(model.length_cm)
            .bind(model.width_cm)
            .bind(model.height_cm)
            .bind(model.power_watts)
            .bind(model.voltage.map(|v| v.as_str()))
            .bind(model.amperage)
            .bind(if model.requires_case { 1 } else { 0 })
            .bind(model.units_per_case)
            .bind(model.daily_rate_cents)
            .bind(model.replacement_value_cents)
            .bind(model.cost_price_cents)
            .bind(if model.active { 1 } else { 0 })
            .bind(&model.search_text)
            .bind(&model.created_at)
            .bind(&model.updated_at)
            .bind(&model.deleted_at)
            .bind(&model.created_by),
        )
        .await?;

        for val in values {
            self.execute(
                sqlx::query(
                    "INSERT INTO equipment_model_attribute_value (
                        id, tenant_id, model_id, definition_id, value_text, value_number, value_bool, created_at, updated_at, deleted_at, created_by
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&val.id)
                .bind(&val.tenant_id)
                .bind(&val.model_id)
                .bind(&val.definition_id)
                .bind(&val.value_text)
                .bind(val.value_number)
                .bind(val.value_bool.map(|b| if b { 1 } else { 0 }))
                .bind(&val.created_at)
                .bind(&val.updated_at)
                .bind(&val.deleted_at)
                .bind(&val.created_by),
            )
            .await?;
        }
        Ok(())
    }

    async fn update_model(
        &self,
        model: &EquipmentModel,
        values: &[EquipmentModelAttributeValue],
    ) -> Result<()> {
        let model_code_norm = model.model_code.as_deref().map(normalize_unique_key);
        let sku_norm = model.sku.as_deref().map(normalize_unique_key);
        self.execute(
            sqlx::query(
                "UPDATE equipment_model SET
                    category_id = ?, brand_id = ?, name = ?, name_sort = ?,
                    model_code = ?, model_code_normalized = ?, sku = ?, sku_normalized = ?,
                    ean = ?, ncm = ?, description = ?, control_mode = ?,
                    weight_kg = ?, length_cm = ?, width_cm = ?, height_cm = ?,
                    power_watts = ?, voltage = ?, amperage = ?,
                    requires_case = ?, units_per_case = ?,
                    daily_rate_cents = ?, replacement_value_cents = ?, cost_price_cents = ?,
                    active = ?, search_text = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(&model.category_id)
            .bind(&model.brand_id)
            .bind(&model.name)
            .bind(&model.name_sort)
            .bind(&model.model_code)
            .bind(model_code_norm)
            .bind(&model.sku)
            .bind(sku_norm)
            .bind(&model.ean)
            .bind(&model.ncm)
            .bind(&model.description)
            .bind(model.control_mode.as_str())
            .bind(model.weight_kg)
            .bind(model.length_cm)
            .bind(model.width_cm)
            .bind(model.height_cm)
            .bind(model.power_watts)
            .bind(model.voltage.map(|v| v.as_str()))
            .bind(model.amperage)
            .bind(if model.requires_case { 1 } else { 0 })
            .bind(model.units_per_case)
            .bind(model.daily_rate_cents)
            .bind(model.replacement_value_cents)
            .bind(model.cost_price_cents)
            .bind(if model.active { 1 } else { 0 })
            .bind(&model.search_text)
            .bind(&model.updated_at)
            .bind(&model.tenant_id)
            .bind(&model.id),
        )
        .await?;

        self.execute(
            sqlx::query(
                "UPDATE equipment_model_attribute_value SET deleted_at = ?, updated_at = ?
                 WHERE tenant_id = ? AND model_id = ? AND deleted_at IS NULL",
            )
            .bind(&model.updated_at)
            .bind(&model.updated_at)
            .bind(&model.tenant_id)
            .bind(&model.id),
        )
        .await?;

        for val in values {
            self.execute(
                sqlx::query(
                    "INSERT INTO equipment_model_attribute_value (
                        id, tenant_id, model_id, definition_id, value_text, value_number, value_bool, created_at, updated_at, deleted_at, created_by
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&val.id)
                .bind(&val.tenant_id)
                .bind(&val.model_id)
                .bind(&val.definition_id)
                .bind(&val.value_text)
                .bind(val.value_number)
                .bind(val.value_bool.map(|b| if b { 1 } else { 0 }))
                .bind(&val.created_at)
                .bind(&val.updated_at)
                .bind(&val.deleted_at)
                .bind(&val.created_by),
            )
            .await?;
        }
        Ok(())
    }

    async fn set_model_active(
        &self,
        tenant_id: &str,
        id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE equipment_model SET active = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(if active { 1 } else { 0 })
            .bind(updated_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn soft_delete_model(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE equipment_model SET deleted_at = ?, updated_at = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(deleted_at)
            .bind(deleted_at)
            .bind(tenant_id)
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn count_active_models(&self, tenant_id: &str) -> Result<u64> {
        let row = self
            .fetch_one(
                sqlx::query(
                    "SELECT COUNT(*) AS cnt FROM equipment_model WHERE tenant_id = ? AND deleted_at IS NULL AND active = 1",
                )
                .bind(tenant_id),
            )
            .await?;
        let cnt: i64 = row.get("cnt");
        Ok(cnt.max(0) as u64)
    }

    async fn list_models_by_brand(
        &self,
        tenant_id: &str,
        brand_id: &str,
    ) -> Result<Vec<EquipmentModel>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, category_id, brand_id, name, name_sort,
                            model_code, sku, ean, ncm, description, control_mode,
                            weight_kg, length_cm, width_cm, height_cm, power_watts,
                            voltage, amperage, requires_case, units_per_case,
                            daily_rate_cents, replacement_value_cents, cost_price_cents,
                            active, search_text, created_at, updated_at, deleted_at, created_by
                     FROM equipment_model
                     WHERE tenant_id = ? AND brand_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(brand_id),
            )
            .await?;
        rows.iter().map(parse_model_row).collect()
    }

    async fn list_models_by_category(
        &self,
        tenant_id: &str,
        category_id: &str,
    ) -> Result<Vec<EquipmentModel>> {
        let rows = self
            .fetch_all(
                sqlx::query(
                    "SELECT id, tenant_id, category_id, brand_id, name, name_sort,
                            model_code, sku, ean, ncm, description, control_mode,
                            weight_kg, length_cm, width_cm, height_cm, power_watts,
                            voltage, amperage, requires_case, units_per_case,
                            daily_rate_cents, replacement_value_cents, cost_price_cents,
                            active, search_text, created_at, updated_at, deleted_at, created_by
                     FROM equipment_model
                     WHERE tenant_id = ? AND category_id = ? AND deleted_at IS NULL",
                )
                .bind(tenant_id)
                .bind(category_id),
            )
            .await?;
        rows.iter().map(parse_model_row).collect()
    }

    async fn update_model_search_text(
        &self,
        tenant_id: &str,
        model_id: &str,
        search_text: &str,
    ) -> Result<()> {
        self.execute(
            sqlx::query(
                "UPDATE equipment_model SET search_text = ?
                 WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
            )
            .bind(search_text)
            .bind(tenant_id)
            .bind(model_id),
        )
        .await?;
        Ok(())
    }
}
