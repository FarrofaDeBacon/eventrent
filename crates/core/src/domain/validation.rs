use crate::error::{CoreError, Result as CoreResult};

pub const BRAZILIAN_STATES: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

/// Valida e normaliza endereço de e-mail em um subconjunto ASCII estrito.
///
/// Regras aplicadas:
/// 1. Sem espaços em branco ou caracteres de controle.
/// 2. Comprimento total entre 5 e 254 caracteres.
/// 3. Presença de exatamente um caractere `@`, separando parte local e domínio.
/// 4. Parte local: 1 a 64 caracteres ASCII (`a-z`, `0-9` e `. _ % + -`), sem pontos no início/fim e sem pontos consecutivos; não aceita aspas nem comentários.
/// 5. Domínio: pelo menos dois rótulos separados por `.`, cada rótulo com apenas `a-z`, `0-9` e hífen `-` (sem hífen no início/fim).
/// 6. TLD: no mínimo 2 caracteres (`a-z`, `0-9` ou `-`, sem hífen no início/fim).
pub fn validate_email(email: &str) -> CoreResult<String> {
    if !email.is_ascii() {
        return Err(CoreError::Validation(
            "O e-mail deve conter apenas caracteres ASCII.".to_string(),
        ));
    }

    if email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(CoreError::Validation(
            "O e-mail não pode conter espaços ou caracteres de controle.".to_string(),
        ));
    }

    let lowered = email.to_lowercase();
    let total_chars = lowered.chars().count();
    if !(5..=254).contains(&total_chars) {
        return Err(CoreError::Validation(
            "O e-mail deve ter entre 5 e 254 caracteres.".to_string(),
        ));
    }

    let parts: Vec<&str> = lowered.split('@').collect();
    if parts.len() != 2 {
        return Err(CoreError::Validation(
            "Informe um e-mail válido contendo um único '@'.".to_string(),
        ));
    }

    let (local, domain) = (parts[0], parts[1]);
    let local_chars = local.chars().count();
    if local.is_empty()
        || local_chars > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
    {
        return Err(CoreError::Validation(
            "Parte local do e-mail inválida (máximo 64 caracteres, sem pontos nas extremidades ou consecutivos).".to_string(),
        ));
    }

    // Subconjunto ASCII restrito: parte local aceita apenas a-z, 0-9 e . _ % + - (sem aspas, comentários ou espaços)
    if !local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-'))
    {
        return Err(CoreError::Validation(
            "Parte local do e-mail contém caracteres inválidos (permitidos apenas letras ASCII, dígitos e . _ % + -).".to_string(),
        ));
    }

    if domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.starts_with('-')
        || domain.ends_with('-')
        || domain.contains("..")
    {
        return Err(CoreError::Validation(
            "Domínio do e-mail inválido.".to_string(),
        ));
    }

    let domain_labels: Vec<&str> = domain.split('.').collect();
    if domain_labels.len() < 2 {
        return Err(CoreError::Validation(
            "Domínio do e-mail deve conter pelo menos uma extensão (ex: .com).".to_string(),
        ));
    }

    // Cada rótulo do domínio aceita apenas a-z, 0-9 e hífen (sem hífen nas pontas)
    for label in &domain_labels {
        if label.is_empty()
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(CoreError::Validation(
                "Rótulo de domínio do e-mail inválido (use apenas letras ASCII, dígitos ou hífen sem hífen nas extremidades).".to_string(),
            ));
        }
    }

    let Some(tld) = domain_labels.last() else {
        return Err(CoreError::Validation(
            "Extensão do e-mail inválida.".to_string(),
        ));
    };

    if tld.len() < 2
        || tld.starts_with('-')
        || tld.ends_with('-')
        || !tld.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(CoreError::Validation(
            "Extensão do e-mail inválida.".to_string(),
        ));
    }

    Ok(lowered)
}

pub fn validate_brazilian_state(state: &str) -> CoreResult<String> {
    let upper = state.trim().to_uppercase();
    if BRAZILIAN_STATES.contains(&upper.as_str()) {
        Ok(upper)
    } else {
        Err(CoreError::Validation(format!(
            "UF inválida: '{state}'. Informe uma das 27 unidades federativas brasileiras (ex: SP, RJ, MG)."
        )))
    }
}

pub fn validate_country_code(country: &str) -> CoreResult<String> {
    let trimmed = country.trim().to_uppercase();
    if trimmed.len() == 2 && trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        Ok(trimmed)
    } else {
        Err(CoreError::Validation(
            "Código do país inválido: deve conter 2 letras ISO (ex: BR).".to_string(),
        ))
    }
}

