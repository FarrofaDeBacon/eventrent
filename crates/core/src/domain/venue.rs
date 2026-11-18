use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::customer::{build_name_sort, normalize_for_search, CustomerCursor};
use crate::domain::document::{mask_email_for_audit, mask_phone_for_audit};
use crate::domain::settings::AllocationSettings;
use crate::domain::validation::{
    normalize_and_validate_phone, normalize_and_validate_postal_code, validate_brazilian_state,
    validate_country_code, validate_email,
};
use crate::error::{CoreError, Result as CoreResult};

pub const MAX_VENUE_NAME_LEN: usize = 150;
pub const MAX_VENUE_SPACE_NAME_LEN: usize = 100;
pub const MAX_VENUE_TECH_FIELD_LEN: usize = 2000;
pub const MAX_VENUE_SPACES: usize = 100;
pub const MAX_VENUE_CONTACTS: usize = 20;
pub const MAX_INTERNAL_TRANSFER_MINUTES: i64 = 240;

pub type VenueCursor = CustomerCursor;

fn default_true() -> bool {
    true
}

fn default_country_br() -> String {
    "BR".to_string()
}

fn has_disallowed_control_chars(s: &str, allow_multiline: bool) -> bool {
    s.chars().any(|c| {
        if allow_multiline {
            c.is_control() && c != '\n' && c != '\r' && c != '\t'
        } else {
            c.is_control()
        }
    })
}

