use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

pub const MAX_WAREHOUSE_NAME_LEN: usize = 120;
pub const MAX_WAREHOUSE_CODE_LEN: usize = 30;
pub const MAX_SERIAL_NUMBER_LEN: usize = 80;
pub const MAX_PATRIMONY_TAG_LEN: usize = 30;
pub const MAX_NOTES_LEN: usize = 2000;
pub const MAX_REASON_LEN: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
            "maintenance" => Ok(Self::Maintenance),
            "damaged" => Ok(Self::Damaged),
            "lost" => Ok(Self::Lost),
            "retired" => Ok(Self::Retired),
            other => Err(CoreError::Validation(format!(
                "Status de unidade inválido: '{other}'. Valores permitidos: available, allocated, in_transit, maintenance, damaged, lost, retired."
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
    RentalOut,
    TransferOut,
    TransferIn,
    MaintenanceOut,
    MaintenanceIn,
    LossOut,
    Adjustment,
}

impl StockMovementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PurchaseIn => "purchase_in",
            Self::ReturnIn => "return_in",
            Self::RentalOut => "rental_out",
            Self::TransferOut => "transfer_out",
            Self::TransferIn => "transfer_in",
            Self::MaintenanceOut => "maintenance_out",
            Self::MaintenanceIn => "maintenance_in",
            Self::LossOut => "loss_out",
            Self::Adjustment => "adjustment",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "purchase_in" => Ok(Self::PurchaseIn),
            "return_in" => Ok(Self::ReturnIn),
            "rental_out" => Ok(Self::RentalOut),
            "transfer_out" => Ok(Self::TransferOut),
            "transfer_in" => Ok(Self::TransferIn),
            "maintenance_out" => Ok(Self::MaintenanceOut),
            "maintenance_in" => Ok(Self::MaintenanceIn),
            "loss_out" => Ok(Self::LossOut),
            "adjustment" => Ok(Self::Adjustment),
            other => Err(CoreError::Validation(format!(
                "Tipo de movimentação de estoque inválido: '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitMediaType {
    DamagePhoto,
    InspectionPhoto,
    SerialPhoto,
    InvoiceDoc,
}

impl UnitMediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DamagePhoto => "damage_photo",
            Self::InspectionPhoto => "inspection_photo",
            Self::SerialPhoto => "serial_photo",
            Self::InvoiceDoc => "invoice_doc",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_lowercase().as_str() {
            "damage_photo" => Ok(Self::DamagePhoto),
            "inspection_photo" => Ok(Self::InspectionPhoto),
            "serial_photo" => Ok(Self::SerialPhoto),
            "invoice_doc" => Ok(Self::InvoiceDoc),
            other => Err(CoreError::Validation(format!(
                "Tipo de mídia de unidade física inválido: '{other}'."
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
    pub quantity_allocated: i64,
    pub quantity_maintenance: i64,
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
pub struct EquipmentUnitMedia {
    pub id: String,
    pub tenant_id: String,
    pub equipment_unit_id: String,
    pub blob_id: String,
    pub media_type: UnitMediaType,
    pub caption: Option<String>,
    pub created_at: String,
    pub created_by: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEquipmentUnitInput {
    pub warehouse_id: Option<String>,
    pub serial_number: Option<String>,
    pub status: Option<UnitStatus>,
    pub physical_condition: Option<PhysicalCondition>,
    pub notes: Option<String>,
    pub acquisition_date: Option<String>,
    pub warranty_expires_at: Option<String>,
    pub active: Option<bool>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachUnitMediaInput {
    pub equipment_unit_id: String,
    pub blob_id: String,
    pub media_type: UnitMediaType,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquipmentUnitFilter {
    pub equipment_model_id: Option<String>,
    pub warehouse_id: Option<String>,
    pub status: Option<UnitStatus>,
    pub search: Option<String>,
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
    pub media: Vec<EquipmentUnitMedia>,
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

pub fn validate_equipment_unit_input(
    model_id: &str,
    serial_number: Option<&str>,
    notes: Option<&str>,
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
        StockMovementType::RentalOut => {
            if from_wh.is_none() {
                return Err(CoreError::Validation(
                    "Saída para locação exige depósito de origem.".to_string(),
                ));
            }
        }
        StockMovementType::ReturnIn => {
            if to_wh.is_none() {
                return Err(CoreError::Validation(
                    "Retorno de locação exige depósito de destino.".to_string(),
                ));
            }
        }
        StockMovementType::MaintenanceOut => {
            if from_wh.is_none() {
                return Err(CoreError::Validation(
                    "Envio para manutenção exige depósito de origem.".to_string(),
                ));
            }
        }
        StockMovementType::MaintenanceIn => {
            if to_wh.is_none() {
                return Err(CoreError::Validation(
                    "Retorno de manutenção exige depósito de destino.".to_string(),
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
            UnitStatus::parse("allocated").unwrap(),
            UnitStatus::Allocated
        );
        assert_eq!(
            UnitStatus::parse("in_transit").unwrap(),
            UnitStatus::InTransit
        );
        assert_eq!(
            UnitStatus::parse("maintenance").unwrap(),
            UnitStatus::Maintenance
        );
        assert_eq!(UnitStatus::parse("damaged").unwrap(), UnitStatus::Damaged);
        assert_eq!(UnitStatus::parse("lost").unwrap(), UnitStatus::Lost);
        assert_eq!(UnitStatus::parse("retired").unwrap(), UnitStatus::Retired);
        assert!(UnitStatus::parse("invalid_status").is_err());
    }

    #[test]
    fn test_physical_condition_parsing() {
        assert_eq!(
            PhysicalCondition::parse("new").unwrap(),
            PhysicalCondition::New
        );
        assert_eq!(
            PhysicalCondition::parse("good").unwrap(),
            PhysicalCondition::Good
        );
        assert_eq!(
            PhysicalCondition::parse("fair").unwrap(),
            PhysicalCondition::Fair
        );
        assert_eq!(
            PhysicalCondition::parse("poor").unwrap(),
            PhysicalCondition::Poor
        );
        assert_eq!(
            PhysicalCondition::parse("damaged").unwrap(),
            PhysicalCondition::Damaged
        );
        assert!(PhysicalCondition::parse("broken").is_err());
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
