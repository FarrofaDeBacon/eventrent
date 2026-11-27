use chrono::Utc;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::domain::catalog::{
    build_catalog_search_text, normalize_unique_key, resolve_inherited_attributes,
    validate_attribute_definition_fields, validate_attribute_key_in_hierarchy, validate_brand_name,
    validate_category_fields, validate_category_move_attribute_keys,
    validate_category_tree_placement, validate_equipment_model_fields,
    validate_model_attribute_values, AttributeDataType, AttributeDefinition, Brand,
    CatalogListFilter, CatalogListResult, CreateAttributeDefinitionInput, CreateBrandInput,
    CreateCategoryInput, CreateEquipmentModelInput, EquipmentCategory, EquipmentCategoryTreeItem,
    EquipmentModel, EquipmentModelAttributeValue, EquipmentModelWithDetails,
    InheritedAttributeDefinition, SuggestedCategoriesResult, UpdateAttributeDefinitionInput,
    UpdateBrandInput, UpdateCategoryInput, UpdateEquipmentModelInput, MAX_ATTRIBUTES_PER_CATEGORY,
};
use crate::domain::inventory::can_manage_financials;
use crate::domain::permission::{PERM_CATALOG_MANAGE, PERM_CATALOG_READ};
use crate::domain::user::SessionInfo;
use crate::error::{CoreError, Result};
use crate::services::{build_audit_entry, validate_search_query_limits, CoreServices};