fn validate_optional_text(
    value: Option<String>,
    field_label: &str,
    max_len: usize,
    allow_multiline: bool,
) -> CoreResult<Option<String>> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if has_disallowed_control_chars(trimmed, allow_multiline) {
                return Err(CoreError::Validation(format!(
                    "{field_label} contém caracteres de controle inválidos."
                )));
            }
            if trimmed.chars().count() > max_len {
                return Err(CoreError::Validation(format!(
                    "{field_label} deve ter no máximo {max_len} caracteres."
                )));
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

fn validate_required_text(
    raw: &str,
    field_label: &str,
    max_len: usize,
    allow_multiline: bool,
) -> CoreResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(format!(
            "{field_label} é obrigatório."
        )));
    }
    if has_disallowed_control_chars(trimmed, allow_multiline) {
        return Err(CoreError::Validation(format!(
            "{field_label} contém caracteres de controle inválidos."
        )));
    }
    if trimmed.chars().count() > max_len {
        return Err(CoreError::Validation(format!(
            "{field_label} deve ter no máximo {max_len} caracteres."
        )));
    }
    Ok(trimmed.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Venue {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub street: Option<String>,
    pub number: Option<String>,
    pub complement: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub parking_notes: Option<String>,
    pub loading_dock_notes: Option<String>,
    pub access_hours: Option<String>,
    pub internal_transfer_minutes: Option<i64>,
    pub notes: Option<String>,
    pub active: bool,
    pub search_text: String,
    pub name_sort: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VenueSpace {
    pub id: String,
    pub tenant_id: String,
    pub venue_id: String,
    pub name: String,
    pub name_normalized: String,
    pub capacity_people: Option<i64>,
    pub area_sqm: Option<f64>,
    pub ceiling_height_m: Option<f64>,
    pub power_available: Option<String>,
    pub loading_access: Option<String>,
    pub rigging_points: Option<String>,
    pub acoustics_notes: Option<String>,
    pub setup_hours_notes: Option<String>,
    pub noise_restrictions_notes: Option<String>,
    pub load_dimensions_notes: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VenueContact {
    pub id: String,
    pub tenant_id: String,
    pub venue_id: String,
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

impl fmt::Debug for VenueContact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VenueContact")
            .field("id", &self.id)
            .field("tenant_id", &self.tenant_id)
            .field("venue_id", &self.venue_id)
            .field("name", &self.name)
            .field("email", &self.email.as_deref().map(mask_email_for_audit))
            .field("phone", &self.phone.as_deref().map(mask_phone_for_audit))
            .field("role", &self.role)
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VenueWithDetails {
    #[serde(flatten)]
    pub venue: Venue,
    pub spaces: Vec<VenueSpace>,
    pub contacts: Vec<VenueContact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VenueListItem {
    #[serde(flatten)]
    pub venue: Venue,
    pub spaces_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VenuePage {
    pub items: Vec<VenueListItem>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VenueFilter {
    pub query: Option<String>,
    pub active_only: Option<bool>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueSpaceInput {
    pub id: Option<String>,
    pub name: String,
    pub capacity_people: Option<i64>,
    pub area_sqm: Option<f64>,
    pub ceiling_height_m: Option<f64>,
    pub power_available: Option<String>,
    pub loading_access: Option<String>,
    pub rigging_points: Option<String>,
    pub acoustics_notes: Option<String>,
    pub setup_hours_notes: Option<String>,
    pub noise_restrictions_notes: Option<String>,
    pub load_dimensions_notes: Option<String>,
    pub notes: Option<String>,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VenueContactInput {
    pub id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
}

impl fmt::Debug for VenueContactInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VenueContactInput")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("email", &self.email.as_deref().map(mask_email_for_audit))
            .field("phone", &self.phone.as_deref().map(mask_phone_for_audit))
            .field("role", &self.role)
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVenueInput {
    pub name: String,
    pub street: Option<String>,
    pub number: Option<String>,
    pub complement: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    #[serde(default = "default_country_br")]
    pub country: String,
    pub parking_notes: Option<String>,
    pub loading_dock_notes: Option<String>,
    pub access_hours: Option<String>,
    pub internal_transfer_minutes: Option<i64>,
    pub notes: Option<String>,
    #[serde(default)]
    pub spaces: Vec<VenueSpaceInput>,
    #[serde(default)]
    pub contacts: Vec<VenueContactInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVenueInput {
    pub id: String,
    pub name: String,
    pub street: Option<String>,
    pub number: Option<String>,
    pub complement: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    #[serde(default = "default_country_br")]
    pub country: String,
    pub parking_notes: Option<String>,
    pub loading_dock_notes: Option<String>,
    pub access_hours: Option<String>,
    pub internal_transfer_minutes: Option<i64>,
    pub notes: Option<String>,
    #[serde(default)]
    pub spaces: Vec<VenueSpaceInput>,
    #[serde(default)]
    pub contacts: Vec<VenueContactInput>,
    pub expected_updated_at: String,
}

pub struct ValidatedVenueFields {
    pub name: String,
    pub street: Option<String>,
    pub number: Option<String>,
    pub complement: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub parking_notes: Option<String>,
    pub loading_dock_notes: Option<String>,
    pub access_hours: Option<String>,
    pub internal_transfer_minutes: Option<i64>,
    pub notes: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn validate_venue_fields(
    name: &str,
    street: Option<String>,
    number: Option<String>,
    complement: Option<String>,
    neighborhood: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: &str,
    parking_notes: Option<String>,
    loading_dock_notes: Option<String>,
    access_hours: Option<String>,
    internal_transfer_minutes: Option<i64>,
    notes: Option<String>,
) -> CoreResult<ValidatedVenueFields> {
    let name = validate_required_text(name, "Nome do local", MAX_VENUE_NAME_LEN, false)?;
    let country = validate_country_code(country)?;
    let street = validate_optional_text(street, "Logradouro", 200, false)?;
    let number = validate_optional_text(number, "Número", 40, false)?;
    let complement = validate_optional_text(complement, "Complemento", 120, false)?;
    let neighborhood = validate_optional_text(neighborhood, "Bairro", 120, false)?;
    let city = validate_optional_text(city, "Cidade", 120, false)?;
    let state = match validate_optional_text(state, "UF/Estado", 60, false)? {
        Some(s) if country == "BR" => Some(validate_brazilian_state(&s)?),
        other => other,
    };
    let postal_code = match validate_optional_text(postal_code, "CEP", 20, false)? {
        Some(p) => Some(normalize_and_validate_postal_code(&p, &country)?),
        None => None,
    };
    let parking_notes = validate_optional_text(
        parking_notes,
        "Observações de estacionamento",
        MAX_VENUE_TECH_FIELD_LEN,
        true,
    )?;
    let loading_dock_notes = validate_optional_text(
        loading_dock_notes,
        "Docas / Carga e descarga",
        MAX_VENUE_TECH_FIELD_LEN,
        true,
    )?;
    let access_hours = validate_optional_text(access_hours, "Horários de acesso", 200, false)?;
    if let Some(mins) = internal_transfer_minutes {
        if !(0..=MAX_INTERNAL_TRANSFER_MINUTES).contains(&mins) {
            return Err(CoreError::Validation(format!(
                "Deslocamento interno deve estar entre 0 e {MAX_INTERNAL_TRANSFER_MINUTES} minutos."
            )));
        }
    }
    let notes = validate_optional_text(notes, "Observações", MAX_VENUE_TECH_FIELD_LEN, true)?;

    Ok(ValidatedVenueFields {
        name,
        street,
        number,
        complement,
        neighborhood,
        city,
        state,
        postal_code,
        country,
        parking_notes,
        loading_dock_notes,
        access_hours,
        internal_transfer_minutes,
        notes,
    })
}

pub fn resolve_venue_internal_transfer_minutes(
    venue: &Venue,
    settings: &AllocationSettings,
) -> i64 {
    venue
        .internal_transfer_minutes
        .unwrap_or(settings.internal_transfer_minutes)
}

pub fn validate_venue_spaces(spaces: &mut [VenueSpaceInput]) -> CoreResult<()> {
    if spaces.len() > MAX_VENUE_SPACES {
        return Err(CoreError::Validation(format!(
            "Um local pode ter no máximo {MAX_VENUE_SPACES} espaços."
        )));
    }
    let mut seen_names = HashSet::new();
    for (idx, space) in spaces.iter_mut().enumerate() {
        space.name = validate_required_text(
            &space.name,
            "Nome do espaço",
            MAX_VENUE_SPACE_NAME_LEN,
            false,
        )?;
        let normalized = normalize_for_search(&space.name);
        if !seen_names.insert(normalized) {
            return Err(CoreError::Validation(format!(
                "Já existe um espaço com o nome '{}' neste local.",
                space.name
            )));
        }
        if let Some(cap) = space.capacity_people {
            if !(0..=1_000_000).contains(&cap) {
                return Err(CoreError::Validation(
                    "Capacidade de pessoas deve estar entre 0 e 1.000.000.".to_string(),
                ));
            }
        }
        if let Some(area) = space.area_sqm {
            if !area.is_finite() || area <= 0.0 || area > 1_000_000.0 {
                return Err(CoreError::Validation(
                    "Área (m²) deve ser maior que 0 e no máximo 1.000.000.".to_string(),
                ));
            }
        }
        if let Some(height) = space.ceiling_height_m {
            if !height.is_finite() || height <= 0.0 || height > 100.0 {
                return Err(CoreError::Validation(
                    "Pé-direito (m) deve ser maior que 0 e no máximo 100.".to_string(),
                ));
            }
        }
        space.power_available = validate_optional_text(
            space.power_available.take(),
            "Energia disponível",
            MAX_VENUE_TECH_FIELD_LEN,
            false,
        )?;
        space.loading_access = validate_optional_text(
            space.loading_access.take(),
            "Acesso de carga",
            MAX_VENUE_TECH_FIELD_LEN,
            false,
        )?;
        space.rigging_points = validate_optional_text(
            space.rigging_points.take(),
            "Pontos de rigging",
            MAX_VENUE_TECH_FIELD_LEN,
            false,
        )?;
        space.acoustics_notes = validate_optional_text(
            space.acoustics_notes.take(),
            "Acústica",
            MAX_VENUE_TECH_FIELD_LEN,
            true,
        )?;
        space.setup_hours_notes = validate_optional_text(
            space.setup_hours_notes.take(),
            "Horários de montagem",
            MAX_VENUE_TECH_FIELD_LEN,
            true,
        )?;
        space.noise_restrictions_notes = validate_optional_text(
            space.noise_restrictions_notes.take(),
            "Restrições de ruído",
            MAX_VENUE_TECH_FIELD_LEN,
            true,
        )?;
        space.load_dimensions_notes = validate_optional_text(
            space.load_dimensions_notes.take(),
            "Dimensões de acesso de carga",
            MAX_VENUE_TECH_FIELD_LEN,
            true,
        )?;
        space.notes = validate_optional_text(
            space.notes.take(),
            "Observações do espaço",
            MAX_VENUE_TECH_FIELD_LEN,
            true,
        )?;
        if space.sort_order == 0 {
            space.sort_order = idx as i64;
        }
    }
    Ok(())
}

pub fn validate_venue_contacts(
    contacts: &mut [VenueContactInput],
    venue_country: &str,
) -> CoreResult<()> {
    if contacts.len() > MAX_VENUE_CONTACTS {
        return Err(CoreError::Validation(format!(
            "Um local pode ter no máximo {MAX_VENUE_CONTACTS} contatos."
        )));
    }
    if contacts.is_empty() {
        return Ok(());
    }
    let mut primary_count = 0usize;
    for contact in contacts.iter_mut() {
        contact.name = validate_required_text(&contact.name, "Nome do contato", 120, false)?;
        contact.email =
            match validate_optional_text(contact.email.take(), "E-mail do contato", 254, false)? {
                Some(e) => Some(validate_email(&e)?),
                None => None,
            };
        contact.phone =
            match validate_optional_text(contact.phone.take(), "Telefone do contato", 40, false)? {
                Some(p) => Some(normalize_and_validate_phone(&p, Some(venue_country))?),
                None => None,
            };
        if contact.email.is_none() && contact.phone.is_none() {
            return Err(CoreError::Validation(format!(
                "O contato '{}' deve ter pelo menos um telefone ou e-mail.",
                contact.name
            )));
        }
        contact.role = validate_optional_text(contact.role.take(), "Cargo/Função", 80, false)?;
        if contact.is_primary {
            primary_count += 1;
        }
    }
    if primary_count > 1 {
        return Err(CoreError::Validation(
            "Apenas um contato pode ser marcado como principal.".to_string(),
        ));
    }
    if primary_count == 0 {
        contacts[0].is_primary = true;
    }
    Ok(())
}

pub fn build_venue_search_text(
    fields: &ValidatedVenueFields,
    spaces: &[VenueSpaceInput],
    contacts: &[VenueContactInput],
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(normalize_for_search(&fields.name));
    if let Some(c) = &fields.city {
        parts.push(normalize_for_search(c));
    }
    if let Some(s) = &fields.state {
        parts.push(normalize_for_search(s));
    }
    if let Some(n) = &fields.neighborhood {
        parts.push(normalize_for_search(n));
    }
    for sp in spaces {
        parts.push(normalize_for_search(&sp.name));
    }
    for ct in contacts {
        parts.push(normalize_for_search(&ct.name));
        if let Some(phone) = &ct.phone {
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                parts.push(digits);
            }
        }
    }
    parts.join(" ")
}

pub fn build_venue_name_sort(name: &str) -> String {
    build_name_sort(name)
}
