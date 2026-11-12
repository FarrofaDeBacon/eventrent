use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Cpf,
    Cnpj,
    Foreign,
    #[default]
    None,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cpf => "cpf",
            Self::Cnpj => "cnpj",
            Self::Foreign => "foreign",
            Self::None => "none",
        }
    }
}

impl std::str::FromStr for DocumentType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cpf" => Ok(Self::Cpf),
            "cnpj" => Ok(Self::Cnpj),
            "foreign" => Ok(Self::Foreign),
            "none" => Ok(Self::None),
            other => Err(format!("Tipo de documento inválido: '{other}'")),
        }
    }
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpf => write!(f, "CPF"),
            Self::Cnpj => write!(f, "CNPJ"),
            Self::Foreign => write!(f, "Estrangeiro"),
            Self::None => write!(f, "Sem documento"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentNumber {
    pub doc_type: DocumentType,
    pub raw: String,
    pub formatted: String,
}

/// Remove apenas '.', '-', '/' e espaços, convertendo letras ASCII para maiúsculas.
/// Qualquer outro caractere (incluindo acentuados ou símbolos especiais) gera erro de validação.
pub fn normalize_document(input: &str) -> Result<String> {
    let mut clean = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '.' | '-' | '/' | ' ' => continue,
            c if c.is_ascii_alphanumeric() => clean.push(c.to_ascii_uppercase()),
            other => {
                return Err(CoreError::Validation(format!(
                    "Documento contém caractere inválido: '{other}'. Utilize apenas dígitos, letras ASCII [A-Z] e separadores (., -, /)."
                )));
            }
        }
    }
    Ok(clean)
}

/// Valida um CPF (11 dígitos numéricos) aplicando o algoritmo Módulo 11 e rejeitando sequências repetidas.
pub fn validate_cpf(input: &str) -> Result<String> {
    let clean = normalize_document(input)?;

    if clean.len() != 11 {
        return Err(CoreError::Validation(format!(
            "CPF deve conter exatamente 11 dígitos numéricos, mas recebeu {} caracteres.",
            clean.len()
        )));
    }

    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(CoreError::Validation(
            "CPF deve conter apenas dígitos numéricos.".to_string(),
        ));
    }

    // Rejeição de CPFs com todos os dígitos repetidos (00000000000, 11111111111, ...)
    let mut chars_iter = clean.chars();
    let Some(first_char) = chars_iter.next() else {
        return Err(CoreError::Validation(
            "CPF deve conter exatamente 11 dígitos numéricos.".to_string(),
        ));
    };
    if chars_iter.all(|c| c == first_char) {
        return Err(CoreError::Validation(
            "CPF inválido: sequência com todos os dígitos repetidos.".to_string(),
        ));
    }

    let mut digits = Vec::with_capacity(11);
    for c in clean.chars() {
        let Some(d) = c.to_digit(10) else {
            return Err(CoreError::Validation(
                "CPF deve conter apenas dígitos numéricos.".to_string(),
            ));
        };
        digits.push(d);
    }

    // Primeiro dígito verificador (pesos de 10 a 2)
    let sum1: u32 = (0..9).map(|i| digits[i] * (10 - i as u32)).sum();
    let remainder1 = sum1 % 11;
    let expected_dv1 = if remainder1 < 2 { 0 } else { 11 - remainder1 };

    if digits[9] != expected_dv1 {
        return Err(CoreError::Validation(format!(
            "CPF inválido: primeiro dígito verificador incorreto (esperado {expected_dv1}, recebido {}).",
            digits[9]
        )));
    }

    // Segundo dígito verificador (pesos de 11 a 2)
    let sum2: u32 = (0..10).map(|i| digits[i] * (11 - i as u32)).sum();
    let remainder2 = sum2 % 11;
    let expected_dv2 = if remainder2 < 2 { 0 } else { 11 - remainder2 };

    if digits[10] != expected_dv2 {
        return Err(CoreError::Validation(format!(
            "CPF inválido: segundo dígito verificador incorreto (esperado {expected_dv2}, recebido {}).",
            digits[10]
        )));
    }

    Ok(clean)
}

