use crate::AppState;
use chrono::{DateTime, Duration, Utc};
use eventrent_core::{
    SessionInfo, PERM_BRANDING_MANAGE, PERM_BRANDING_READ, PERM_ROLE_READ, PERM_USER_MANAGE,
    PERM_USER_READ,
};

/// Definição formal de comando registrado no runtime Tauri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDefinition {
    pub name: &'static str,
    pub is_public: bool,
    pub required_permission: Option<&'static str>,
}

/// Registro centralizado e canônico de todos os comandos invocáveis do Tauri.
/// Usado pela suíte de auditoria para garantir que nenhum comando privado deixe de passar pela guarda central.
pub const COMMAND_REGISTRY: &[CommandDefinition] = &[
    // Comandos públicos (pré-autenticação / ciclo de vida da sessão)
    CommandDefinition {
        name: "get_app_status",
        is_public: true,
        required_permission: None,
    },
    CommandDefinition {
        name: "complete_initial_setup",
        is_public: true,
        required_permission: None,
    },
    CommandDefinition {
        name: "login",
        is_public: true,
        required_permission: None,
    },
    CommandDefinition {
        name: "logout",
        is_public: true,
        required_permission: None,
    },
    CommandDefinition {
        name: "get_current_session",
        is_public: true,
        required_permission: None,
    },
    CommandDefinition {
        name: "recover_password",
        is_public: true,
        required_permission: None,
    },
    // Comandos protegidos por autenticação e RBAC
    CommandDefinition {
        name: "get_branding",
        is_public: false,
        required_permission: Some(PERM_BRANDING_READ),
    },
    CommandDefinition {
        name: "save_branding",
        is_public: false,
        required_permission: Some(PERM_BRANDING_MANAGE),
    },
    CommandDefinition {
        name: "list_users",
        is_public: false,
        required_permission: Some(PERM_USER_READ),
    },
    CommandDefinition {
        name: "create_user",
        is_public: false,
        required_permission: Some(PERM_USER_MANAGE),
    },
    CommandDefinition {
        name: "update_user",
        is_public: false,
        required_permission: Some(PERM_USER_MANAGE),
    },
    CommandDefinition {
        name: "set_user_active",
        is_public: false,
        required_permission: Some(PERM_USER_MANAGE),
    },
    CommandDefinition {
        name: "reset_user_password",
        is_public: false,
        required_permission: Some(PERM_USER_MANAGE),
    },
    CommandDefinition {
        name: "list_roles",
        is_public: false,
        required_permission: Some(PERM_ROLE_READ),
    },
];

/// Valida se a sessão expirou por inatividade utilizando um relógio injetável (sem sleep).
pub fn evaluate_session_inactivity(
    last_activity_at: &str,
    inactivity_timeout_minutes: u32,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let last_activity = DateTime::parse_from_rfc3339(last_activity_at)
        .map_err(|e| format!("Timestamp de sessão inválido: {e}"))?
        .with_timezone(&Utc);

    let timeout = Duration::minutes(i64::from(inactivity_timeout_minutes.max(1)));
    if now - last_activity > timeout {
        return Err("Sessão expirada por inatividade.".to_string());
    }
    Ok(())
}

/// Guarda central única de autenticação e autorização RBAC para comandos Tauri.
///
/// Todos os comandos não públicos obrigatoriamente passam por esta guarda.
/// Ela garante:
/// 1. Existência de sessão ativa;
/// 2. Validação de expiração por inatividade;
/// 3. Atualização automática do timestamp de última atividade (touch);
/// 4. Verificação estrita da permissão exigida via RBAC.
pub fn require_session_and_permission(
    state: &AppState,
    required_permission: Option<&str>,
) -> Result<SessionInfo, String> {
    let session = state.require_session()?;

    if let Some(perm) = required_permission {
        if !session.has_permission(perm) {
            return Err(format!("Permissão insuficiente: requer '{perm}'."));
        }
    }

    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_non_public_commands_have_required_permissions() {
        for cmd in COMMAND_REGISTRY {
            if !cmd.is_public {
                assert!(
                    cmd.required_permission.is_some(),
                    "Comando protegido '{}' deve definir uma permissão obrigatória",
                    cmd.name
                );
            }
        }
    }

    #[test]
    fn test_public_commands_are_strictly_whitelisted() {
        let expected_public = [
            "get_app_status",
            "complete_initial_setup",
            "login",
            "logout",
            "get_current_session",
            "recover_password",
        ];

        for cmd in COMMAND_REGISTRY {
            if cmd.is_public {
                assert!(
                    expected_public.contains(&cmd.name),
                    "Comando público inesperado encontrado no registro: {}",
                    cmd.name
                );
            }
        }
    }

    #[test]
    fn test_session_inactivity_expiration_with_injectable_clock() {
        let base_time = Utc::now();
        let timeout_minutes = 30;
        let last_activity_at = base_time.to_rfc3339();

        // 1. Acesso 15 minutos depois: dentro do limite (OK)
        let active_check = evaluate_session_inactivity(
            &last_activity_at,
            timeout_minutes,
            base_time + Duration::minutes(15),
        );
        assert!(active_check.is_ok());

        // 2. Acesso 30 minutos depois: limite exato (OK)
        let boundary_check = evaluate_session_inactivity(
            &last_activity_at,
            timeout_minutes,
            base_time + Duration::minutes(30),
        );
        assert!(boundary_check.is_ok());

        // 3. Acesso 31 minutos depois: expirou sem nenhum sleep real (ERRO)
        let expired_check = evaluate_session_inactivity(
            &last_activity_at,
            timeout_minutes,
            base_time + Duration::minutes(31),
        );
        assert!(expired_check.is_err());
        assert!(expired_check.unwrap_err().contains("expirada por inatividade"));
    }

    #[test]
    fn test_command_source_files_use_central_guard() {
        // Auditoria estática: verifica que todos os arquivos de comando protegidos
        // invocam `require_session_and_permission`.
        let commands_dir = std::path::Path::new("src/commands");
        if !commands_dir.exists() {
            // Se executado da raiz do workspace
            return;
        }

        let protected_files = ["users.rs", "branding.rs"];
        for file in protected_files {
            let path = commands_dir.join(file);
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Falha ao ler arquivo de comando {}", path.display()));
            assert!(
                content.contains("require_session_and_permission"),
                "Arquivo {} deve utilizar a guarda central require_session_and_permission",
                path.display()
            );
        }
    }
}
