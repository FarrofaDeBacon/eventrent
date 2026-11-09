use crate::error::{CoreError, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

pub struct BrandingValidator;

impl BrandingValidator {
    pub const MAX_LOGO_BYTES: usize = 256 * 1024; // 256 KB decodificados
    pub const MAX_ICON_BYTES: usize = 64 * 1024; // 64 KB decodificados
    pub const MAX_COMPANY_NAME_LEN: usize = 80;

    const PREFIX_PNG: &'static str = "data:image/png;base64,";
    const PREFIX_JPEG: &'static str = "data:image/jpeg;base64,";
    const PREFIX_WEBP: &'static str = "data:image/webp;base64,";

    /// Valida o nome da empresa: entre 1 e 80 caracteres, sem caracteres de controle.
    pub fn validate_company_name(name: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Validation(
                "Nome da empresa não pode ser vazio.".to_string(),
            ));
        }
        if trimmed.chars().count() > Self::MAX_COMPANY_NAME_LEN {
            return Err(CoreError::Validation(format!(
                "Nome da empresa excede o limite máximo de {} caracteres.",
                Self::MAX_COMPANY_NAME_LEN
            )));
        }
        if trimmed.chars().any(|c| c.is_control()) {
            return Err(CoreError::Validation(
                "Nome da empresa não pode conter caracteres de controle.".to_string(),
            ));
        }
        Ok(trimmed.to_string())
    }

    /// Valida uma cor hexadecimal no formato estrito #RRGGBB, normalizando para minúsculas.
    /// Rejeita qualquer tentativa de injeção de CSS (ex.: semicolons, nomes, rgb(), etc.).
    pub fn validate_hex_color(field_name: &str, color: &str) -> Result<String> {
        let trimmed = color.trim();
        if !trimmed.starts_with('#') || trimmed.len() != 7 {
            return Err(CoreError::Validation(format!(
                "Cor {field_name} inválida '{trimmed}'. Use o formato hexadecimal #RRGGBB."
            )));
        }
        if !trimmed[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CoreError::Validation(format!(
                "Cor {field_name} contém caracteres não-hexadecimais '{trimmed}'. Use o formato hexadecimal #RRGGBB."
            )));
        }
        Ok(trimmed.to_ascii_lowercase())
    }

    /// Valida um Data URI de imagem (PNG, JPEG ou WebP) verificando limite de tamanho e magic bytes.
    /// Rejeita estritamente imagens SVG por motivos de segurança (sanitização complexa e PDF).
    pub fn validate_image_data_uri(
        field_name: &str,
        data_uri: &str,
        is_icon: bool,
    ) -> Result<String> {
        // Rejeição estrita a qualquer presença de SVG no Data URI
        let lower = data_uri.to_ascii_lowercase();
        if lower.contains("<svg") || lower.contains("image/svg") {
            return Err(CoreError::Validation(
                "Imagens no formato SVG não são permitidas por motivos de segurança. Utilize PNG, JPEG ou WebP."
                    .to_string(),
            ));
        }

        // Validação estrita do prefixo (sem parâmetros adicionais, sem espaços antes/depois)
        let (mime, payload) = if let Some(rest) = data_uri.strip_prefix(Self::PREFIX_PNG) {
            ("image/png", rest)
        } else if let Some(rest) = data_uri.strip_prefix(Self::PREFIX_JPEG) {
            ("image/jpeg", rest)
        } else if let Some(rest) = data_uri.strip_prefix(Self::PREFIX_WEBP) {
            ("image/webp", rest)
        } else {
            return Err(CoreError::Validation(format!(
                "{field_name} deve conter prefixo exato 'data:image/png;base64,', 'data:image/jpeg;base64,' ou 'data:image/webp;base64,' sem parâmetros ou espaços."
            )));
        };

        let max_decoded_bytes = if is_icon {
            Self::MAX_ICON_BYTES
        } else {
            Self::MAX_LOGO_BYTES
        };

        // Rejeição pré-decodificação pelo tamanho codificado
        // Em Base64 com padding, cada bloco de 3 bytes resulta em 4 caracteres
        let max_encoded_len = max_decoded_bytes.div_ceil(3) * 4;
        if payload.len() > max_encoded_len {
            return Err(CoreError::Validation(format!(
                "{field_name} excede o limite máximo permitido de {} KB antes da decodificação.",
                max_decoded_bytes / 1024
            )));
        }

        // Decodificação estrita RFC 4648 sem tolerância a espaços ou quebras de linha
        let decoded = BASE64_STANDARD.decode(payload.as_bytes()).map_err(|_| {
            CoreError::Validation(format!(
                "Dados de base64 inválidos, com padding incorreto ou caracteres ilegais para {field_name}."
            ))
        })?;

        // Verificação final do limite sobre o tamanho decodificado
        if decoded.len() > max_decoded_bytes {
            return Err(CoreError::Validation(format!(
                "{field_name} excede o limite máximo permitido de {} KB (tamanho decodificado: {} KB).",
                max_decoded_bytes / 1024,
                decoded.len().div_ceil(1024)
            )));
        }

        // Validação estrita de magic bytes de acordo com o MIME declarado
        match mime {
            "image/png" => {
                const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
                if decoded.len() < 8 || decoded[0..8] != PNG_MAGIC {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo PNG inválida ou corrompida para {field_name}."
                    )));
                }
            }
            "image/jpeg" => {
                const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];
                if decoded.len() < 3 || decoded[0..3] != JPEG_MAGIC {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo JPEG inválida ou corrompida para {field_name}."
                    )));
                }
            }
            "image/webp" => {
                if decoded.len() < 12 || &decoded[0..4] != b"RIFF" || &decoded[8..12] != b"WEBP" {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo WebP inválida ou corrompida para {field_name}."
                    )));
                }
            }
            _ => unreachable!(),
        }

        // Rejeição estrita caso o payload decodificado contenha marcação SVG
        if decoded.windows(4).any(|w| w.eq_ignore_ascii_case(b"<svg")) {
            return Err(CoreError::Validation(
                "Imagens no formato SVG não são permitidas por motivos de segurança. Utilize PNG, JPEG ou WebP."
                    .to_string(),
            ));
        }

        Ok(data_uri.to_string())
    }

    /// Valida completamente a entrada de atualização de identidade visual.
    pub fn validate_input(input: &UpdateBrandingInput) -> Result<UpdateBrandingInput> {
        let company_name = Self::validate_company_name(&input.company_name)?;
        let primary_color = Self::validate_hex_color("primária", &input.primary_color)?;
        let secondary_color = Self::validate_hex_color("secundária", &input.secondary_color)?;
        let accent_color = Self::validate_hex_color("de destaque", &input.accent_color)?;

        let logo_url = match &input.logo_url {
            Some(uri) if !uri.trim().is_empty() => {
                Some(Self::validate_image_data_uri("Logo claro", uri, false)?)
            }
            _ => None,
        };

        let logo_dark_url = match &input.logo_dark_url {
            Some(uri) if !uri.trim().is_empty() => {
                Some(Self::validate_image_data_uri("Logo escuro", uri, false)?)
            }
            _ => None,
        };

        let icon_url = match &input.icon_url {
            Some(uri) if !uri.trim().is_empty() => Some(Self::validate_image_data_uri(
                "Ícone da empresa",
                uri,
                true,
            )?),
            _ => None,
        };

        Ok(UpdateBrandingInput {
            company_name,
            primary_color,
            secondary_color,
            accent_color,
            logo_url,
            logo_dark_url,
            icon_url,
        })
    }
}