impl CoreServices {
    pub async fn list_brands(
        &self,
        session: &SessionInfo,
        include_inactive: bool,
    ) -> Result<Vec<Brand>> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        self.stores
            .catalog
            .list_brands(&session.tenant_id, include_inactive)
            .await
    }

    pub async fn create_brand(
        &self,
        session: &SessionInfo,
        input: &CreateBrandInput,
    ) -> Result<Brand> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let (clean_name, normalized) = validate_brand_name(&input.name)?;
        let now = Utc::now().to_rfc3339();
        let brand = Brand {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            name: clean_name.clone(),
            name_normalized: normalized.clone(),
            active: input.active,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let session_clone = session.clone();
        let brand_clone = brand.clone();
        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let brand = brand_clone.clone();
            Box::pin(async move {
                if stores
                    .catalog()
                    .find_brand_by_name_normalized(&session.tenant_id, &brand.name_normalized)
                    .await?
                    .is_some()
                {
                    return Err(CoreError::Conflict(format!(
                        "Já existe uma marca cadastrada com o nome '{}'.",
                        brand.name
                    )));
                }

                stores.catalog().create_brand(&brand).await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.brand.created",
                        "brand",
                        &brand.id,
                        &format!("Marca '{}' criada no catálogo.", brand.name),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await?;

        Ok(brand)
    }

    pub async fn update_brand(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateBrandInput,
    ) -> Result<Brand> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let (clean_name, normalized) = validate_brand_name(&input.name)?;
        let now = Utc::now().to_rfc3339();

        let session_clone = session.clone();
        let id_str = id.to_string();
        let active = input.active;
        let expected_updated_at = input.expected_updated_at.clone();

        let updated = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let id = id_str.clone();
                let clean_name = clean_name.clone();
                let normalized = normalized.clone();
                let now = now.clone();
                let expected_updated_at = expected_updated_at.clone();
                Box::pin(async move {
                    let existing = stores
                        .catalog()
                        .find_brand_by_id(&session.tenant_id, &id)
                        .await?
                        .ok_or_else(|| CoreError::NotFound("Marca não encontrada.".to_string()))?;

                    if existing.updated_at != expected_updated_at {
                        return Err(CoreError::Conflict(
                            "A marca foi alterada por outra pessoa. Recarregue e tente novamente."
                                .to_string(),
                        ));
                    }

                    if let Some(other) = stores
                        .catalog()
                        .find_brand_by_name_normalized(&session.tenant_id, &normalized)
                        .await?
                    {
                        if other.id != existing.id {
                            return Err(CoreError::Conflict(format!(
                                "Já existe outra marca cadastrada com o nome '{}'.",
                                clean_name
                            )));
                        }
                    }

                    let name_changed = existing.name != clean_name;
                    let updated = Brand {
                        id: existing.id,
                        tenant_id: existing.tenant_id,
                        name: clean_name,
                        name_normalized: normalized,
                        active,
                        created_at: existing.created_at,
                        updated_at: now,
                        deleted_at: existing.deleted_at,
                        created_by: existing.created_by,
                    };

                    stores.catalog().update_brand(&updated).await?;
                    if name_changed {
                        let affected = stores
                            .catalog()
                            .list_models_by_brand(&session.tenant_id, &updated.id)
                            .await?;
                        if !affected.is_empty() {
                            let all_cats = stores
                                .catalog()
                                .list_categories(&session.tenant_id, true)
                                .await?;
                            let cat_names: HashMap<String, String> =
                                all_cats.into_iter().map(|c| (c.id, c.name)).collect();
                            for m in affected {
                                let cat_name = cat_names
                                    .get(&m.category_id)
                                    .map(String::as_str)
                                    .unwrap_or("");
                                let new_search = build_catalog_search_text(
                                    &m.name,
                                    m.model_code.as_deref(),
                                    m.sku.as_deref(),
                                    m.ean.as_deref(),
                                    m.ncm.as_deref(),
                                    cat_name,
                                    Some(&updated.name),
                                    m.description.as_deref(),
                                );
                                stores
                                    .catalog()
                                    .update_model_search_text(
                                        &session.tenant_id,
                                        &m.id,
                                        &new_search,
                                    )
                                    .await?;
                            }
                        }
                    }
                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &session.tenant_id,
                            Some(&session.user_id),
                            "catalog.brand.updated",
                            "brand",
                            &updated.id,
                            &format!("Marca '{}' atualizada.", updated.name),
                        ))
                        .await?;
                    Ok(updated)
                })
            })
            .await?;

        Ok(updated)
    }

    pub async fn delete_brand(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let id_str = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();
            Box::pin(async move {
                let existing = stores
                    .catalog()
                    .find_brand_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Marca não encontrada.".to_string()))?;

                let model_count = stores
                    .catalog()
                    .count_models_by_brand(&session.tenant_id, &id)
                    .await?;
                if model_count > 0 {
                    return Err(CoreError::Conflict(
                        "Não é possível excluir a marca pois existem modelos de equipamentos vinculados."
                            .to_string(),
                    ));
                }

                stores
                    .catalog()
                    .soft_delete_brand(&session.tenant_id, &id, &now)
                    .await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.brand.deleted",
                        "brand",
                        &existing.id,
                        &format!("Marca '{}' excluída.", existing.name),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn list_categories_tree(
        &self,
        session: &SessionInfo,
        include_inactive: bool,
    ) -> Result<Vec<EquipmentCategoryTreeItem>> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        let categories = self
            .stores
            .catalog
            .list_categories(&session.tenant_id, include_inactive)
            .await?;
        let all_defs = self
            .stores
            .catalog
            .list_all_attribute_definitions(&session.tenant_id)
            .await?;
        let model_counts = self
            .stores
            .catalog
            .count_models_per_category(&session.tenant_id)
            .await?;

        let mut defs_by_cat: HashMap<String, Vec<AttributeDefinition>> = HashMap::new();
        for def in all_defs {
            defs_by_cat
                .entry(def.category_id.clone())
                .or_default()
                .push(def);
        }
        let counts_map: HashMap<String, i64> = model_counts.into_iter().collect();

        let mut children_map: HashMap<Option<String>, Vec<EquipmentCategory>> = HashMap::new();
        for cat in categories {
            children_map
                .entry(cat.parent_id.clone())
                .or_default()
                .push(cat);
        }
        for list in children_map.values_mut() {
            list.sort_by(|a, b| {
                a.sort_order
                    .cmp(&b.sort_order)
                    .then_with(|| a.name_normalized.cmp(&b.name_normalized))
            });
        }

        let mut out = Vec::new();
        fn walk(
            parent_id: Option<&str>,
            depth: usize,
            children_map: &HashMap<Option<String>, Vec<EquipmentCategory>>,
            defs_by_cat: &mut HashMap<String, Vec<AttributeDefinition>>,
            counts_map: &HashMap<String, i64>,
            out: &mut Vec<EquipmentCategoryTreeItem>,
        ) {
            let key = parent_id.map(|s| s.to_string());
            if let Some(nodes) = children_map.get(&key) {
                for cat in nodes {
                    let mut own_attrs = defs_by_cat.remove(&cat.id).unwrap_or_default();
                    own_attrs.sort_by(|a, b| {
                        a.sort_order
                            .cmp(&b.sort_order)
                            .then_with(|| a.label.cmp(&b.label))
                    });
                    let model_count = *counts_map.get(&cat.id).unwrap_or(&0);
                    let cat_id = cat.id.clone();
                    out.push(EquipmentCategoryTreeItem {
                        category: cat.clone(),
                        depth,
                        own_attributes: own_attrs,
                        model_count,
                    });
                    walk(
                        Some(&cat_id),
                        depth + 1,
                        children_map,
                        defs_by_cat,
                        counts_map,
                        out,
                    );
                }
            }
        }

        walk(
            None,
            1,
            &children_map,
            &mut defs_by_cat,
            &counts_map,
            &mut out,
        );
        Ok(out)
    }

    pub async fn get_category_effective_attributes(
        &self,
        session: &SessionInfo,
        category_id: &str,
    ) -> Result<Vec<InheritedAttributeDefinition>> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        let categories = self
            .stores
            .catalog
            .list_categories(&session.tenant_id, true)
            .await?;
        let all_defs = self
            .stores
            .catalog
            .list_all_attribute_definitions(&session.tenant_id)
            .await?;
        resolve_inherited_attributes(&categories, &all_defs, category_id)
    }

    pub async fn create_category(
        &self,
        session: &SessionInfo,
        input: &CreateCategoryInput,
    ) -> Result<EquipmentCategory> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let (clean_name, normalized, clean_desc) =
            validate_category_fields(&input.name, input.description.as_deref())?;
        let parent_id = input
            .parent_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let now = Utc::now().to_rfc3339();

        let category = EquipmentCategory {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            parent_id: parent_id.clone(),
            name: clean_name,
            name_normalized: normalized,
            description: clean_desc,
            sort_order: input.sort_order,
            active: input.active,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let session_clone = session.clone();
        let cat_clone = category.clone();
        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let cat = cat_clone.clone();
            Box::pin(async move {
                let all_cats = stores
                    .catalog()
                    .list_categories(&session.tenant_id, true)
                    .await?;
                validate_category_tree_placement(&all_cats, None, cat.parent_id.as_deref())?;

                if stores
                    .catalog()
                    .find_category_by_parent_and_name(
                        &session.tenant_id,
                        cat.parent_id.as_deref(),
                        &cat.name_normalized,
                    )
                    .await?
                    .is_some()
                {
                    return Err(CoreError::Conflict(format!(
                        "Já existe uma categoria '{}' neste nível.",
                        cat.name
                    )));
                }

                stores.catalog().create_category(&cat).await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.category.created",
                        "equipment_category",
                        &cat.id,
                        &format!("Categoria '{}' criada.", cat.name),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await?;

        Ok(category)
    }

    pub async fn update_category(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateCategoryInput,
    ) -> Result<EquipmentCategory> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let (clean_name, normalized, clean_desc) =
            validate_category_fields(&input.name, input.description.as_deref())?;
        let parent_id = input
            .parent_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let now = Utc::now().to_rfc3339();

        let session_clone = session.clone();
        let id_str = id.to_string();
        let sort_order = input.sort_order;
        let active = input.active;
        let expected_updated_at = input.expected_updated_at.clone();

        let updated = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let id = id_str.clone();
                let parent_id = parent_id.clone();
                let clean_name = clean_name.clone();
                let normalized = normalized.clone();
                let clean_desc = clean_desc.clone();
                let now = now.clone();
                let expected_updated_at = expected_updated_at.clone();
                Box::pin(async move {
                    let all_cats = stores
                        .catalog()
                        .list_categories(&session.tenant_id, true)
                        .await?;
                    let existing = all_cats
                        .iter()
                        .find(|c| c.id == id)
                        .cloned()
                        .ok_or_else(|| {
                            CoreError::NotFound("Categoria não encontrada.".to_string())
                        })?;

                    if existing.updated_at != expected_updated_at {
                        return Err(CoreError::Conflict(
                            "A categoria foi alterada por outra pessoa. Recarregue e tente novamente."
                                .to_string(),
                        ));
                    }

                    validate_category_tree_placement(
                        &all_cats,
                        Some(&id),
                        parent_id.as_deref(),
                    )?;

                    if parent_id != existing.parent_id {
                        let all_defs = stores
                            .catalog()
                            .list_all_attribute_definitions(&session.tenant_id)
                            .await?;
                        validate_category_move_attribute_keys(
                            &all_cats,
                            &all_defs,
                            &id,
                            parent_id.as_deref(),
                        )?;
                    }

                    if let Some(other) = stores
                        .catalog()
                        .find_category_by_parent_and_name(
                            &session.tenant_id,
                            parent_id.as_deref(),
                            &normalized,
                        )
                        .await?
                    {
                        if other.id != existing.id {
                            return Err(CoreError::Conflict(format!(
                                "Já existe outra categoria '{}' neste nível.",
                                clean_name
                            )));
                        }
                    }

                    let name_or_parent_changed =
                        existing.name != clean_name || existing.parent_id != parent_id;
                    let updated = EquipmentCategory {
                        id: existing.id,
                        tenant_id: existing.tenant_id,
                        parent_id,
                        name: clean_name,
                        name_normalized: normalized,
                        description: clean_desc,
                        sort_order,
                        active,
                        created_at: existing.created_at,
                        updated_at: now,
                        deleted_at: existing.deleted_at,
                        created_by: existing.created_by,
                    };

                    stores.catalog().update_category(&updated).await?;
                    if name_or_parent_changed {
                        let affected = stores
                            .catalog()
                            .list_models_by_category(&session.tenant_id, &updated.id)
                            .await?;
                        if !affected.is_empty() {
                            let all_brands = stores
                                .catalog()
                                .list_brands(&session.tenant_id, true)
                                .await?;
                            let brand_names: HashMap<String, String> =
                                all_brands.into_iter().map(|b| (b.id, b.name)).collect();
                            for m in affected {
                                let b_name = m
                                    .brand_id
                                    .as_ref()
                                    .and_then(|bid| brand_names.get(bid))
                                    .map(String::as_str);
                                let new_search = build_catalog_search_text(
                                    &m.name,
                                    m.model_code.as_deref(),
                                    m.sku.as_deref(),
                                    m.ean.as_deref(),
                                    m.ncm.as_deref(),
                                    &updated.name,
                                    b_name,
                                    m.description.as_deref(),
                                );
                                stores
                                    .catalog()
                                    .update_model_search_text(&session.tenant_id, &m.id, &new_search)
                                    .await?;
                            }
                        }
                    }
                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &session.tenant_id,
                            Some(&session.user_id),
                            "catalog.category.updated",
                            "equipment_category",
                            &updated.id,
                            &format!("Categoria '{}' atualizada.", updated.name),
                        ))
                        .await?;
                    Ok(updated)
                })
            })
            .await?;

        Ok(updated)
    }

    pub async fn delete_category(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let id_str = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();
            Box::pin(async move {
                let existing = stores
                    .catalog()
                    .find_category_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Categoria não encontrada.".to_string())
                    })?;

                let child_count = stores
                    .catalog()
                    .count_child_categories(&session.tenant_id, &id)
                    .await?;
                if child_count > 0 {
                    return Err(CoreError::Conflict(
                        "Não é possível excluir a categoria pois ela possui subcategorias ativas."
                            .to_string(),
                    ));
                }

                let model_count = stores
                    .catalog()
                    .count_models_by_category(&session.tenant_id, &id)
                    .await?;
                if model_count > 0 {
                    return Err(CoreError::Conflict(
                        "Não é possível excluir a categoria pois existem modelos de equipamentos vinculados."
                            .to_string(),
                    ));
                }

                stores
                    .catalog()
                    .soft_delete_category(&session.tenant_id, &id, &now)
                    .await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.category.deleted",
                        "equipment_category",
                        &existing.id,
                        &format!("Categoria '{}' excluída.", existing.name),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn create_attribute_definition(
        &self,
        session: &SessionInfo,
        input: &CreateAttributeDefinitionInput,
    ) -> Result<AttributeDefinition> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let (clean_key, clean_label, clean_unit, clean_choices) =
            validate_attribute_definition_fields(
                &input.field_key,
                &input.label,
                input.data_type,
                input.unit.as_deref(),
                &input.choices,
            )?;

        let now = Utc::now().to_rfc3339();
        let def = AttributeDefinition {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            category_id: input.category_id.clone(),
            field_key: clean_key,
            label: clean_label,
            data_type: input.data_type,
            unit: clean_unit,
            choices: clean_choices,
            is_required: input.is_required,
            sort_order: input.sort_order,
            active: input.active,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let session_clone = session.clone();
        let def_clone = def.clone();
        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let def = def_clone.clone();
            Box::pin(async move {
                let all_cats = stores
                    .catalog()
                    .list_categories(&session.tenant_id, true)
                    .await?;
                let all_defs = stores
                    .catalog()
                    .list_all_attribute_definitions(&session.tenant_id)
                    .await?;
                let own_count = all_defs
                    .iter()
                    .filter(|d| d.category_id == def.category_id)
                    .count();
                if own_count >= MAX_ATTRIBUTES_PER_CATEGORY {
                    return Err(CoreError::Validation(format!(
                        "Limite máximo de {} atributos por categoria atingido.",
                        MAX_ATTRIBUTES_PER_CATEGORY
                    )));
                }
                validate_attribute_key_in_hierarchy(
                    &all_cats,
                    &all_defs,
                    &def.category_id,
                    &def.field_key,
                    None,
                )?;

                stores.catalog().create_attribute_definition(&def).await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.attribute.created",
                        "attribute_definition",
                        &def.id,
                        &format!(
                            "Atributo '{}' ({}) adicionado à categoria {}.",
                            def.label, def.field_key, def.category_id
                        ),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await?;

        Ok(def)
    }

    pub async fn update_attribute_definition(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateAttributeDefinitionInput,
    ) -> Result<AttributeDefinition> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let (clean_key, clean_label, clean_unit, clean_choices) =
            validate_attribute_definition_fields(
                &input.field_key,
                &input.label,
                input.data_type,
                input.unit.as_deref(),
                &input.choices,
            )?;

        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let id_str = id.to_string();
        let input_clone = input.clone();

        let updated = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let id = id_str.clone();
                let clean_key = clean_key.clone();
                let clean_label = clean_label.clone();
                let clean_unit = clean_unit.clone();
                let clean_choices = clean_choices.clone();
                let input = input_clone.clone();
                let now = now.clone();
                Box::pin(async move {
                    let existing = stores
                        .catalog()
                        .find_attribute_definition_by_id(&session.tenant_id, &id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Definição de atributo não encontrada.".to_string())
                        })?;

                    if existing.updated_at != input.expected_updated_at {
                        return Err(CoreError::Conflict(
                            "O atributo foi alterado por outra pessoa. Recarregue e tente novamente."
                                .to_string(),
                        ));
                    }

                    let all_cats = stores
                        .catalog()
                        .list_categories(&session.tenant_id, true)
                        .await?;
                    let all_defs = stores
                        .catalog()
                        .list_all_attribute_definitions(&session.tenant_id)
                        .await?;
                    validate_attribute_key_in_hierarchy(
                        &all_cats,
                        &all_defs,
                        &existing.category_id,
                        &clean_key,
                        Some(&existing.id),
                    )?;

                    if input.data_type != existing.data_type {
                        let used_count = stores
                            .catalog()
                            .count_models_using_attribute_definition(&session.tenant_id, &existing.id)
                            .await?;
                        if used_count > 0 {
                            return Err(CoreError::Conflict(format!(
                                "Não é possível alterar o tipo do atributo '{}' pois ele já possui valores preenchidos em {} modelo(s).",
                                existing.label, used_count
                            )));
                        }
                    } else if existing.data_type == AttributeDataType::Select {
                        let used_values = stores
                            .catalog()
                            .list_distinct_text_values_for_attribute_definition(
                                &session.tenant_id,
                                &existing.id,
                            )
                            .await?;
                        let removed_in_use: Vec<String> = used_values
                            .into_iter()
                            .filter(|val| !clean_choices.iter().any(|c| c == val))
                            .collect();
                        if !removed_in_use.is_empty() {
                            return Err(CoreError::Conflict(format!(
                                "Não é possível remover opções do atributo '{}' que já estão em uso: {}.",
                                existing.label,
                                removed_in_use.join(", ")
                            )));
                        }
                    }

                    let updated = AttributeDefinition {
                        id: existing.id,
                        tenant_id: existing.tenant_id,
                        category_id: existing.category_id,
                        field_key: clean_key,
                        label: clean_label,
                        data_type: input.data_type,
                        unit: clean_unit,
                        choices: clean_choices,
                        is_required: input.is_required,
                        sort_order: input.sort_order,
                        active: input.active,
                        created_at: existing.created_at,
                        updated_at: now,
                        deleted_at: existing.deleted_at,
                        created_by: existing.created_by,
                    };

                    stores
                        .catalog()
                        .update_attribute_definition(&updated)
                        .await?;
                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &session.tenant_id,
                            Some(&session.user_id),
                            "catalog.attribute.updated",
                            "attribute_definition",
                            &updated.id,
                            &format!("Atributo '{}' atualizado.", updated.label),
                        ))
                        .await?;
                    Ok(updated)
                })
            })
            .await?;

        Ok(updated)
    }

    pub async fn delete_attribute_definition(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let id_str = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();
            Box::pin(async move {
                let existing = stores
                    .catalog()
                    .find_attribute_definition_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Definição de atributo não encontrada.".to_string())
                    })?;

                stores
                    .catalog()
                    .soft_delete_attribute_definition(&session.tenant_id, &id, &now)
                    .await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.attribute.deleted",
                        "attribute_definition",
                        &existing.id,
                        &format!("Atributo '{}' excluído.", existing.label),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn list_equipment_models(
        &self,
        session: &SessionInfo,
        filter: &CatalogListFilter,
    ) -> Result<CatalogListResult> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        validate_search_query_limits(filter.query.as_deref())?;

        let descendant_ids = if let Some(ref cat_id) = filter.category_id {
            let all_cats = self
                .stores
                .catalog
                .list_categories(&session.tenant_id, true)
                .await?;
            Some(collect_subtree_ids(&all_cats, cat_id))
        } else {
            None
        };

        let mut result = self
            .stores
            .catalog
            .list_models(&session.tenant_id, filter, descendant_ids.as_deref())
            .await?;

        for item in &mut result.items {
            self.sanitize_model_financials(session, &mut item.model);
        }

        Ok(result)
    }

    pub async fn get_equipment_model(
        &self,
        session: &SessionInfo,
        id: &str,
    ) -> Result<EquipmentModelWithDetails> {
        self.check_session_and_permission(session, PERM_CATALOG_READ)?;
        let mut model = self
            .stores
            .catalog
            .find_model_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
            })?;
        self.sanitize_model_financials(session, &mut model);

        let categories = self
            .stores
            .catalog
            .list_categories(&session.tenant_id, true)
            .await?;
        let category_name = categories
            .iter()
            .find(|c| c.id == model.category_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();

        let brand_name = if let Some(ref bid) = model.brand_id {
            self.stores
                .catalog
                .find_brand_by_id(&session.tenant_id, bid)
                .await?
                .map(|b| b.name)
        } else {
            None
        };

        let all_defs = self
            .stores
            .catalog
            .list_all_attribute_definitions(&session.tenant_id)
            .await?;
        let effective_attributes =
            resolve_inherited_attributes(&categories, &all_defs, &model.category_id)?;
        let attribute_values = self
            .stores
            .catalog
            .list_model_attribute_values(&session.tenant_id, &model.id)
            .await?;

        Ok(EquipmentModelWithDetails {
            model,
            category_name,
            brand_name,
            effective_attributes,
            attribute_values,
        })
    }

    pub async fn create_equipment_model(
        &self,
        session: &SessionInfo,
        input: &CreateEquipmentModelInput,
    ) -> Result<EquipmentModelWithDetails> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let validated = validate_equipment_model_fields(
            &input.name,
            input.model_code.as_deref(),
            input.sku.as_deref(),
            input.ean.as_deref(),
            input.ncm.as_deref(),
            input.description.as_deref(),
            input.weight_kg,
            input.length_cm,
            input.width_cm,
            input.height_cm,
            input.power_watts,
            input.amperage,
            input.units_per_case,
            input.daily_rate_cents,
            input.replacement_value_cents,
            input.cost_price_cents,
        )?;

        if (input.replacement_value_cents.is_some() || input.cost_price_cents.is_some())
            && !can_manage_financials(session)
        {
            return Err(CoreError::Forbidden(
                "Permissão insuficiente para definir valores financeiros do modelo (requer 'finance:manage' ou Proprietário)."
                    .to_string(),
            ));
        }

        let brand_id = input
            .brand_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let now = Utc::now().to_rfc3339();
        let model_id = Uuid::now_v7().to_string();

        let session_clone = session.clone();
        let input_clone = input.clone();
        let validated_clone = validated.clone();
        let brand_id_clone = brand_id.clone();
        let now_clone = now.clone();
        let model_id_clone = model_id.clone();

        let mut result = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let input = input_clone.clone();
                let v = validated_clone.clone();
                let brand_id = brand_id_clone.clone();
                let now = now_clone.clone();
                let model_id = model_id_clone.clone();
                Box::pin(async move {
                    let categories = stores
                        .catalog()
                        .list_categories(&session.tenant_id, true)
                        .await?;
                    let category = categories
                        .iter()
                        .find(|c| c.id == input.category_id)
                        .cloned()
                        .ok_or_else(|| {
                            CoreError::NotFound("Categoria selecionada não encontrada.".to_string())
                        })?;
                    if !category.active {
                        return Err(CoreError::Validation(format!(
                            "A categoria '{}' está inativa e não pode ser atribuída a um novo modelo.",
                            category.name
                        )));
                    }

                    let brand_name = if let Some(ref bid) = brand_id {
                        let b = stores
                            .catalog()
                            .find_brand_by_id(&session.tenant_id, bid)
                            .await?
                            .ok_or_else(|| {
                                CoreError::NotFound("Marca selecionada não encontrada.".to_string())
                            })?;
                        if !b.active {
                            return Err(CoreError::Validation(format!(
                                "A marca '{}' está inativa e não pode ser atribuída a um novo modelo.",
                                b.name
                            )));
                        }
                        Some(b.name)
                    } else {
                        None
                    };

                    let all_defs = stores
                        .catalog()
                        .list_all_attribute_definitions(&session.tenant_id)
                        .await?;
                    let effective_attributes =
                        resolve_inherited_attributes(&categories, &all_defs, &category.id)?;
                    let validated_attrs = validate_model_attribute_values(
                        &effective_attributes,
                        &input.attribute_values,
                    )?;

                    let attr_values: Vec<EquipmentModelAttributeValue> = validated_attrs
                        .into_iter()
                        .map(|av| EquipmentModelAttributeValue {
                            id: Uuid::now_v7().to_string(),
                            tenant_id: session.tenant_id.clone(),
                            model_id: model_id.clone(),
                            definition_id: av.definition_id,
                            value_text: av.value_text,
                            value_number: av.value_number,
                            value_bool: av.value_bool,
                            created_at: now.clone(),
                            updated_at: now.clone(),
                            deleted_at: None,
                            created_by: Some(session.user_id.clone()),
                        })
                        .collect();

                    if let Some(ref sku_norm) = v.sku_normalized {
                        if let Some(other) = stores
                            .catalog()
                            .find_model_by_sku_normalized(&session.tenant_id, sku_norm)
                            .await?
                        {
                            return Err(CoreError::Conflict(format!(
                                "O SKU '{}' já está em uso pelo modelo '{}'.",
                                v.sku.as_deref().unwrap_or(sku_norm),
                                other.name
                            )));
                        }
                    }
                    if let (Some(ref bid), Some(ref code_norm)) =
                        (&brand_id, &v.model_code_normalized)
                    {
                        if let Some(other) = stores
                            .catalog()
                            .find_model_by_brand_and_code_normalized(
                                &session.tenant_id,
                                bid,
                                code_norm,
                            )
                            .await?
                        {
                            return Err(CoreError::Conflict(format!(
                                "O código '{}' já está em uso para esta marca pelo modelo '{}'.",
                                v.model_code.as_deref().unwrap_or(code_norm),
                                other.name
                            )));
                        }
                    }

                    let search_text = build_catalog_search_text(
                        &v.name,
                        v.model_code.as_deref(),
                        v.sku.as_deref(),
                        v.ean.as_deref(),
                        v.ncm.as_deref(),
                        &category.name,
                        brand_name.as_deref(),
                        v.description.as_deref(),
                    );

                    let model = EquipmentModel {
                        id: model_id,
                        tenant_id: session.tenant_id.clone(),
                        category_id: category.id,
                        brand_id,
                        name: v.name,
                        name_sort: v.name_sort,
                        model_code: v.model_code,
                        sku: v.sku,
                        ean: v.ean,
                        ncm: v.ncm,
                        description: v.description,
                        control_mode: input.control_mode,
                        weight_kg: v.weight_kg,
                        length_cm: v.length_cm,
                        width_cm: v.width_cm,
                        height_cm: v.height_cm,
                        power_watts: v.power_watts,
                        voltage: input.voltage,
                        amperage: v.amperage,
                        requires_case: input.requires_case,
                        units_per_case: v.units_per_case,
                        daily_rate_cents: v.daily_rate_cents,
                        replacement_value_cents: v.replacement_value_cents,
                        cost_price_cents: v.cost_price_cents,
                        active: input.active,
                        search_text,
                        created_at: now.clone(),
                        updated_at: now,
                        deleted_at: None,
                        created_by: Some(session.user_id.clone()),
                    };

                    stores.catalog().create_model(&model, &attr_values).await?;
                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &session.tenant_id,
                            Some(&session.user_id),
                            "catalog.model.created",
                            "equipment_model",
                            &model.id,
                            &format!("Modelo '{}' criado no catálogo.", model.name),
                        ))
                        .await?;

                    Ok(EquipmentModelWithDetails {
                        model,
                        category_name: category.name,
                        brand_name,
                        effective_attributes,
                        attribute_values: attr_values,
                    })
                })
            })
            .await?;

        self.sanitize_model_financials(session, &mut result.model);
        Ok(result)
    }

    pub async fn update_equipment_model(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateEquipmentModelInput,
    ) -> Result<EquipmentModelWithDetails> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let validated = validate_equipment_model_fields(
            &input.name,
            input.model_code.as_deref(),
            input.sku.as_deref(),
            input.ean.as_deref(),
            input.ncm.as_deref(),
            input.description.as_deref(),
            input.weight_kg,
            input.length_cm,
            input.width_cm,
            input.height_cm,
            input.power_watts,
            input.amperage,
            input.units_per_case,
            input.daily_rate_cents,
            input.replacement_value_cents,
            input.cost_price_cents,
        )?;

        if (input.replacement_value_cents.is_some() || input.cost_price_cents.is_some())
            && !can_manage_financials(session)
        {
            return Err(CoreError::Forbidden(
                "Permissão insuficiente para definir valores financeiros do modelo (requer 'finance:manage' ou Proprietário)."
                    .to_string(),
            ));
        }

        let brand_id = input
            .brand_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let now = Utc::now().to_rfc3339();

        let session_clone = session.clone();
        let id_str = id.to_string();
        let input_clone = input.clone();
        let validated_clone = validated.clone();
        let brand_id_clone = brand_id.clone();
        let now_clone = now.clone();

        let mut result = self
            .run_transaction(|stores| {
                let session = session_clone.clone();
                let id = id_str.clone();
                let input = input_clone.clone();
                let v = validated_clone.clone();
                let brand_id = brand_id_clone.clone();
                let now = now_clone.clone();
                Box::pin(async move {
                    let existing = stores
                        .catalog()
                        .find_model_by_id(&session.tenant_id, &id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                        })?;

                    if existing.updated_at != input.expected_updated_at {
                        return Err(CoreError::Conflict(
                            "O modelo foi alterado por outra pessoa. Recarregue e tente novamente."
                                .to_string(),
                        ));
                    }

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

                        let balances = stores
                            .inventory()
                            .list_stock_balances_by_model(&session.tenant_id, &existing.id)
                            .await?;
                        if balances.iter().any(|b| b.quantity_on_hand > 0 || b.quantity_maintenance > 0) {
                            return Err(CoreError::Validation(
                                "Não é permitido alterar o modo de controle de um modelo que possui saldo de estoque."
                                    .to_string(),
                            ));
                        }
                    }

                    let categories = stores
                        .catalog()
                        .list_categories(&session.tenant_id, true)
                        .await?;
                    let category = categories
                        .iter()
                        .find(|c| c.id == input.category_id)
                        .cloned()
                        .ok_or_else(|| {
                            CoreError::NotFound("Categoria selecionada não encontrada.".to_string())
                        })?;
                    if category.id != existing.category_id && !category.active {
                        return Err(CoreError::Validation(format!(
                            "A categoria '{}' está inativa e não pode ser atribuída ao modelo.",
                            category.name
                        )));
                    }

                    let brand_name = if let Some(ref bid) = brand_id {
                        let b = stores
                            .catalog()
                            .find_brand_by_id(&session.tenant_id, bid)
                            .await?
                            .ok_or_else(|| {
                                CoreError::NotFound("Marca selecionada não encontrada.".to_string())
                            })?;
                        if existing.brand_id.as_deref() != Some(b.id.as_str()) && !b.active {
                            return Err(CoreError::Validation(format!(
                                "A marca '{}' está inativa e não pode ser atribuída ao modelo.",
                                b.name
                            )));
                        }
                        Some(b.name)
                    } else {
                        None
                    };

                    let all_defs = stores
                        .catalog()
                        .list_all_attribute_definitions(&session.tenant_id)
                        .await?;
                    let effective_attributes =
                        resolve_inherited_attributes(&categories, &all_defs, &category.id)?;
                    let validated_attrs = validate_model_attribute_values(
                        &effective_attributes,
                        &input.attribute_values,
                    )?;

                    let existing_attr_map: HashMap<String, EquipmentModelAttributeValue> = stores
                        .catalog()
                        .list_model_attribute_values(&session.tenant_id, &existing.id)
                        .await?
                        .into_iter()
                        .map(|v| (v.definition_id.clone(), v))
                        .collect();

                    if let Some(ref sku_norm) = v.sku_normalized {
                        if let Some(other) = stores
                            .catalog()
                            .find_model_by_sku_normalized(&session.tenant_id, sku_norm)
                            .await?
                        {
                            if other.id != existing.id {
                                return Err(CoreError::Conflict(format!(
                                    "O SKU '{}' já está em uso pelo modelo '{}'.",
                                    v.sku.as_deref().unwrap_or(sku_norm),
                                    other.name
                                )));
                            }
                        }
                    }
                    if let (Some(ref bid), Some(ref code_norm)) =
                        (&brand_id, &v.model_code_normalized)
                    {
                        if let Some(other) = stores
                            .catalog()
                            .find_model_by_brand_and_code_normalized(
                                &session.tenant_id,
                                bid,
                                code_norm,
                            )
                            .await?
                        {
                            if other.id != existing.id {
                                return Err(CoreError::Conflict(format!(
                                    "O código '{}' já está em uso para esta marca pelo modelo '{}'.",
                                    v.model_code.as_deref().unwrap_or(code_norm),
                                    other.name
                                )));
                            }
                        }
                    }

                    let attr_values: Vec<EquipmentModelAttributeValue> = validated_attrs
                        .into_iter()
                        .map(|av| {
                            if let Some(prev) = existing_attr_map.get(&av.definition_id) {
                                EquipmentModelAttributeValue {
                                    id: prev.id.clone(),
                                    tenant_id: session.tenant_id.clone(),
                                    model_id: existing.id.clone(),
                                    definition_id: av.definition_id,
                                    value_text: av.value_text,
                                    value_number: av.value_number,
                                    value_bool: av.value_bool,
                                    created_at: prev.created_at.clone(),
                                    updated_at: now.clone(),
                                    deleted_at: None,
                                    created_by: prev.created_by.clone(),
                                }
                            } else {
                                EquipmentModelAttributeValue {
                                    id: Uuid::now_v7().to_string(),
                                    tenant_id: session.tenant_id.clone(),
                                    model_id: existing.id.clone(),
                                    definition_id: av.definition_id,
                                    value_text: av.value_text,
                                    value_number: av.value_number,
                                    value_bool: av.value_bool,
                                    created_at: now.clone(),
                                    updated_at: now.clone(),
                                    deleted_at: None,
                                    created_by: Some(session.user_id.clone()),
                                }
                            }
                        })
                        .collect();

                    let search_text = build_catalog_search_text(
                        &v.name,
                        v.model_code.as_deref(),
                        v.sku.as_deref(),
                        v.ean.as_deref(),
                        v.ncm.as_deref(),
                        &category.name,
                        brand_name.as_deref(),
                        v.description.as_deref(),
                    );

                    let updated = EquipmentModel {
                        id: existing.id,
                        tenant_id: existing.tenant_id,
                        category_id: category.id,
                        brand_id,
                        name: v.name,
                        name_sort: v.name_sort,
                        model_code: v.model_code,
                        sku: v.sku,
                        ean: v.ean,
                        ncm: v.ncm,
                        description: v.description,
                        control_mode: input.control_mode,
                        weight_kg: v.weight_kg,
                        length_cm: v.length_cm,
                        width_cm: v.width_cm,
                        height_cm: v.height_cm,
                        power_watts: v.power_watts,
                        voltage: input.voltage,
                        amperage: v.amperage,
                        requires_case: input.requires_case,
                        units_per_case: v.units_per_case,
                        daily_rate_cents: v.daily_rate_cents,
                        replacement_value_cents: v.replacement_value_cents,
                        cost_price_cents: v.cost_price_cents,
                        active: input.active,
                        search_text,
                        created_at: existing.created_at,
                        updated_at: now,
                        deleted_at: existing.deleted_at,
                        created_by: existing.created_by,
                    };

                    stores
                        .catalog()
                        .update_model(&updated, &attr_values)
                        .await?;
                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &session.tenant_id,
                            Some(&session.user_id),
                            "catalog.model.updated",
                            "equipment_model",
                            &updated.id,
                            &format!("Modelo '{}' atualizado no catálogo.", updated.name),
                        ))
                        .await?;

                    Ok(EquipmentModelWithDetails {
                        model: updated,
                        category_name: category.name,
                        brand_name,
                        effective_attributes,
                        attribute_values: attr_values,
                    })
                })
            })
            .await?;

        self.sanitize_model_financials(session, &mut result.model);
        Ok(result)
    }

    pub async fn set_equipment_model_active(
        &self,
        session: &SessionInfo,
        id: &str,
        active: bool,
    ) -> Result<()> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let id_str = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();
            Box::pin(async move {
                let existing = stores
                    .catalog()
                    .find_model_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                    })?;

                stores
                    .catalog()
                    .set_model_active(&session.tenant_id, &id, active, &now)
                    .await?;
                let action = if active {
                    "catalog.model.activated"
                } else {
                    "catalog.model.deactivated"
                };
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        action,
                        "equipment_model",
                        &existing.id,
                        &format!(
                            "Modelo '{}' {}.",
                            existing.name,
                            if active { "ativado" } else { "desativado" }
                        ),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn delete_equipment_model(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();
        let id_str = id.to_string();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();
            Box::pin(async move {
                let existing = stores
                    .catalog()
                    .find_model_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Modelo de equipamento não encontrado.".to_string())
                    })?;

                stores
                    .catalog()
                    .soft_delete_model(&session.tenant_id, &id, &now)
                    .await?;
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.model.deleted",
                        "equipment_model",
                        &existing.id,
                        &format!("Modelo '{}' excluído do catálogo.", existing.name),
                    ))
                    .await?;
                Ok(())
            })
        })
        .await
    }

    /// Carrega de forma idempotente a árvore sugerida de categorias Audiovisuais e seus campos técnicos.
    pub async fn load_suggested_catalog_categories(
        &self,
        session: &SessionInfo,
    ) -> Result<SuggestedCategoriesResult> {
        self.check_session_and_permission(session, PERM_CATALOG_MANAGE)?;
        let now = Utc::now().to_rfc3339();
        let session_clone = session.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let now = now.clone();
            Box::pin(async move {
                struct SuggestedAttr {
                    field_key: &'static str,
                    label: &'static str,
                    data_type: AttributeDataType,
                    unit: Option<&'static str>,
                    choices: &'static [&'static str],
                }
                struct SuggestedCat {
                    parent_name: Option<&'static str>,
                    name: &'static str,
                    sort_order: i64,
                    attrs: &'static [SuggestedAttr],
                }

                let specs: &[SuggestedCat] = &[
                    SuggestedCat {
                        parent_name: None,
                        name: "Áudio",
                        sort_order: 1,
                        attrs: &[SuggestedAttr {
                            field_key: "impedancia_ohms",
                            label: "Impedância",
                            data_type: AttributeDataType::Number,
                            unit: Some("Ω"),
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Áudio"),
                        name: "Mesas de som",
                        sort_order: 1,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "canais",
                                label: "Canais",
                                data_type: AttributeDataType::Number,
                                unit: Some("ch"),
                                choices: &[],
                            },
                            SuggestedAttr {
                                field_key: "protocolo_digital",
                                label: "Protocolo Digital",
                                data_type: AttributeDataType::Select,
                                unit: None,
                                choices: &["Analógica", "Dante", "AES50", "MADI", "USB"],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: Some("Áudio"),
                        name: "Microfones",
                        sort_order: 2,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "tipo_captacao",
                                label: "Tipo de Captação",
                                data_type: AttributeDataType::Select,
                                unit: None,
                                choices: &["Dinâmico", "Condensador", "Fita"],
                            },
                            SuggestedAttr {
                                field_key: "sem_fio",
                                label: "Sem fio",
                                data_type: AttributeDataType::Boolean,
                                unit: None,
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: Some("Áudio"),
                        name: "Caixas de som",
                        sort_order: 3,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "potencia_rms_w",
                                label: "Potência RMS",
                                data_type: AttributeDataType::Number,
                                unit: Some("W"),
                                choices: &[],
                            },
                            SuggestedAttr {
                                field_key: "caixa_ativa",
                                label: "Ativa",
                                data_type: AttributeDataType::Boolean,
                                unit: None,
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "Iluminação",
                        sort_order: 2,
                        attrs: &[SuggestedAttr {
                            field_key: "protocolo_dmx",
                            label: "Protocolo DMX",
                            data_type: AttributeDataType::Select,
                            unit: None,
                            choices: &["DMX512", "Art-Net", "sACN", "Sem DMX"],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Iluminação"),
                        name: "Moving Heads",
                        sort_order: 1,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "tipo_feixe",
                                label: "Tipo de Feixe",
                                data_type: AttributeDataType::Select,
                                unit: None,
                                choices: &["Beam", "Spot", "Wash", "Híbrido"],
                            },
                            SuggestedAttr {
                                field_key: "canais_dmx",
                                label: "Canais DMX",
                                data_type: AttributeDataType::Number,
                                unit: Some("ch"),
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: Some("Iluminação"),
                        name: "Refletores LED",
                        sort_order: 2,
                        attrs: &[SuggestedAttr {
                            field_key: "grau_abertura",
                            label: "Grau de Abertura",
                            data_type: AttributeDataType::Number,
                            unit: Some("°"),
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "Vídeo e Projeção",
                        sort_order: 3,
                        attrs: &[SuggestedAttr {
                            field_key: "resolucao_nativa",
                            label: "Resolução Nativa",
                            data_type: AttributeDataType::Select,
                            unit: None,
                            choices: &["HD", "Full HD", "WUXGA", "4K UHD"],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Vídeo e Projeção"),
                        name: "Projetores",
                        sort_order: 1,
                        attrs: &[SuggestedAttr {
                            field_key: "lumens_ansi",
                            label: "Luminosidade",
                            data_type: AttributeDataType::Number,
                            unit: Some("lm"),
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Vídeo e Projeção"),
                        name: "Painéis de LED",
                        sort_order: 2,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "pixel_pitch_mm",
                                label: "Pixel Pitch",
                                data_type: AttributeDataType::Number,
                                unit: Some("mm"),
                                choices: &[],
                            },
                            SuggestedAttr {
                                field_key: "uso_externo_ip65",
                                label: "Uso Externo (IP65)",
                                data_type: AttributeDataType::Boolean,
                                unit: None,
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "Informática",
                        sort_order: 4,
                        attrs: &[SuggestedAttr {
                            field_key: "sistema_operacional",
                            label: "Sistema Operacional",
                            data_type: AttributeDataType::Text,
                            unit: None,
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Informática"),
                        name: "Notebooks",
                        sort_order: 1,
                        attrs: &[],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "Estrutura e Box Truss",
                        sort_order: 5,
                        attrs: &[SuggestedAttr {
                            field_key: "material_estrutura",
                            label: "Material",
                            data_type: AttributeDataType::Select,
                            unit: None,
                            choices: &["Alumínio", "Aço"],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Estrutura e Box Truss"),
                        name: "Treliças Q30",
                        sort_order: 1,
                        attrs: &[],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "Energia e Cabeamento",
                        sort_order: 6,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "comprimento_m",
                                label: "Comprimento",
                                data_type: AttributeDataType::Number,
                                unit: Some("m"),
                                choices: &[],
                            },
                            SuggestedAttr {
                                field_key: "conector",
                                label: "Conector",
                                data_type: AttributeDataType::Text,
                                unit: None,
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: Some("Energia e Cabeamento"),
                        name: "Cabos de Áudio",
                        sort_order: 1,
                        attrs: &[],
                    },
                    SuggestedCat {
                        parent_name: Some("Energia e Cabeamento"),
                        name: "Cabos de Energia",
                        sort_order: 2,
                        attrs: &[],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "DJ",
                        sort_order: 7,
                        attrs: &[],
                    },
                    SuggestedCat {
                        parent_name: Some("DJ"),
                        name: "Players (CDJ)",
                        sort_order: 1,
                        attrs: &[SuggestedAttr {
                            field_key: "pro_dj_link",
                            label: "Pro DJ Link",
                            data_type: AttributeDataType::Boolean,
                            unit: None,
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("DJ"),
                        name: "Mixers DJ",
                        sort_order: 2,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "canais",
                                label: "Canais",
                                data_type: AttributeDataType::Number,
                                unit: Some("ch"),
                                choices: &[],
                            },
                            SuggestedAttr {
                                field_key: "efeitos",
                                label: "Efeitos",
                                data_type: AttributeDataType::Boolean,
                                unit: None,
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: Some("DJ"),
                        name: "Controladoras DJ",
                        sort_order: 3,
                        attrs: &[
                            SuggestedAttr {
                                field_key: "canais",
                                label: "Canais",
                                data_type: AttributeDataType::Number,
                                unit: Some("ch"),
                                choices: &[],
                            },
                            SuggestedAttr {
                                field_key: "efeitos",
                                label: "Efeitos",
                                data_type: AttributeDataType::Boolean,
                                unit: None,
                                choices: &[],
                            },
                        ],
                    },
                    SuggestedCat {
                        parent_name: None,
                        name: "Acessórios",
                        sort_order: 8,
                        attrs: &[],
                    },
                    SuggestedCat {
                        parent_name: Some("Acessórios"),
                        name: "Pedestais e suportes",
                        sort_order: 1,
                        attrs: &[SuggestedAttr {
                            field_key: "carga_maxima_kg",
                            label: "Carga Máxima",
                            data_type: AttributeDataType::Number,
                            unit: Some("kg"),
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Acessórios"),
                        name: "Cases e racks",
                        sort_order: 2,
                        attrs: &[SuggestedAttr {
                            field_key: "unidades_rack_u",
                            label: "Unidades de Rack",
                            data_type: AttributeDataType::Number,
                            unit: Some("U"),
                            choices: &[],
                        }],
                    },
                    SuggestedCat {
                        parent_name: Some("Acessórios"),
                        name: "Fitas e consumíveis",
                        sort_order: 3,
                        attrs: &[],
                    },
                ];

                let mut root_ids: HashMap<&'static str, String> = HashMap::new();
                let mut categories_created = 0usize;
                let mut attributes_created = 0usize;

                for spec in specs {
                    let parent_id = spec
                        .parent_name
                        .and_then(|pname| root_ids.get(pname).cloned());
                    let norm = normalize_unique_key(spec.name);

                    let cat_id = if let Some(existing) = stores
                        .catalog()
                        .find_category_by_parent_and_name(
                            &session.tenant_id,
                            parent_id.as_deref(),
                            &norm,
                        )
                        .await?
                    {
                        existing.id
                    } else {
                        let new_cat = EquipmentCategory {
                            id: Uuid::now_v7().to_string(),
                            tenant_id: session.tenant_id.clone(),
                            parent_id: parent_id.clone(),
                            name: spec.name.to_string(),
                            name_normalized: norm,
                            description: None,
                            sort_order: spec.sort_order,
                            active: true,
                            created_at: now.clone(),
                            updated_at: now.clone(),
                            deleted_at: None,
                            created_by: Some(session.user_id.clone()),
                        };
                        stores.catalog().create_category(&new_cat).await?;
                        categories_created += 1;
                        new_cat.id
                    };

                    if spec.parent_name.is_none() {
                        root_ids.insert(spec.name, cat_id.clone());
                    }

                    let existing_attrs = stores
                        .catalog()
                        .list_attribute_definitions_by_category(&session.tenant_id, &cat_id)
                        .await?;
                    let existing_keys: HashSet<String> =
                        existing_attrs.into_iter().map(|d| d.field_key).collect();

                    for (idx, attr) in spec.attrs.iter().enumerate() {
                        if !existing_keys.contains(attr.field_key) {
                            let def = AttributeDefinition {
                                id: Uuid::now_v7().to_string(),
                                tenant_id: session.tenant_id.clone(),
                                category_id: cat_id.clone(),
                                field_key: attr.field_key.to_string(),
                                label: attr.label.to_string(),
                                data_type: attr.data_type,
                                unit: attr.unit.map(|s| s.to_string()),
                                choices: attr.choices.iter().map(|s| s.to_string()).collect(),
                                is_required: false,
                                sort_order: (idx as i64) + 1,
                                active: true,
                                created_at: now.clone(),
                                updated_at: now.clone(),
                                deleted_at: None,
                                created_by: Some(session.user_id.clone()),
                            };
                            stores.catalog().create_attribute_definition(&def).await?;
                            attributes_created += 1;
                        }
                    }
                }

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "catalog.suggested_categories.loaded",
                        "equipment_category",
                        &session.tenant_id,
                        &format!(
                            "Carga sugerida de categorias executada ({} categorias, {} atributos criados).",
                            categories_created, attributes_created
                        ),
                    ))
                    .await?;

                Ok(SuggestedCategoriesResult {
                    categories_created,
                    attributes_created,
                })
            })
        })
        .await
    }
}

fn collect_subtree_ids(categories: &[EquipmentCategory], root_id: &str) -> Vec<String> {
    let mut children_map: HashMap<&str, Vec<&str>> = HashMap::new();
    for cat in categories {
        if let Some(ref pid) = cat.parent_id {
            children_map
                .entry(pid.as_str())
                .or_default()
                .push(cat.id.as_str());
        }
    }
    let mut result = Vec::new();
    let mut stack = vec![root_id];
    let mut visited = HashSet::new();
    while let Some(curr) = stack.pop() {
        if !visited.insert(curr) {
            continue;
        }
        result.push(curr.to_string());
        if let Some(kids) = children_map.get(curr) {
            for &kid in kids {
                stack.push(kid);
            }
        }
    }
    result
}
