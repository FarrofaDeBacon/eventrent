use crate::domain::document::{
    mask_document_for_audit, mask_email_for_audit, mask_phone_for_audit, DocumentType,
};
use crate::error::{CoreError, Result as CoreResult};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonType {
    Individual,
    Legal,
}

impl std::fmt::Display for PersonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Individual => write!(f, "individual"),
            Self::Legal => write!(f, "legal"),
        }
    }
}

impl std::str::FromStr for PersonType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "individual" | "fisica" | "pf" => Ok(Self::Individual),
            "legal" | "juridica" | "pj" => Ok(Self::Legal),
            other => Err(format!("Tipo de pessoa inválido: '{other}'")),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Customer {
    pub id: String,
    pub tenant_id: String,
    pub person_type: PersonType,
    pub name: String,
    pub trade_name: Option<String>,
    pub document_type: DocumentType,
    pub document_number: Option<String>,
    pub state_registration: Option<String>,
    pub municipal_registration: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    pub search_text: String,
    pub name_sort: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerContact {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    pub is_primary: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerAddress {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub label: String,
    pub street: String,
    pub number: String,
    pub complement: Option<String>,
    pub neighborhood: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub is_primary: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerWithDetails {
    pub customer: Customer,
    pub contacts: Vec<CustomerContact>,
    pub addresses: Vec<CustomerAddress>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateContactInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    pub is_primary: bool,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateAddressInput {
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    pub street: String,
    pub number: String,
    pub complement: Option<String>,
    pub neighborhood: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    #[serde(default)]
    pub country: Option<String>,
    pub is_primary: bool,
}

pub type UpdateContactInput = CreateContactInput;
pub type UpdateAddressInput = CreateAddressInput;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateCustomerInput {
    pub person_type: PersonType,
    pub name: String,
    pub trade_name: Option<String>,
    #[serde(default)]
    pub document_type: DocumentType,
    pub document_number: Option<String>,
    pub state_registration: Option<String>,
    pub municipal_registration: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub contacts: Vec<CreateContactInput>,
    #[serde(default)]
    pub addresses: Vec<CreateAddressInput>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCustomerInput {
    pub person_type: PersonType,
    pub name: String,
    pub trade_name: Option<String>,
    #[serde(default)]
    pub document_type: DocumentType,
    pub document_number: Option<String>,
    pub state_registration: Option<String>,
    pub municipal_registration: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub contacts: Vec<CreateContactInput>,
    #[serde(default)]
    pub addresses: Vec<CreateAddressInput>,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CustomerFilter {
    pub query: Option<String>,
    pub person_type: Option<PersonType>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerPage {
    pub items: Vec<Customer>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerCursor {
    pub name_sort: String,
    pub id: String,
}

impl CustomerCursor {
    pub fn new(name_sort: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name_sort: name_sort.into(),
            id: id.into(),
        }
    }

    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        BASE64_URL_SAFE_NO_PAD.encode(json.as_bytes())
    }

    pub fn decode(cursor_str: &str) -> CoreResult<Self> {
        let trimmed = cursor_str.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Validation(
                "Cursor de paginação inválido ou vazio.".to_string(),
            ));
        }
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(trimmed).map_err(|_| {
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

/// Remove acentos de caracteres Unicode (suportando formas precompostas NFC e decompostas NFD U+0300..U+036F) e converte para minúsculas.
pub fn remove_diacritics(input: &str) -> String {
    input
        .chars()
        .filter(|&c| !('\u{0300}'..='\u{036f}').contains(&c))
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'y',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Normaliza um texto para busca textual semântica:
/// - Converte para minúsculas e remove diacríticos (acentos).
/// - Colapsa múltiplos espaços em branco adjacentes em um único espaço.
/// - Trim das extremidades.
pub fn normalize_for_search(input: &str) -> String {
    let unaccented = remove_diacritics(input);
    let mut words = Vec::new();
    for word in unaccented.split_whitespace() {
        if !word.is_empty() {
            words.push(word);
        }
    }
    words.join(" ")
}

/// Gera o `name_sort` padronizado para ordenação de nomes sem sensibilidade a maiúsculas ou acentos.
pub fn build_name_sort(name: &str) -> String {
    normalize_for_search(name)
}

/// Constrói o conteúdo da coluna indexada `search_text` concatenando dados essenciais do cliente.
pub fn build_customer_search_text(
    name: &str,
    trade_name: Option<&str>,
    document_raw: &str,
    document_formatted: &str,
    contact_terms: &[String],
) -> String {
    let mut parts = Vec::new();

    parts.push(normalize_for_search(name));

    if let Some(trade) = trade_name {
        let norm_trade = normalize_for_search(trade);
        if !norm_trade.is_empty() {
            parts.push(norm_trade);
        }
    }

    // Inclui tanto o documento cru quanto formatado para matching imediato
    parts.push(document_raw.to_ascii_lowercase());
    parts.push(document_formatted.to_ascii_lowercase());

    for term in contact_terms {
        let norm = normalize_for_search(term);
        if !norm.is_empty() {
            parts.push(norm);
        }
    }

    parts.join(" ")
}

impl fmt::Debug for Customer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Customer")
            .field("id", &self.id)
            .field("tenant_id", &self.tenant_id)
            .field("person_type", &self.person_type)
            .field("name", &self.name)
            .field("document_type", &self.document_type)
            .field(
                "document_number",
                &self.document_number.as_deref().map(mask_document_for_audit),
            )
            .field("active", &self.active)
            .field("search_text", &"[REDACTED]")
            .field("name_sort", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Debug for CustomerContact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomerContact")
            .field("id", &self.id)
            .field("tenant_id", &self.tenant_id)
            .field("customer_id", &self.customer_id)
            .field("name", &self.name)
            .field("email", &self.email.as_deref().map(mask_email_for_audit))
            .field("phone", &self.phone.as_deref().map(mask_phone_for_audit))
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

impl fmt::Debug for CustomerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomerAddress")
            .field("id", &self.id)
            .field("tenant_id", &self.tenant_id)
            .field("customer_id", &self.customer_id)
            .field("label", &self.label)
            .field("street", &"[REDACTED]")
            .field("number", &"[REDACTED]")
            .field("city", &self.city)
            .field("state", &self.state)
            .field("postal_code", &"[REDACTED]")
            .field("country", &self.country)
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

impl fmt::Debug for CreateContactInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateContactInput")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("email", &self.email.as_deref().map(mask_email_for_audit))
            .field("phone", &self.phone.as_deref().map(mask_phone_for_audit))
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

impl fmt::Debug for CreateAddressInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateAddressInput")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("street", &"[REDACTED]")
            .field("number", &"[REDACTED]")
            .field("city", &self.city)
            .field("state", &self.state)
            .field("postal_code", &"[REDACTED]")
            .field("country", &self.country)
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

impl fmt::Debug for CreateCustomerInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateCustomerInput")
            .field("person_type", &self.person_type)
            .field("name", &self.name)
            .field("document_type", &self.document_type)
            .field(
                "document_number",
                &self.document_number.as_deref().map(mask_document_for_audit),
            )
            .field("contacts_count", &self.contacts.len())
            .field("addresses_count", &self.addresses.len())
            .finish()
    }
}

impl fmt::Debug for UpdateCustomerInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateCustomerInput")
            .field("person_type", &self.person_type)
            .field("name", &self.name)
            .field("document_type", &self.document_type)
            .field(
                "document_number",
                &self.document_number.as_deref().map(mask_document_for_audit),
            )
            .field("contacts_count", &self.contacts.len())
            .field("addresses_count", &self.addresses.len())
            .finish()
    }
}

impl fmt::Debug for CustomerFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomerFilter")
            .field("query", &self.query.as_ref().map(|_| "[REDACTED]"))
            .field("person_type", &self.person_type)
            .field("cursor", &self.cursor.as_ref().map(|_| "[REDACTED]"))
            .field("limit", &self.limit)
            .finish()
    }
}
