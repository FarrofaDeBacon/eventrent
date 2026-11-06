use crate::AppState;
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
/// Usado pela suíte de auditoria para garantir sincronia estrita com generate_handler!.
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
        name: "get_public_branding",
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
    // Comandos protegidos por autenticação e RBAC (o core é a autoridade máxima)
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
    use chrono::{Duration, Utc};
    use eventrent_core::CoreServices;

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
            "get_public_branding",
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
    fn test_session_inactivity_expiration_uses_core_production_path() {
        let base_time = Utc::now();
        let timeout_minutes = 30;
        let session = SessionInfo {
            user_id: "test-user".to_string(),
            tenant_id: "test-tenant".to_string(),
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            role_id: "role-1".to_string(),
            role_name: "Admin".to_string(),
            role_slug: "admin".to_string(),
            permissions: vec![],
            authenticated_at: base_time.to_rfc3339(),
            last_activity_at: base_time.to_rfc3339(),
            inactivity_timeout_minutes: timeout_minutes,
        };

        // 1. Acesso 15 minutos depois: dentro do limite (OK)
        let active_check =
            CoreServices::verify_session_active_at(&session, base_time + Duration::minutes(15));
        assert!(active_check.is_ok());

        // 2. Acesso 30 minutos depois: limite exato (OK)
        let boundary_check =
            CoreServices::verify_session_active_at(&session, base_time + Duration::minutes(30));
        assert!(boundary_check.is_ok());

        // 3. Acesso 31 minutos depois: expirou no caminho real do core (ERRO)
        let expired_check =
            CoreServices::verify_session_active_at(&session, base_time + Duration::minutes(31));
        assert!(expired_check.is_err());
    }

    #[test]
    fn test_command_registry_matches_generate_handler_in_lib_rs() {
        // Validação estrutural estrita: extrai a lista do generate_handler! em src/lib.rs
        // e falha se houver qualquer divergência em relação a COMMAND_REGISTRY.
        let lib_path = std::path::Path::new("src/lib.rs");
        let content = if lib_path.exists() {
            std::fs::read_to_string(lib_path).expect("Falha ao ler src/lib.rs")
        } else {
            let alt_path = std::path::Path::new("src-tauri/src/lib.rs");
            std::fs::read_to_string(alt_path).expect("Falha ao ler src-tauri/src/lib.rs")
        };

        let start_marker = "generate_handler![";
        let start_pos = content
            .find(start_marker)
            .expect("generate_handler![ não encontrado em lib.rs")
            + start_marker.len();
        let end_pos = content[start_pos..]
            .find(']')
            .expect("Fechamento ] do generate_handler! não encontrado")
            + start_pos;

        let handler_block = &content[start_pos..end_pos];
        let mut handler_commands: Vec<&str> = handler_block
            .lines()
            .map(|l| l.trim().trim_end_matches(','))
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .map(|l| l.rsplit("::").next().unwrap_or(l))
            .collect();
        handler_commands.sort();

        let mut registry_commands: Vec<&str> = COMMAND_REGISTRY.iter().map(|c| c.name).collect();
        registry_commands.sort();

        assert_eq!(
            handler_commands, registry_commands,
            "Divergência detectada entre generate_handler! e COMMAND_REGISTRY!"
        );
    }
}