/// Valida um CNPJ (14 caracteres) suportando tanto o formato puramente numérico tradicional
/// quanto o CNPJ ALFANUMÉRICO regulamentado pela Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024.
///
/// Regras (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024):
/// - As primeiras 12 posições são alfanuméricas [0-9A-Z] (raiz e filial).
/// - As 2 últimas posições são estritamente dígitos verificadores numéricos [0-9].
/// - O valor de cada caractere é o código ASCII - 48 ('0'-'9' = 0-9; 'A'-'Z' = 17-42).
/// - Pesos DV1: 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2 sobre os 12 primeiros caracteres.
/// - Pesos DV2: 6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2 sobre os 12 primeiros caracteres + DV1.
/// - Resto < 2 -> DV = 0; senão DV = 11 - Resto.
pub fn validate_cnpj(input: &str) -> Result<String> {
    let clean = normalize_document(input)?;

    if clean.len() != 14 {
        return Err(CoreError::Validation(format!(
            "CNPJ deve conter exatamente 14 caracteres alfanuméricos, mas recebeu {} caracteres.",
            clean.len()
        )));
    }

    // Todos os caracteres devem ser ASCII alfanuméricos maiúsculos
    if !clean.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CoreError::Validation(
            "CNPJ contém caracteres inválidos. Apenas dígitos e letras de A a Z são aceitos."
                .to_string(),
        ));
    }

    // As duas últimas posições (dígitos verificadores) devem ser estritamente numéricas
    let chars: Vec<char> = clean.chars().collect();
    if !chars[12].is_ascii_digit() || !chars[13].is_ascii_digit() {
        return Err(CoreError::Validation(
            "Os dois últimos caracteres do CNPJ (dígitos verificadores) devem ser numéricos."
                .to_string(),
        ));
    }

    // Rejeição de CNPJs com todos os caracteres idênticos (ex: 00000000000000)
    let first_char = chars[0];
    if chars.iter().all(|&c| c == first_char) {
        return Err(CoreError::Validation(
            "CNPJ inválido: sequência com todos os caracteres idênticos.".to_string(),
        ));
    }

    // Cálculo do valor do caractere: ASCII - 48
    let char_value = |c: char| -> u32 { (c as u32) - 48 };

    // Pesos Módulo 11 para CNPJ
    let weights1: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum1: u32 = (0..12).map(|i| char_value(chars[i]) * weights1[i]).sum();
    let remainder1 = sum1 % 11;
    let expected_dv1 = if remainder1 < 2 { 0 } else { 11 - remainder1 };

    let Some(dv1) = chars[12].to_digit(10) else {
        return Err(CoreError::Validation(
            "Os dois últimos caracteres do CNPJ (dígitos verificadores) devem ser numéricos."
                .to_string(),
        ));
    };
    if dv1 != expected_dv1 {
        return Err(CoreError::Validation(format!(
            "CNPJ inválido: primeiro dígito verificador incorreto (esperado {expected_dv1}, recebido {dv1})."
        )));
    }

    let weights2: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum2: u32 = (0..12)
        .map(|i| char_value(chars[i]) * weights2[i])
        .sum::<u32>()
        + (expected_dv1 * weights2[12]);
    let remainder2 = sum2 % 11;
    let expected_dv2 = if remainder2 < 2 { 0 } else { 11 - remainder2 };

    let Some(dv2) = chars[13].to_digit(10) else {
        return Err(CoreError::Validation(
            "Os dois últimos caracteres do CNPJ (dígitos verificadores) devem ser numéricos."
                .to_string(),
        ));
    };
    if dv2 != expected_dv2 {
        return Err(CoreError::Validation(format!(
            "CNPJ inválido: segundo dígito verificador incorreto (esperado {expected_dv2}, recebido {dv2})."
        )));
    }

    Ok(clean)
}

/// Valida um documento estrangeiro: texto normalizado em maiúsculas, 5 a 30 caracteres alfanuméricos, sem dígito verificador.
pub fn validate_foreign_document(input: &str) -> Result<String> {
    let clean = normalize_document(input)?;
    if !(5..=30).contains(&clean.len()) {
        return Err(CoreError::Validation(format!(
            "Documento estrangeiro deve conter entre 5 e 30 caracteres alfanuméricos, mas recebeu {} caracteres.",
            clean.len()
        )));
    }
    Ok(clean)
}

/// Valida o documento conforme o tipo informado (`cpf`, `cnpj`, `foreign` ou `none`),
/// retornando `(document_type, raw_nullable, formatted_for_search)`.
pub fn parse_document_with_type(
    doc_type: DocumentType,
    input: Option<&str>,
) -> Result<(DocumentType, Option<String>, String)> {
    let trimmed = input.map(str::trim).filter(|s| !s.is_empty());
    match doc_type {
        DocumentType::None => {
            if trimmed.is_some() {
                return Err(CoreError::Validation(
                    "Quando o tipo de documento é 'none', o número do documento deve ficar vazio."
                        .to_string(),
                ));
            }
            Ok((DocumentType::None, None, String::new()))
        }
        DocumentType::Cpf => {
            let raw_input = trimmed
                .ok_or_else(|| CoreError::Validation("Número de CPF é obrigatório.".to_string()))?;
            let valid = validate_cpf(raw_input)?;
            let formatted = format_cpf(&valid);
            Ok((DocumentType::Cpf, Some(valid), formatted))
        }
        DocumentType::Cnpj => {
            let raw_input = trimmed.ok_or_else(|| {
                CoreError::Validation("Número de CNPJ é obrigatório.".to_string())
            })?;
            let valid = validate_cnpj(raw_input)?;
            let formatted = format_cnpj(&valid);
            Ok((DocumentType::Cnpj, Some(valid), formatted))
        }
        DocumentType::Foreign => {
            let raw_input = trimmed.ok_or_else(|| {
                CoreError::Validation("Número do documento estrangeiro é obrigatório.".to_string())
            })?;
            let valid = validate_foreign_document(raw_input)?;
            Ok((DocumentType::Foreign, Some(valid.clone()), valid))
        }
    }
}

