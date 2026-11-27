use serde::{Deserialize, Serialize};

use crate::domain::user::SessionInfo;
use crate::error::{CoreError, Result};

pub const MAX_WAREHOUSE_NAME_LEN: usize = 120;
pub const MAX_WAREHOUSE_CODE_LEN: usize = 30;
pub const MAX_SERIAL_NUMBER_LEN: usize = 80;
pub const MAX_PATRIMONY_TAG_LEN: usize = 30;
pub const MAX_NOTES_LEN: usize = 2000;
pub const MAX_REASON_LEN: usize = 500;
pub const MAX_LOCATION_NOTE_LEN: usize = 250;
pub const MAX_INVOICE_NUMBER_LEN: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitStatus {
    Available,
    Maintenance,
    Damaged,
    Lost,
    Decommissioned,
}

impl UnitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Maintenance => "maintenance",
            Self::Damaged => "damaged",
            Self::Lost => "lost",
            Self::Decommissioned => "decommissioned",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "available" => Ok(Self::Available),
            "maintenance" => Ok(Self::Maintenance),
            "damaged" => Ok(Self::Damaged),
            "lost" => Ok(Self::Lost),
            "decommissioned" => Ok(Self::Decommissioned),
            other => Err(CoreError::Validation(format!(
                "Status de unidade inválido: '{other}'. Valores permitidos: available, maintenance, damaged, lost, decommissioned."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UnitOrigin {
    #[default]
    Own,
    ThirdParty,
}

impl UnitOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::ThirdParty => "third_party",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "own" => Ok(Self::Own),
            "third_party" => Ok(Self::ThirdParty),
            other => Err(CoreError::Validation(format!(
                "Origem de unidade inválida: '{other}'. Valores permitidos: own, third_party."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalCondition {
    New,
    Good,
    Fair,
    Poor,
    Damaged,
}

impl PhysicalCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Good => "good",
            Self::Fair => "fair",
            Self::Poor => "poor",
            Self::Damaged => "damaged",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "new" => Ok(Self::New),
            "good" => Ok(Self::Good),
            "fair" => Ok(Self::Fair),
            "poor" => Ok(Self::Poor),
            "damaged" => Ok(Self::Damaged),
            other => Err(CoreError::Validation(format!(
                "Condição física inválida: '{other}'. Valores permitidos: new, good, fair, poor, damaged."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockMovementType {
    PurchaseIn,
    ReturnIn,
    TransferOut,
    TransferIn,
    StatusChange,
    LossOut,
    DecommissionOut,
    Adjustment,
}

impl StockMovementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PurchaseIn => "purchase_in",
            Self::ReturnIn => "return_in",
            Self::TransferOut => "transfer_out",
            Self::TransferIn => "transfer_in",
            Self::StatusChange => "status_change",
            Self::LossOut => "loss_out",
            Self::DecommissionOut => "decommission_out",
            Self::Adjustment => "adjustment",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "purchase_in" => Ok(Self::PurchaseIn),
            "return_in" => Ok(Self::ReturnIn),
            "transfer_out" => Ok(Self::TransferOut),
            "transfer_in" => Ok(Self::TransferIn),
            "status_change" => Ok(Self::StatusChange),
            "loss_out" => Ok(Self::LossOut),
            "decommission_out" => Ok(Self::DecommissionOut),
            "adjustment" => Ok(Self::Adjustment),
            other => Err(CoreError::Validation(format!(
                "Tipo de movimentação de estoque inválido: '{other}'."
            ))),
        }
    }
}

pub fn format_patrimony_tag(number: i64) -> String {
    format!("EQ{:06}", number.max(0))
}

pub fn parse_patrimony_number(tag: &str) -> Option<i64> {
    let trimmed = tag.trim();
    if trimmed.len() >= 3 && trimmed.starts_with("EQ") {
        trimmed[2..].parse::<i64>().ok()
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub is_default: bool,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentUnit {
    pub id: String,
    pub tenant_id: String,
    pub equipment_model_id: String,
    pub warehouse_id: String,
    pub patrimony_tag: String,
    pub serial_number: Option<String>,
    pub status: UnitStatus,
    pub physical_condition: PhysicalCondition,
    pub notes: Option<String>,
    pub acquisition_date: Option<String>,
    pub warranty_expires_at: Option<String>,
    pub acquisition_cost_cents: Option<i64>,
    pub supplier_id: Option<String>,
    pub invoice_number: Option<String>,
    pub origin: UnitOrigin,
    pub return_by: Option<String>,
    pub usage_hours: i64,
    pub location_note: Option<String>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockBalance {
    pub id: String,
    pub tenant_id: String,
    pub warehouse_id: String,
    pub equipment_model_id: String,
    pub quantity_on_hand: i64,
    pub quantity_maintenance: i64,
    pub min_quantity: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: String,
    pub tenant_id: String,
    pub equipment_model_id: String,
    pub equipment_unit_id: Option<String>,
    pub from_warehouse_id: Option<String>,
    pub to_warehouse_id: Option<String>,
    pub movement_type: StockMovementType,
    pub quantity: i64,
    pub reason: String,
    pub reference_id: Option<String>,
    pub performed_by_user_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWarehouseInput {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWarehouseInput {
    pub expected_updated_at: String,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEquipmentUnitInput {
    pub equipment_model_id: String,
    pub warehouse_id: Option<String>,
    pub serial_number: Option<String>,
    pub physical_condition: Option<PhysicalCondition>,
    pub notes: Option<String>,
    pub acquisition_date: Option<String>,
    pub warranty_expires_at: Option<String>,
    pub acquisition_cost_cents: Option<i64>,
    pub supplier_id: Option<String>,
    pub invoice_number: Option<String>,
    pub origin: Option<UnitOrigin>,
    pub return_by: Option<String>,
    pub usage_hours: Option<i64>,
    pub location_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEquipmentUnitInput {
    pub expected_updated_at: String,
    pub warehouse_id: Option<String>,
    pub serial_number: Option<String>,
    pub status: Option<UnitStatus>,
    pub physical_condition: Option<PhysicalCondition>,
    pub notes: Option<String>,
    pub acquisition_date: Option<String>,
    pub warranty_expires_at: Option<String>,
    pub acquisition_cost_cents: Option<i64>,
    pub supplier_id: Option<String>,
    pub invoice_number: Option<String>,
    pub origin: Option<UnitOrigin>,
    pub return_by: Option<String>,
    pub usage_hours: Option<i64>,
    pub location_note: Option<String>,
    pub active: Option<bool>,
    pub status_change_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecommissionEquipmentUnitInput {
    pub expected_updated_at: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordStockMovementInput {
    pub equipment_model_id: String,
    pub equipment_unit_id: Option<String>,
    pub from_warehouse_id: Option<String>,
    pub to_warehouse_id: Option<String>,
    pub movement_type: StockMovementType,
    pub quantity: i64,
    pub reason: String,
    pub reference_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquipmentUnitFilter {
    pub equipment_model_id: Option<String>,
    pub warehouse_id: Option<String>,
    pub status: Option<UnitStatus>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockMovementFilter {
    pub equipment_model_id: Option<String>,
    pub equipment_unit_id: Option<String>,
    pub warehouse_id: Option<String>,
    pub movement_type: Option<StockMovementType>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentUnitDetails {
    pub unit: EquipmentUnit,
    pub model_name: String,
    pub warehouse_name: String,
    pub model_thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentUnitListPage {
    pub items: Vec<EquipmentUnitDetails>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

// ==========================================
// REDAÇÃO FINANCEIRA CENTRALIZADA (R5)
// ==========================================

pub fn can_view_financials(session: &SessionInfo) -> bool {
    session.role_slug == "owner" || session.has_permission("finance:read")
}

pub fn can_manage_financials(session: &SessionInfo) -> bool {
    session.role_slug == "owner" || session.has_permission("finance:manage")
}

pub fn sanitize_unit_financials(unit: &mut EquipmentUnit, session: &SessionInfo) {
    if !can_view_financials(session) {
        unit.acquisition_cost_cents = None;
        unit.supplier_id = None;
        unit.invoice_number = None;
    }
}

pub fn validate_financial_mutation_permissions(
    session: &SessionInfo,
    cost: Option<i64>,
    supplier_id: Option<&str>,
    invoice_number: Option<&str>,
) -> Result<()> {
    if !can_manage_financials(session) {
        let has_cost = cost.is_some();
        let has_supplier = supplier_id.map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_invoice = invoice_number
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if has_cost || has_supplier || has_invoice {
            return Err(CoreError::Forbidden(
                "Permissão insuficiente para alterar campos financeiros da unidade (requer 'finance:manage' ou Proprietário).".to_string(),
            ));
        }
    }
    Ok(())
}

// ==========================================
// TRANSIÇÃO DE ESTADOS DA UNIDADE (R3)
// ==========================================

pub fn validate_status_transition(
    current: UnitStatus,
    next: UnitStatus,
    reason: Option<&str>,
) -> Result<()> {
    if current == next {
        return Ok(());
    }
    if current == UnitStatus::Decommissioned {
        return Err(CoreError::Conflict(
            "Unidade baixada (decommissioned) é terminal e não pode ter seu status alterado."
                .to_string(),
        ));
    }
    if current == UnitStatus::Lost && next == UnitStatus::Available {
        let r = reason.unwrap_or("").trim();
        if r.is_empty() {
            return Err(CoreError::Validation(
                "Reativação de unidade com status 'lost' para 'available' exige justificativa obrigatória.".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_warehouse_input(name: &str, code: &str) -> Result<()> {
    let name_trimmed = name.trim();
    if name_trimmed.is_empty() {
        return Err(CoreError::Validation(
            "Nome do depósito é obrigatório.".to_string(),
        ));
    }
    if name_trimmed.len() > MAX_WAREHOUSE_NAME_LEN {
        return Err(CoreError::Validation(format!(
            "Nome do depósito não pode exceder {MAX_WAREHOUSE_NAME_LEN} caracteres."
        )));
    }
    let code_trimmed = code.trim();
    if code_trimmed.is_empty() {
        return Err(CoreError::Validation(
            "Código do depósito é obrigatório.".to_string(),
        ));
    }
    if code_trimmed.len() > MAX_WAREHOUSE_CODE_LEN {
        return Err(CoreError::Validation(format!(
            "Código do depósito não pode exceder {MAX_WAREHOUSE_CODE_LEN} caracteres."
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn validate_equipment_unit_input(
    model_id: &str,
    serial_number: Option<&str>,
    notes: Option<&str>,
    origin: UnitOrigin,
    supplier_id: Option<&str>,
    return_by: Option<&str>,
    cost: Option<i64>,
    location_note: Option<&str>,
    invoice_number: Option<&str>,
) -> Result<()> {
    if model_id.trim().is_empty() {
        return Err(CoreError::Validation(
            "ID do modelo é obrigatório.".to_string(),
        ));
    }
    if let Some(sn) = serial_number {
        if sn.trim().len() > MAX_SERIAL_NUMBER_LEN {
            return Err(CoreError::Validation(format!(
                "Número de série não pode exceder {MAX_SERIAL_NUMBER_LEN} caracteres."
            )));
        }
    }
    if let Some(n) = notes {
        if n.trim().len() > MAX_NOTES_LEN {
            return Err(CoreError::Validation(format!(
                "Observações não podem exceder {MAX_NOTES_LEN} caracteres."
            )));
        }
    }
    if let Some(c) = cost {
        if c < 0 {
            return Err(CoreError::Validation(
                "Custo de aquisição não pode ser negativo.".to_string(),
            ));
        }
    }
    if let Some(loc) = location_note {
        if loc.trim().len() > MAX_LOCATION_NOTE_LEN {
            return Err(CoreError::Validation(format!(
                "Nota de localização não pode exceder {MAX_LOCATION_NOTE_LEN} caracteres."
            )));
        }
    }
    if let Some(inv) = invoice_number {
        if inv.trim().len() > MAX_INVOICE_NUMBER_LEN {
            return Err(CoreError::Validation(format!(
                "Número da nota fiscal não pode exceder {MAX_INVOICE_NUMBER_LEN} caracteres."
            )));
        }
    }
    if origin == UnitOrigin::ThirdParty {
        let has_supplier = supplier_id.map(|s| !s.trim().is_empty()).unwrap_or(false);
        if !has_supplier {
            return Err(CoreError::Validation(
                "Unidade de terceiro (third_party) exige vincular fornecedor (supplier_id)."
                    .to_string(),
            ));
        }
        let has_return = return_by.map(|r| !r.trim().is_empty()).unwrap_or(false);
        if !has_return {
            return Err(CoreError::Validation(
                "Unidade de terceiro (third_party) exige data limite de devolução (return_by)."
                    .to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_stock_movement_input(
    model_id: &str,
    quantity: i64,
    reason: &str,
    from_wh: Option<&str>,
    to_wh: Option<&str>,
    m_type: StockMovementType,
) -> Result<()> {
    if model_id.trim().is_empty() {
        return Err(CoreError::Validation(
            "ID do modelo é obrigatório.".to_string(),
        ));
    }
    if quantity <= 0 {
        return Err(CoreError::Validation(
            "Quantidade movimentada deve ser maior que zero.".to_string(),
        ));
    }
    let reason_trimmed = reason.trim();
    if reason_trimmed.is_empty() {
        return Err(CoreError::Validation(
            "Motivo da movimentação de estoque é obrigatório.".to_string(),
        ));
    }
    if reason_trimmed.len() > MAX_REASON_LEN {
        return Err(CoreError::Validation(format!(
            "Motivo não pode exceder {MAX_REASON_LEN} caracteres."
        )));
    }

    match m_type {
        StockMovementType::TransferOut | StockMovementType::TransferIn => {
            if from_wh.is_none() || to_wh.is_none() {
                return Err(CoreError::Validation(
                    "Transferências exigem depósito de origem e depósito de destino.".to_string(),
                ));
            }
            if from_wh == to_wh {
                return Err(CoreError::Validation(
                    "Depósito de origem e destino não podem ser iguais em transferência."
                        .to_string(),
                ));
            }
        }
        StockMovementType::PurchaseIn => {
            if to_wh.is_none() {
                return Err(CoreError::Validation(
                    "Entrada por compra exige especificação do depósito de destino.".to_string(),
                ));
            }
        }
        StockMovementType::ReturnIn => {
            if to_wh.is_none() {
                return Err(CoreError::Validation(
                    "Retorno de locação ou terceiros exige depósito de destino.".to_string(),
                ));
            }
        }
        StockMovementType::LossOut => {
            if from_wh.is_none() {
                return Err(CoreError::Validation(
                    "Baixa por perda/extravio exige depósito de origem.".to_string(),
                ));
            }
        }
        StockMovementType::DecommissionOut => {
            if from_wh.is_none() {
                return Err(CoreError::Validation(
                    "Baixa por descarte/sucateamento exige depósito de origem.".to_string(),
                ));
            }
        }
        StockMovementType::StatusChange => {
            // Mudança de status não exige transferência entre depósitos diferentes
        }
        StockMovementType::Adjustment => {
            if from_wh.is_none() && to_wh.is_none() {
                return Err(CoreError::Validation(
                    "Ajuste de inventário exige informar ao menos um depósito.".to_string(),
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patrimony_tag_formatting_and_parsing() {
        assert_eq!(format_patrimony_tag(1), "EQ000001");
        assert_eq!(format_patrimony_tag(42), "EQ000042");
        assert_eq!(format_patrimony_tag(999999), "EQ999999");

        assert_eq!(parse_patrimony_number("EQ000001"), Some(1));
        assert_eq!(parse_patrimony_number("EQ000042"), Some(42));
        assert_eq!(parse_patrimony_number("EQ999999"), Some(999999));
        assert_eq!(parse_patrimony_number("INVALID"), None);
    }

    #[test]
    fn test_unit_status_parsing() {
        assert_eq!(
            UnitStatus::parse("available").unwrap(),
            UnitStatus::Available
        );
        assert_eq!(
            UnitStatus::parse("maintenance").unwrap(),
            UnitStatus::Maintenance
        );
        assert_eq!(UnitStatus::parse("damaged").unwrap(), UnitStatus::Damaged);
        assert_eq!(UnitStatus::parse("lost").unwrap(), UnitStatus::Lost);
        assert_eq!(
            UnitStatus::parse("decommissioned").unwrap(),
            UnitStatus::Decommissioned
        );
        assert!(UnitStatus::parse("allocated").is_err());
        assert!(UnitStatus::parse("in_transit").is_err());
        assert!(UnitStatus::parse("retired").is_err());
    }

    #[test]
    fn test_unit_origin_parsing() {
        assert_eq!(UnitOrigin::parse("own").unwrap(), UnitOrigin::Own);
        assert_eq!(
            UnitOrigin::parse("third_party").unwrap(),
            UnitOrigin::ThirdParty
        );
        assert!(UnitOrigin::parse("invalid").is_err());
    }

    #[test]
    fn test_status_transitions() {
        assert!(
            validate_status_transition(UnitStatus::Available, UnitStatus::Maintenance, None)
                .is_ok()
        );
        assert!(
            validate_status_transition(UnitStatus::Maintenance, UnitStatus::Available, None)
                .is_ok()
        );
        // Decommissioned é terminal
        assert!(validate_status_transition(
            UnitStatus::Decommissioned,
            UnitStatus::Available,
            None
        )
        .is_err());
        // Lost para Available exige justificativa
        assert!(validate_status_transition(UnitStatus::Lost, UnitStatus::Available, None).is_err());
        assert!(
            validate_status_transition(UnitStatus::Lost, UnitStatus::Available, Some("")).is_err()
        );
        assert!(validate_status_transition(
            UnitStatus::Lost,
            UnitStatus::Available,
            Some("Achado")
        )
        .is_ok());
    }

    #[test]
    fn test_stock_movement_validation() {
        assert!(validate_stock_movement_input(
            "mod_1",
            10,
            "Compra",
            None,
            Some("wh_1"),
            StockMovementType::PurchaseIn
        )
        .is_ok());
        assert!(validate_stock_movement_input(
            "mod_1",
            0,
            "Compra",
            None,
            Some("wh_1"),
            StockMovementType::PurchaseIn
        )
        .is_err());
        assert!(validate_stock_movement_input(
            "mod_1",
            5,
            "",
            None,
            Some("wh_1"),
            StockMovementType::PurchaseIn
        )
        .is_err());
        // Transferência com mesma origem e destino deve falhar
        assert!(validate_stock_movement_input(
            "mod_1",
            5,
            "Transferência",
            Some("wh_1"),
            Some("wh_1"),
            StockMovementType::TransferOut
        )
        .is_err());
    }
}