pub fn normalize_and_validate_postal_code(postal_code: &str, country: &str) -> CoreResult<String> {
    let trimmed = postal_code.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "CEP/Código postal é obrigatório.".to_string(),
        ));
    }
    if country.eq_ignore_ascii_case("BR") {
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == ' ')
        {
            return Err(CoreError::Validation(
                "CEP inválido: deve conter apenas dígitos e separadores usuais (traço ou ponto)."
                    .to_string(),
            ));
        }
        let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 8 {
            return Err(CoreError::Validation(
                "CEP inválido: deve conter exatamente 8 dígitos numéricos.".to_string(),
            ));
        }
        Ok(digits)
    } else {
        if trimmed.chars().count() > 20 {
            return Err(CoreError::Validation(
                "Código postal excede o limite máximo de 20 caracteres.".to_string(),
            ));
        }
        Ok(trimmed.to_string())
    }
}

pub fn normalize_and_validate_phone(phone: &str, country: Option<&str>) -> CoreResult<String> {
    let trimmed = phone.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "Telefone não pode ser vazio.".to_string(),
        ));
    }

    if !trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '(' | ')' | '-' | '.' | ' '))
    {
        return Err(CoreError::Validation(
            "Telefone contém caracteres inválidos. Utilize apenas números, espaços e os símbolos +, -, (, ), .".to_string(),
        ));
    }

    // '+' só é permitido como primeiro caractere
    if trimmed.chars().skip(1).any(|c| c == '+') {
        return Err(CoreError::Validation(
            "Telefone inválido: o símbolo '+' só pode aparecer no início do número.".to_string(),
        ));
    }

    let starts_with_plus = trimmed.starts_with('+');
    let mut digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    if starts_with_plus {
        if digits.starts_with("55") {
            digits = digits[2..].to_string();
            if digits.len() == 10 || digits.len() == 11 {
                return Ok(format!("+55{digits}"));
            }
            return Err(CoreError::Validation(
                "Telefone inválido: para o Brasil (+55) deve conter 10 ou 11 dígitos com DDD (ex: 11987654321).".to_string(),
            ));
        }

        if (8..=15).contains(&digits.len()) {
            return Ok(format!("+{digits}"));
        }
        return Err(CoreError::Validation(
            "Telefone internacional inválido: deve conter entre 8 e 15 dígitos.".to_string(),
        ));
    }

    let is_br = country
        .map(|c| c.eq_ignore_ascii_case("BR"))
        .unwrap_or(true);

    if is_br {
        if digits.starts_with("55") && (digits.len() == 12 || digits.len() == 13) {
            digits = digits[2..].to_string();
        }
        if digits.len() == 10 || digits.len() == 11 {
            Ok(format!("+55{digits}"))
        } else {
            Err(CoreError::Validation(
                "Telefone inválido: para o Brasil deve conter 10 ou 11 dígitos com DDD (ex: 11987654321).".to_string(),
            ))
        }
    } else if (8..=15).contains(&digits.len()) {
        Ok(format!("+{digits}"))
    } else {
        Err(CoreError::Validation(
            "Telefone inválido: para outros países deve conter entre 8 e 15 dígitos.".to_string(),
        ))
    }
}

/// Valida e normaliza o nome de usuário / proprietário (1 a 100 caracteres, sem caracteres de controle, já aparado).
pub fn validate_user_name(name: &str) -> CoreResult<String> {
    let trimmed = name.trim();
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(
            "O nome do usuário não pode conter caracteres de controle ou quebras de linha."
                .to_string(),
        ));
    }
    let char_count = trimmed.chars().count();
    if !(1..=100).contains(&char_count) {
        return Err(CoreError::Validation(
            "O nome do usuário deve conter entre 1 e 100 caracteres.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Verifica se um caractere é de controle ou caractere invisível/formato/direção perigoso.
/// Bloqueia:
/// - Caracteres de controle (`c.is_control()`)
/// - Espaços invisíveis e de formatação: U+200B a U+200F (zero-width space/non-joiner/joiner/LRM/RLM)
/// - Controles de direção de texto bidi: U+202A a U+202E (LRE, RLE, PDF, LRO, RLO)
/// - Controles de isolamento bidi: U+2066 a U+2069 (LRI, RLI, FSI, PDI)
/// - Separadores de linha/parágrafo Unicode: U+2028 (LINE SEPARATOR), U+2029 (PARAGRAPH SEPARATOR)
/// - Byte Order Mark / zero-width no-break space: U+FEFF
pub fn is_dangerous_or_invisible_control_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{FEFF}'
        )
}

/// Valida se um texto não contém caracteres de controle nem caracteres invisíveis ou de direção bidi.
pub fn ensure_no_control_or_invisible_chars(field_name: &str, text: &str) -> CoreResult<()> {
    if text.chars().any(is_dangerous_or_invisible_control_char) {
        return Err(CoreError::Validation(format!(
            "O campo '{field_name}' contém caracteres de controle, invisíveis ou de direção de texto inválidos."
        )));
    }
    Ok(())
}
