use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::domain::customer::{build_name_sort, normalize_for_search};
use crate::error::{CoreError, Result};

pub const MAX_CATEGORY_DEPTH: usize = 4;
pub const MAX_ATTRIBUTES_PER_CATEGORY: usize = 50;
pub const MAX_CATALOG_NAME_LEN: usize = 150;
pub const MAX_CATALOG_SHORT_FIELD_LEN: usize = 60;
pub const MAX_CATALOG_DESCRIPTION_LEN: usize = 2000;
pub const MAX_CATALOG_ATTR_TEXT_LEN: usize = 500;
pub const MAX_CATALOG_SELECT_CHOICES: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeDataType {
    Text,
    Number,
    Boolean,
    Select,
}

impl AttributeDataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Select => "select",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim() {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "boolean" => Ok(Self::Boolean),
            "select" => Ok(Self::Select),
            other => Err(CoreError::Validation(format!(
                "Tipo de atributo inválido: '{}'. Valores aceitos: text, number, boolean, select.",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlMode {
    Serialized,
    Quantity,
}

impl ControlMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Serialized => "serialized",
            Self::Quantity => "quantity",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim() {
            "serialized" => Ok(Self::Serialized),
            "quantity" => Ok(Self::Quantity),
            other => Err(CoreError::Validation(format!(
                "Modo de controle inválido: '{}'. Valores aceitos: serialized, quantity.",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoltageOption {
    #[serde(rename = "110")]
    V110,
    #[serde(rename = "220")]
    V220,
    #[serde(rename = "bivolt")]
    Bivolt,
}

impl VoltageOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V110 => "110",
            Self::V220 => "220",
            Self::Bivolt => "bivolt",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim() {
            "110" => Ok(Self::V110),
            "220" => Ok(Self::V220),
            "bivolt" => Ok(Self::Bivolt),
            other => Err(CoreError::Validation(format!(
                "Tensão inválida: '{}'. Valores aceitos: 110, 220, bivolt.",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Brand {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub name_normalized: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquipmentCategory {
    pub id: String,
    pub tenant_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub name_normalized: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributeDefinition {
    pub id: String,
    pub tenant_id: String,
    pub category_id: String,
    pub field_key: String,
    pub label: String,
    pub data_type: AttributeDataType,
    pub unit: Option<String>,
    pub choices: Vec<String>,
    pub is_required: bool,
    pub sort_order: i64,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InheritedAttributeDefinition {
    pub definition: AttributeDefinition,
    pub source_category_id: String,
    pub source_category_name: String,
    pub is_inherited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquipmentCategoryTreeItem {
    pub category: EquipmentCategory,
    pub depth: usize,
    pub own_attributes: Vec<AttributeDefinition>,
    pub model_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentModel {
    pub id: String,
    pub tenant_id: String,
    pub category_id: String,
    pub brand_id: Option<String>,
    pub name: String,
    pub name_sort: String,
    pub model_code: Option<String>,
    pub sku: Option<String>,
    pub ean: Option<String>,
    pub ncm: Option<String>,
    pub description: Option<String>,
    pub control_mode: ControlMode,
    pub weight_kg: Option<f64>,
    pub length_cm: Option<f64>,
    pub width_cm: Option<f64>,
    pub height_cm: Option<f64>,
    pub power_watts: Option<i64>,
    pub voltage: Option<VoltageOption>,
    pub amperage: Option<f64>,
    pub requires_case: bool,
    pub units_per_case: Option<i64>,
    pub daily_rate_cents: Option<i64>,
    pub replacement_value_cents: Option<i64>,
    pub cost_price_cents: Option<i64>,
    pub active: bool,
    pub search_text: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentModelAttributeValue {
    pub id: String,
    pub tenant_id: String,
    pub model_id: String,
    pub definition_id: String,
    pub value_text: Option<String>,
    pub value_number: Option<f64>,
    pub value_bool: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentModelListItem {
    pub model: EquipmentModel,
    pub category_name: String,
    pub brand_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentModelWithDetails {
    pub model: EquipmentModel,
    pub category_name: String,
    pub brand_name: Option<String>,
    pub effective_attributes: Vec<InheritedAttributeDefinition>,
    pub attribute_values: Vec<EquipmentModelAttributeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogListResult {
    pub items: Vec<EquipmentModelListItem>,
    pub next_cursor: Option<String>,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuggestedCategoriesResult {
    pub categories_created: usize,
    pub attributes_created: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBrandInput {
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBrandInput {
    pub name: String,
    pub active: bool,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryInput {
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryInput {
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub active: bool,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttributeDefinitionInput {
    pub category_id: String,
    pub field_key: String,
    pub label: String,
    pub data_type: AttributeDataType,
    pub unit: Option<String>,
    pub choices: Vec<String>,
    pub is_required: bool,
    pub sort_order: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAttributeDefinitionInput {
    pub field_key: String,
    pub label: String,
    pub data_type: AttributeDataType,
    pub unit: Option<String>,
    pub choices: Vec<String>,
    pub is_required: bool,
    pub sort_order: i64,
    pub active: bool,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValueInput {
    pub definition_id: String,
    pub value_text: Option<String>,
    pub value_number: Option<f64>,
    pub value_bool: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEquipmentModelInput {
    pub category_id: String,
    pub brand_id: Option<String>,
    pub name: String,
    pub model_code: Option<String>,
    pub sku: Option<String>,
    pub ean: Option<String>,
    pub ncm: Option<String>,
    pub description: Option<String>,
    pub control_mode: ControlMode,
    pub weight_kg: Option<f64>,
    pub length_cm: Option<f64>,
    pub width_cm: Option<f64>,
    pub height_cm: Option<f64>,
    pub power_watts: Option<i64>,
    pub voltage: Option<VoltageOption>,
    pub amperage: Option<f64>,
    pub requires_case: bool,
    pub units_per_case: Option<i64>,
    pub daily_rate_cents: Option<i64>,
    pub replacement_value_cents: Option<i64>,
    pub cost_price_cents: Option<i64>,
    pub active: bool,
    pub attribute_values: Vec<AttributeValueInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEquipmentModelInput {
    pub category_id: String,
    pub brand_id: Option<String>,
    pub name: String,
    pub model_code: Option<String>,
    pub sku: Option<String>,
    pub ean: Option<String>,
    pub ncm: Option<String>,
    pub description: Option<String>,
    pub control_mode: ControlMode,
    pub weight_kg: Option<f64>,
    pub length_cm: Option<f64>,
    pub width_cm: Option<f64>,
    pub height_cm: Option<f64>,
    pub power_watts: Option<i64>,
    pub voltage: Option<VoltageOption>,
    pub amperage: Option<f64>,
    pub requires_case: bool,
    pub units_per_case: Option<i64>,
    pub daily_rate_cents: Option<i64>,
    pub replacement_value_cents: Option<i64>,
    pub cost_price_cents: Option<i64>,
    pub active: bool,
    pub attribute_values: Vec<AttributeValueInput>,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogListFilter {
    pub query: Option<String>,
    pub category_id: Option<String>,
    pub brand_id: Option<String>,
    pub control_mode: Option<ControlMode>,
    pub active: Option<bool>,
    pub cursor: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogCursor {
    pub name_sort: String,
    pub id: String,
}

impl CatalogCursor {
    pub fn new(name_sort: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name_sort: name_sort.into(),
            id: id.into(),
        }
    }

    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        URL_SAFE_NO_PAD.encode(json.as_bytes())
    }

    pub fn decode(cursor_str: &str) -> Result<Self> {
        let trimmed = cursor_str.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Validation(
                "Cursor de paginação inválido ou vazio.".to_string(),
            ));
        }
        let bytes = URL_SAFE_NO_PAD.decode(trimmed).map_err(|_| {
            CoreError::Validation("Cursor de paginação inválido ou adulterado.".to_string())
        })?;
        let decoded: Self = serde_json::from_slice(&bytes)
            .map_err(|_| CoreError::Validation("Cursor de paginação malformado.".to_string()))?;
        if decoded.id.trim().is_empty() {
            return Err(CoreError::Validation(
                "Cursor de paginação inválido: identificador ausente.".to_string(),
            ));
        }
        Ok(decoded)
    }
}

fn check_single_line_text(value: &str, field_label: &str, max_len: usize) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(format!(
            "{} é obrigatório(a).",
            field_label
        )));
    }
    if trimmed.chars().count() > max_len {
        return Err(CoreError::Validation(format!(
            "{} excede o limite máximo de {} caracteres.",
            field_label, max_len
        )));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(format!(
            "{} contém caracteres de controle inválidos.",
            field_label
        )));
    }
    Ok(trimmed.to_string())
}

fn check_optional_single_line_text(
    value: Option<&str>,
    field_label: &str,
    max_len: usize,
) -> Result<Option<String>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_len {
        return Err(CoreError::Validation(format!(
            "{} excede o limite máximo de {} caracteres.",
            field_label, max_len
        )));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(format!(
            "{} contém caracteres de controle inválidos.",
            field_label
        )));
    }
    Ok(Some(trimmed.to_string()))
}

fn check_optional_multiline_text(
    value: Option<&str>,
    field_label: &str,
    max_len: usize,
) -> Result<Option<String>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_len {
        return Err(CoreError::Validation(format!(
            "{} excede o limite máximo de {} caracteres.",
            field_label, max_len
        )));
    }
    if trimmed
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(CoreError::Validation(format!(
            "{} contém caracteres de controle inválidos.",
            field_label
        )));
    }
    Ok(Some(trimmed.to_string()))
}

pub fn normalize_unique_key(value: &str) -> String {
    normalize_for_search(value)
}

pub fn validate_brand_name(name: &str) -> Result<(String, String)> {
    let clean = check_single_line_text(name, "Nome da marca", MAX_CATALOG_NAME_LEN)?;
    let normalized = normalize_unique_key(&clean);
    Ok((clean, normalized))
}

pub fn validate_category_fields(
    name: &str,
    description: Option<&str>,
) -> Result<(String, String, Option<String>)> {
    let clean_name = check_single_line_text(name, "Nome da categoria", MAX_CATALOG_NAME_LEN)?;
    let normalized = normalize_unique_key(&clean_name);
    let clean_desc =
        check_optional_multiline_text(description, "Descrição", MAX_CATALOG_DESCRIPTION_LEN)?;
    Ok((clean_name, normalized, clean_desc))
}

pub fn validate_attribute_field_key(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "Chave do atributo é obrigatória.".to_string(),
        ));
    }
    if trimmed.len() > MAX_CATALOG_SHORT_FIELD_LEN {
        return Err(CoreError::Validation(format!(
            "Chave do atributo excede {} caracteres.",
            MAX_CATALOG_SHORT_FIELD_LEN
        )));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(CoreError::Validation(
            "Chave do atributo deve conter apenas letras minúsculas, números e '_'.".to_string(),
        ));
    }
    Ok(trimmed)
}

pub fn validate_attribute_definition_fields(
    field_key: &str,
    label: &str,
    data_type: AttributeDataType,
    unit: Option<&str>,
    choices: &[String],
) -> Result<(String, String, Option<String>, Vec<String>)> {
    let clean_key = validate_attribute_field_key(field_key)?;
    let clean_label =
        check_single_line_text(label, "Rótulo do atributo", MAX_CATALOG_SHORT_FIELD_LEN)?;
    let clean_unit = check_optional_single_line_text(unit, "Unidade", MAX_CATALOG_SHORT_FIELD_LEN)?;

    let clean_choices = if data_type == AttributeDataType::Select {
        if choices.is_empty() || choices.len() > MAX_CATALOG_SELECT_CHOICES {
            return Err(CoreError::Validation(format!(
                "Atributos do tipo seleção devem ter entre 1 e {} opções.",
                MAX_CATALOG_SELECT_CHOICES
            )));
        }
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(choices.len());
        for choice in choices {
            let item =
                check_single_line_text(choice, "Opção de seleção", MAX_CATALOG_SHORT_FIELD_LEN)?;
            let norm = normalize_unique_key(&item);
            if !seen.insert(norm) {
                return Err(CoreError::Validation(format!(
                    "Opção de seleção duplicada: '{}'.",
                    item
                )));
            }
            out.push(item);
        }
        out
    } else {
        Vec::new()
    };

    Ok((clean_key, clean_label, clean_unit, clean_choices))
}

/// Valida código de barras EAN/GTIN (8, 12, 13 ou 14 dígitos) com dígito verificador GS1.
pub fn validate_ean(raw: Option<&str>) -> Result<Option<String>> {
    let Some(val) = raw else {
        return Ok(None);
    };
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let digits: String = trimmed
        .chars()
        .filter(|c| !matches!(c, ' ' | '-'))
        .collect();
    if !matches!(digits.len(), 8 | 12 | 13 | 14) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CoreError::Validation(
            "Código EAN/GTIN inválido. Informe 8, 12, 13 ou 14 dígitos numéricos.".to_string(),
        ));
    }

    let bytes = digits.as_bytes();
    let check_digit = (bytes[bytes.len() - 1] - b'0') as u32;
    let payload = &bytes[..bytes.len() - 1];
    let mut sum = 0u32;
    for (idx_from_right, &b) in payload.iter().rev().enumerate() {
        let digit = (b - b'0') as u32;
        let weight = if idx_from_right % 2 == 0 { 3 } else { 1 };
        sum += digit * weight;
    }
    let expected = (10 - (sum % 10)) % 10;
    if check_digit != expected {
        return Err(CoreError::Validation(
            "Código EAN/GTIN inválido: dígito verificador incorreto.".to_string(),
        ));
    }

    Ok(Some(digits))
}

/// Valida código NCM (8 dígitos numéricos).
pub fn validate_ncm(raw: Option<&str>) -> Result<Option<String>> {
    let Some(val) = raw else {
        return Ok(None);
    };
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let digits: String = trimmed
        .chars()
        .filter(|c| !matches!(c, '.' | ' ' | '-'))
        .collect();
    if digits.len() != 8 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CoreError::Validation(
            "Código NCM inválido. Deve conter exatamente 8 dígitos numéricos.".to_string(),
        ));
    }
    Ok(Some(digits))
}

fn check_optional_positive_f64(value: Option<f64>, field_label: &str) -> Result<Option<f64>> {
    let Some(v) = value else {
        return Ok(None);
    };
    if !v.is_finite() || v <= 0.0 {
        return Err(CoreError::Validation(format!(
            "{} deve ser um número positivo.",
            field_label
        )));
    }
    Ok(Some(v))
}

fn check_optional_non_negative_i64(value: Option<i64>, field_label: &str) -> Result<Option<i64>> {
    let Some(v) = value else {
        return Ok(None);
    };
    if v < 0 {
        return Err(CoreError::Validation(format!(
            "{} não pode ser negativo.",
            field_label
        )));
    }
    Ok(Some(v))
}

#[derive(Debug, Clone)]
pub struct ValidatedModelFields {
    pub name: String,
    pub name_sort: String,
    pub model_code: Option<String>,
    pub model_code_normalized: Option<String>,
    pub sku: Option<String>,
    pub sku_normalized: Option<String>,
    pub ean: Option<String>,
    pub ncm: Option<String>,
    pub description: Option<String>,
    pub weight_kg: Option<f64>,
    pub length_cm: Option<f64>,
    pub width_cm: Option<f64>,
    pub height_cm: Option<f64>,
    pub power_watts: Option<i64>,
    pub amperage: Option<f64>,
    pub units_per_case: Option<i64>,
    pub daily_rate_cents: Option<i64>,
    pub replacement_value_cents: Option<i64>,
    pub cost_price_cents: Option<i64>,
}

#[allow(clippy::too_many_arguments)]
pub fn validate_equipment_model_fields(
    name: &str,
    model_code: Option<&str>,
    sku: Option<&str>,
    ean: Option<&str>,
    ncm: Option<&str>,
    description: Option<&str>,
    weight_kg: Option<f64>,
    length_cm: Option<f64>,
    width_cm: Option<f64>,
    height_cm: Option<f64>,
    power_watts: Option<i64>,
    amperage: Option<f64>,
    units_per_case: Option<i64>,
    daily_rate_cents: Option<i64>,
    replacement_value_cents: Option<i64>,
    cost_price_cents: Option<i64>,
) -> Result<ValidatedModelFields> {
    let clean_name = check_single_line_text(name, "Nome do modelo", MAX_CATALOG_NAME_LEN)?;
    let name_sort = build_name_sort(&clean_name);

    let clean_code = check_optional_single_line_text(
        model_code,
        "Código do modelo",
        MAX_CATALOG_SHORT_FIELD_LEN,
    )?;
    let code_norm = clean_code.as_deref().map(normalize_unique_key);

    let clean_sku = check_optional_single_line_text(sku, "SKU", MAX_CATALOG_SHORT_FIELD_LEN)?;
    let sku_norm = clean_sku.as_deref().map(normalize_unique_key);

    let clean_ean = validate_ean(ean)?;
    let clean_ncm = validate_ncm(ncm)?;
    let clean_desc =
        check_optional_multiline_text(description, "Descrição", MAX_CATALOG_DESCRIPTION_LEN)?;

    let clean_weight = check_optional_positive_f64(weight_kg, "Peso (kg)")?;
    let clean_length = check_optional_positive_f64(length_cm, "Comprimento (cm)")?;
    let clean_width = check_optional_positive_f64(width_cm, "Largura (cm)")?;
    let clean_height = check_optional_positive_f64(height_cm, "Altura (cm)")?;
    let clean_watts = check_optional_non_negative_i64(power_watts, "Potência (W)")?;
    let clean_amperage = check_optional_positive_f64(amperage, "Corrente (A)")?;

    if let Some(units) = units_per_case {
        if units <= 0 {
            return Err(CoreError::Validation(
                "Unidades por case deve ser maior que zero.".to_string(),
            ));
        }
    }

    let clean_daily = check_optional_non_negative_i64(daily_rate_cents, "Diária padrão")?;
    let clean_repl =
        check_optional_non_negative_i64(replacement_value_cents, "Valor de reposição")?;
    let clean_cost = check_optional_non_negative_i64(cost_price_cents, "Custo de aquisição")?;

    Ok(ValidatedModelFields {
        name: clean_name,
        name_sort,
        model_code: clean_code,
        model_code_normalized: code_norm,
        sku: clean_sku,
        sku_normalized: sku_norm,
        ean: clean_ean,
        ncm: clean_ncm,
        description: clean_desc,
        weight_kg: clean_weight,
        length_cm: clean_length,
        width_cm: clean_width,
        height_cm: clean_height,
        power_watts: clean_watts,
        amperage: clean_amperage,
        units_per_case,
        daily_rate_cents: clean_daily,
        replacement_value_cents: clean_repl,
        cost_price_cents: clean_cost,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn build_catalog_search_text(
    name: &str,
    model_code: Option<&str>,
    sku: Option<&str>,
    ean: Option<&str>,
    ncm: Option<&str>,
    category_name: &str,
    brand_name: Option<&str>,
    description: Option<&str>,
) -> String {
    let mut parts = vec![
        normalize_for_search(name),
        normalize_for_search(category_name),
    ];
    for s in [model_code, sku, ean, ncm, brand_name, description]
        .into_iter()
        .flatten()
    {
        let norm = normalize_for_search(s);
        if !norm.is_empty() {
            parts.push(norm);
        }
    }
    parts.join(" ")
}

/// Valida ciclos e profundidade máxima (4 níveis) na árvore de categorias.
/// `category_id` é `Some(id)` ao atualizar uma categoria existente e `None` ao criar.
pub fn validate_category_tree_placement(
    categories: &[EquipmentCategory],
    category_id: Option<&str>,
    new_parent_id: Option<&str>,
) -> Result<usize> {
    let by_id: HashMap<&str, &EquipmentCategory> =
        categories.iter().map(|c| (c.id.as_str(), c)).collect();

    if let Some(self_id) = category_id {
        if new_parent_id == Some(self_id) {
            return Err(CoreError::Validation(
                "Uma categoria não pode ser pai de si mesma.".to_string(),
            ));
        }
    }

    let parent_depth = if let Some(pid) = new_parent_id {
        if !by_id.contains_key(pid) {
            return Err(CoreError::NotFound(
                "Categoria pai não encontrada.".to_string(),
            ));
        }
        let mut depth = 0usize;
        let mut cursor = Some(pid);
        let mut visited = HashSet::new();
        while let Some(curr_id) = cursor {
            if !visited.insert(curr_id) {
                return Err(CoreError::Validation(
                    "Ciclo detectado na hierarquia de categorias.".to_string(),
                ));
            }
            if category_id == Some(curr_id) {
                return Err(CoreError::Validation(
                    "Movimentação inválida: criaria um ciclo na hierarquia de categorias."
                        .to_string(),
                ));
            }
            depth += 1;
            if depth >= MAX_CATEGORY_DEPTH {
                return Err(CoreError::Validation(format!(
                    "Profundidade máxima de {} níveis na árvore de categorias excedida.",
                    MAX_CATEGORY_DEPTH
                )));
            }
            cursor = by_id.get(curr_id).and_then(|c| c.parent_id.as_deref());
        }
        depth
    } else {
        0
    };

    let own_depth = parent_depth + 1;
    if let Some(self_id) = category_id {
        let subtree_height = compute_subtree_height(categories, self_id);
        if parent_depth + subtree_height > MAX_CATEGORY_DEPTH {
            return Err(CoreError::Validation(format!(
                "Movimentação excederia a profundidade máxima de {} níveis com as subcategorias existentes.",
                MAX_CATEGORY_DEPTH
            )));
        }
    }

    Ok(own_depth)
}

fn compute_subtree_height(categories: &[EquipmentCategory], root_id: &str) -> usize {
    let mut max_child_height = 0usize;
    for cat in categories {
        if cat.parent_id.as_deref() == Some(root_id) {
            let h = compute_subtree_height(categories, &cat.id);
            if h > max_child_height {
                max_child_height = h;
            }
        }
    }
    1 + max_child_height
}

/// Resolve os atributos efetivos (próprios + herdados dos ancestrais, da raiz até a categoria alvo).
pub fn resolve_inherited_attributes(
    categories: &[EquipmentCategory],
    all_definitions: &[AttributeDefinition],
    target_category_id: &str,
) -> Result<Vec<InheritedAttributeDefinition>> {
    let by_id: HashMap<&str, &EquipmentCategory> =
        categories.iter().map(|c| (c.id.as_str(), c)).collect();

    if !by_id.contains_key(target_category_id) {
        return Err(CoreError::NotFound("Categoria não encontrada.".to_string()));
    }

    let mut chain = Vec::new();
    let mut cursor = Some(target_category_id);
    let mut visited = HashSet::new();
    while let Some(cid) = cursor {
        if !visited.insert(cid) {
            break;
        }
        if let Some(cat) = by_id.get(cid) {
            chain.push(*cat);
            cursor = cat.parent_id.as_deref();
        } else {
            break;
        }
    }
    chain.reverse(); // raiz -> folha

    let mut defs_by_cat: HashMap<&str, Vec<&AttributeDefinition>> = HashMap::new();
    for def in all_definitions {
        if def.deleted_at.is_none() && def.active {
            defs_by_cat
                .entry(def.category_id.as_str())
                .or_default()
                .push(def);
        }
    }

    let mut result = Vec::new();
    for cat in chain {
        if let Some(mut defs) = defs_by_cat.remove(cat.id.as_str()) {
            defs.sort_by(|a, b| {
                a.sort_order
                    .cmp(&b.sort_order)
                    .then_with(|| a.label.cmp(&b.label))
            });
            for def in defs {
                result.push(InheritedAttributeDefinition {
                    definition: def.clone(),
                    source_category_id: cat.id.clone(),
                    source_category_name: cat.name.clone(),
                    is_inherited: cat.id != target_category_id,
                });
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Clone)]
pub struct ValidatedAttributeValue {
    pub definition_id: String,
    pub value_text: Option<String>,
    pub value_number: Option<f64>,
    pub value_bool: Option<bool>,
}

pub fn validate_model_attribute_values(
    effective_defs: &[InheritedAttributeDefinition],
    inputs: &[AttributeValueInput],
) -> Result<Vec<ValidatedAttributeValue>> {
    let def_map: HashMap<&str, &AttributeDefinition> = effective_defs
        .iter()
        .map(|item| (item.definition.id.as_str(), &item.definition))
        .collect();

    let mut seen = HashSet::new();
    let mut validated = Vec::new();

    for input in inputs {
        let def_id = input.definition_id.trim();
        let def = def_map.get(def_id).ok_or_else(|| {
            CoreError::Validation(format!(
                "Atributo '{}' não pertence à categoria selecionada ou seus ancestrais.",
                def_id
            ))
        })?;
        if !seen.insert(def_id.to_string()) {
            return Err(CoreError::Validation(format!(
                "Atributo '{}' informado em duplicidade.",
                def.label
            )));
        }

        match def.data_type {
            AttributeDataType::Text => {
                let clean = check_optional_single_line_text(
                    input.value_text.as_deref(),
                    &def.label,
                    MAX_CATALOG_ATTR_TEXT_LEN,
                )?;
                if let Some(text) = clean {
                    validated.push(ValidatedAttributeValue {
                        definition_id: def.id.clone(),
                        value_text: Some(text),
                        value_number: None,
                        value_bool: None,
                    });
                }
            }
            AttributeDataType::Number => {
                if let Some(num) = input.value_number {
                    if !num.is_finite() {
                        return Err(CoreError::Validation(format!(
                            "Valor numérico inválido para '{}'.",
                            def.label
                        )));
                    }
                    validated.push(ValidatedAttributeValue {
                        definition_id: def.id.clone(),
                        value_text: None,
                        value_number: Some(num),
                        value_bool: None,
                    });
                }
            }
            AttributeDataType::Boolean => {
                if let Some(b) = input.value_bool {
                    validated.push(ValidatedAttributeValue {
                        definition_id: def.id.clone(),
                        value_text: None,
                        value_number: None,
                        value_bool: Some(b),
                    });
                }
            }
            AttributeDataType::Select => {
                if let Some(text) = check_optional_single_line_text(
                    input.value_text.as_deref(),
                    &def.label,
                    MAX_CATALOG_SHORT_FIELD_LEN,
                )? {
                    if !def.choices.iter().any(|c| c == &text) {
                        return Err(CoreError::Validation(format!(
                            "Opção '{}' inválida para o atributo '{}'.",
                            text, def.label
                        )));
                    }
                    validated.push(ValidatedAttributeValue {
                        definition_id: def.id.clone(),
                        value_text: Some(text),
                        value_number: None,
                        value_bool: None,
                    });
                }
            }
        }
    }

    let provided_ids: HashSet<&str> = validated.iter().map(|v| v.definition_id.as_str()).collect();
    for item in effective_defs {
        if item.definition.is_required && !provided_ids.contains(item.definition.id.as_str()) {
            return Err(CoreError::Validation(format!(
                "Atributo obrigatório '{}' não preenchido.",
                item.definition.label
            )));
        }
    }

    Ok(validated)
}
