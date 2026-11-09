use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantBranding {
    pub id: String,
    pub tenant_id: String,
    pub company_name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: String,
    pub logo_url: Option<String>,
    pub logo_dark_url: Option<String>,
    pub icon_url: Option<String>,
    pub favicon_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBrandingInput {
    pub company_name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: String,
    pub logo_url: Option<String>,
    pub logo_dark_url: Option<String>,
    pub icon_url: Option<String>,
}

/// Dados públicos de identidade visual (não sensíveis).
/// Utilizados antes do login (telas de /login e /setup) para manter o white-label
/// sem requerer sessão ou permissões de usuário.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicBranding {
    pub company_name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: String,
    pub logo_url: Option<String>,
    pub logo_dark_url: Option<String>,
    pub icon_url: Option<String>,
}

impl From<TenantBranding> for PublicBranding {
    fn from(b: TenantBranding) -> Self {
        Self {
            company_name: b.company_name,
            primary_color: b.primary_color,
            secondary_color: b.secondary_color,
            accent_color: b.accent_color,
            logo_url: b.logo_url,
            logo_dark_url: b.logo_dark_url,
            icon_url: b.icon_url,
        }
    }
}
