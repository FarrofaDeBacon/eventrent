use crate::error::{CoreError, Result};
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
    pub const MAX_ICON_BYTES: usize = 64 * 1024;  // 64 KB decodificados
    pub const MAX_COMPANY_NAME_LEN: usize = 80;

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
    pub fn validate_image_data_uri(field_name: &str, data_uri: &str, is_icon: bool) -> Result<String> {
        let trimmed = data_uri.trim();

        // Rejeição estrita a SVG
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("<svg") || lower.contains("image/svg") {
            return Err(CoreError::Validation(
                "Imagens no formato SVG não são permitidas por motivos de segurança. Utilize PNG, JPEG ou WebP."
                    .to_string(),
            ));
        }

        if !trimmed.starts_with("data:image/") {
            return Err(CoreError::Validation(format!(
                "{field_name} deve ser um Data URI iniciando com 'data:image/'."
            )));
        }

        let prefix_end = trimmed.find(";base64,").ok_or_else(|| {
            CoreError::Validation(format!(
                "{field_name} deve ser codificado em base64 no formato 'data:image/<tipo>;base64,<dados>'."
            ))
        })?;

        let mime = &trimmed[5..prefix_end].trim().to_ascii_lowercase();
        let normalized_mime = match mime.as_str() {
            "image/png" => "image/png",
            "image/jpeg" | "image/jpg" => "image/jpeg",
            "image/webp" => "image/webp",
            other => {
                return Err(CoreError::Validation(format!(
                    "Formato de imagem '{other}' não suportado para {field_name}. Utilize PNG, JPEG ou WebP."
                )));
            }
        };

        let payload = &trimmed[prefix_end + 8..];
        let decoded = decode_base64(payload).ok_or_else(|| {
            CoreError::Validation(format!(
                "Dados de base64 inválidos ou corrompidos para {field_name}."
            ))
        })?;

        let max_size = if is_icon {
            Self::MAX_ICON_BYTES
        } else {
            Self::MAX_LOGO_BYTES
        };

        if decoded.len() > max_size {
            return Err(CoreError::Validation(format!(
                "{field_name} excede o limite máximo permitido de {} KB (tamanho decodificado: {} KB).",
                max_size / 1024,
                (decoded.len() + 1023) / 1024
            )));
        }

        // Validação estrita de magic bytes de acordo com o MIME
        match normalized_mime {
            "image/png" => {
                const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
                if decoded.len() < 8 || &decoded[0..8] != &PNG_MAGIC {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo PNG inválida ou corrompida para {field_name}."
                    )));
                }
            }
            "image/jpeg" => {
                const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];
                if decoded.len() < 3 || &decoded[0..3] != &JPEG_MAGIC {
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

        Ok(format!("data:{normalized_mime};base64,{payload}"))
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
            Some(uri) if !uri.trim().is_empty() => {
                Some(Self::validate_image_data_uri("Ícone da empresa", uri, true)?)
            }
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

/// Decodificador Base64 padrão RFC 4648 sem dependências externas.
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut clean = Vec::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        clean.push(b);
    }
    if clean.is_empty() || clean.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks_exact(4) {
        let b0 = decode_b64_char(chunk[0])?;
        let b1 = decode_b64_char(chunk[1])?;
        if chunk[2] == b'=' {
            if chunk[3] != b'=' {
                return None;
            }
            out.push((b0 << 2) | (b1 >> 4));
        } else if chunk[3] == b'=' {
            let b2 = decode_b64_char(chunk[2])?;
            out.push((b0 << 2) | (b1 >> 4));
            out.push(((b1 & 0x0F) << 4) | (b2 >> 2));
        } else {
            let b2 = decode_b64_char(chunk[2])?;
            let b3 = decode_b64_char(chunk[3])?;
            out.push((b0 << 2) | (b1 >> 4));
            out.push(((b1 & 0x0F) << 4) | (b2 >> 2));
            out.push(((b2 & 0x03) << 6) | b3);
        }
    }
    Some(out)
}

fn decode_b64_char(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