/// Identifica automaticamente o tipo de documento (CPF ou CNPJ), valida suas regras e formata.
pub fn parse_document(input: &str) -> Result<DocumentNumber> {
    let clean = normalize_document(input)?;

    match clean.len() {
        11 => {
            let valid = validate_cpf(&clean)?;
            let formatted = format_cpf(&valid);
            Ok(DocumentNumber {
                doc_type: DocumentType::Cpf,
                raw: valid,
                formatted,
            })
        }
        14 => {
            let valid = validate_cnpj(&clean)?;
            let formatted = format_cnpj(&valid);
            Ok(DocumentNumber {
                doc_type: DocumentType::Cnpj,
                raw: valid,
                formatted,
            })
        }
        other => Err(CoreError::Validation(format!(
            "Documento brasileiro deve conter 11 dígitos (CPF) ou 14 caracteres (CNPJ), mas recebeu {other} caracteres."
        ))),
    }
}

/// Formata uma string limpa de CPF (11 dígitos) no padrão `000.000.000-00`.
pub fn format_cpf(clean: &str) -> String {
    if clean.len() != 11 {
        return clean.to_string();
    }
    format!(
        "{}.{}.{}-{}",
        &clean[0..3],
        &clean[3..6],
        &clean[6..9],
        &clean[9..11]
    )
}

/// Formata uma string limpa de CNPJ (14 caracteres) no padrão `XX.XXX.XXX/XXXX-XX`.
pub fn format_cnpj(clean: &str) -> String {
    if clean.len() != 14 {
        return clean.to_string();
    }
    format!(
        "{}.{}.{}/{}-{}",
        &clean[0..2],
        &clean[2..5],
        &clean[5..8],
        &clean[8..12],
        &clean[12..14]
    )
}

/// Mascara o número do documento para conformidade com a LGPD em logs de auditoria e telemetria:
/// - CPF: `***.***.***-25` (preserva apenas os dígitos verificadores).
/// - CNPJ: `**.***.***/0001-**` (preserva apenas o número de ordem/filial).
pub fn mask_document_for_audit(document: &str) -> String {
    let clean = normalize_document(document).unwrap_or_else(|_| {
        document
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_uppercase()
    });
    if clean.len() == 11 {
        format!("***.***.***-{}", &clean[9..11])
    } else if clean.len() == 14 {
        format!("**.***.***/{}-**", &clean[8..12])
    } else {
        "***".to_string()
    }
}

/// Utilitários de geração para suítes de teste automatizadas.
#[cfg(any(test, feature = "test-utils"))]
pub mod testing_utils {
    /// Gera um CPF válido arbitrário a partir dos 9 primeiros dígitos fornecidos (ou base fixa).
    pub fn generate_valid_cpf_from_base(base9: &[u32; 9]) -> String {
        let sum1: u32 = (0..9).map(|i| base9[i] * (10 - i as u32)).sum();
        let rem1 = sum1 % 11;
        let dv1 = if rem1 < 2 { 0 } else { 11 - rem1 };

        let sum2: u32 = (0..9).map(|i| base9[i] * (11 - i as u32)).sum::<u32>() + dv1 * 2;
        let rem2 = sum2 % 11;
        let dv2 = if rem2 < 2 { 0 } else { 11 - rem2 };

        let mut res = base9.iter().map(|d| d.to_string()).collect::<String>();
        res.push_str(&dv1.to_string());
        res.push_str(&dv2.to_string());
        res
    }

    /// Gera um CNPJ válido arbitrário (numérico ou alfanumérico) a partir dos 12 caracteres iniciais.
    pub fn generate_valid_cnpj_from_base(base12: &str) -> String {
        assert_eq!(base12.len(), 12);
        let chars: Vec<char> = base12.to_ascii_uppercase().chars().collect();
        let char_value = |c: char| -> u32 { (c as u32) - 48 };

        let weights1: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
        let sum1: u32 = (0..12).map(|i| char_value(chars[i]) * weights1[i]).sum();
        let rem1 = sum1 % 11;
        let dv1 = if rem1 < 2 { 0 } else { 11 - rem1 };

        let weights2: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
        let sum2: u32 = (0..12)
            .map(|i| char_value(chars[i]) * weights2[i])
            .sum::<u32>()
            + (dv1 * weights2[12]);
        let rem2 = sum2 % 11;
        let dv2 = if rem2 < 2 { 0 } else { 11 - rem2 };

        format!("{base12}{dv1}{dv2}")
    }
}
