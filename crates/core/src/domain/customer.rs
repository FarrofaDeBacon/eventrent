use crate::domain::document::DocumentType;
use base64::prelude::*;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateContactInput {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateAddressInput {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
        BASE64_STANDARD.encode(json.as_bytes())
    }

    pub fn decode(cursor_str: &str) -> Option<Self> {
        let bytes = BASE64_STANDARD.decode(cursor_str.trim()).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

/// Remove acentos de caracteres Unicode em português/latim e converte para minúsculas.
pub fn remove_diacritics(input: &str) -> String {
    input
        .chars()
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
