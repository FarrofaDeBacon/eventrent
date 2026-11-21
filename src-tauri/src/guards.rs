use crate::AppState;
use eventrent_core::SessionInfo;

/// Definição formal de comando registrado no runtime Tauri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDefinition {
    pub name: &'static str,
    pub is_public: bool,
}

/// Registro centralizado e canônico de todos os comandos invocáveis do Tauri.
/// Usado pela suíte de auditoria para garantir sincronia estrita com generate_handler!.
pub const COMMAND_REGISTRY: &[CommandDefinition] = &[
    // Comandos públicos (pré-autenticação / ciclo de vida da sessão)
    CommandDefinition {
        name: "get_app_status",
        is_public: true,
    },
    CommandDefinition {
        name: "complete_initial_setup",
        is_public: true,
    },
    CommandDefinition {
        name: "get_public_branding",
        is_public: true,
    },
    CommandDefinition {
        name: "login",
        is_public: true,
    },
    CommandDefinition {
        name: "logout",
        is_public: true,
    },
    CommandDefinition {
        name: "get_current_session",
        is_public: true,
    },
    CommandDefinition {
        name: "recover_password",
        is_public: true,
    },
    // Comandos protegidos por autenticação (o core é a autoridade única de permissões)
    CommandDefinition {
        name: "regenerate_recovery_key",
        is_public: false,
    },
    CommandDefinition {
        name: "get_branding",
        is_public: false,
    },
    CommandDefinition {
        name: "save_branding",
        is_public: false,
    },
    CommandDefinition {
        name: "list_users",
        is_public: false,
    },
    CommandDefinition {
        name: "create_user",
        is_public: false,
    },
    CommandDefinition {
        name: "update_user",
        is_public: false,
    },
    CommandDefinition {
        name: "set_user_active",
        is_public: false,
    },
    CommandDefinition {
        name: "reset_user_password",
        is_public: false,
    },
    CommandDefinition {
        name: "list_roles",
        is_public: false,
    },
    CommandDefinition {
        name: "list_customers",
        is_public: false,
    },
    CommandDefinition {
        name: "get_customer",
        is_public: false,
    },
    CommandDefinition {
        name: "create_customer",
        is_public: false,
    },
    CommandDefinition {
        name: "update_customer",
        is_public: false,
    },
    CommandDefinition {
        name: "set_customer_active",
        is_public: false,
    },
    CommandDefinition {
        name: "delete_customer",
        is_public: false,
    },
    CommandDefinition {
        name: "add_customer_role",
        is_public: false,
    },
    CommandDefinition {
        name: "list_venues",
        is_public: false,
    },
    CommandDefinition {
        name: "get_venue",
        is_public: false,
    },
    CommandDefinition {
        name: "create_venue",
        is_public: false,
    },
    CommandDefinition {
        name: "update_venue",
        is_public: false,
    },
    CommandDefinition {
        name: "set_venue_active",
        is_public: false,
    },
    CommandDefinition {
        name: "delete_venue",
        is_public: false,
    },
    CommandDefinition {
        name: "check_venue_duplicate_name_city",
        is_public: false,
    },
    CommandDefinition {
        name: "get_allocation_settings",
        is_public: false,
    },
    CommandDefinition {
        name: "update_allocation_settings",
        is_public: false,
    },
    CommandDefinition {
        name: "list_brands",
        is_public: false,
    },
    CommandDefinition {
        name: "create_brand",
        is_public: false,
    },
    CommandDefinition {
        name: "update_brand",
        is_public: false,
    },
    CommandDefinition {
        name: "delete_brand",
        is_public: false,
    },
    CommandDefinition {
        name: "list_categories_tree",
        is_public: false,
    },
    CommandDefinition {
        name: "get_category_effective_attributes",
        is_public: false,
    },
    CommandDefinition {
        name: "create_category",
        is_public: false,
    },
    CommandDefinition {
        name: "update_category",
        is_public: false,
    },
    CommandDefinition {
        name: "delete_category",
        is_public: false,
    },
    CommandDefinition {
        name: "create_attribute_definition",
        is_public: false,
    },
    CommandDefinition {
        name: "update_attribute_definition",
        is_public: false,
    },
    CommandDefinition {
        name: "delete_attribute_definition",
        is_public: false,
    },
    CommandDefinition {
        name: "list_equipment_models",
        is_public: false,
    },
    CommandDefinition {
        name: "get_equipment_model",
        is_public: false,
    },
    CommandDefinition {
        name: "create_equipment_model",
        is_public: false,
    },
    CommandDefinition {
        name: "update_equipment_model",
        is_public: false,
    },
    CommandDefinition {
        name: "set_equipment_model_active",
        is_public: false,
    },
    CommandDefinition {
        name: "delete_equipment_model",
        is_public: false,
    },
    CommandDefinition {
        name: "load_suggested_catalog_categories",
        is_public: false,
    },
    CommandDefinition {
        name: "list_model_media",
        is_public: false,
    },
    CommandDefinition {
        name: "get_media_blob_bytes",
        is_public: false,
    },
    CommandDefinition {
        name: "attach_model_photo",
        is_public: false,
    },
    CommandDefinition {
        name: "attach_model_document",
        is_public: false,
    },
    CommandDefinition {
        name: "update_model_media_metadata",
        is_public: false,
    },
    CommandDefinition {
        name: "reorder_model_media",
        is_public: false,
    },
    CommandDefinition {
        name: "set_primary_model_photo",
        is_public: false,
    },
    CommandDefinition {
        name: "remove_model_media",
        is_public: false,
    },
    CommandDefinition {
        name: "collect_unreferenced_media",
        is_public: false,
    },
    CommandDefinition {
        name: "open_model_document_external",
        is_public: false,
    },
    CommandDefinition {
        name: "save_model_document_as",
        is_public: false,
    },
];

/// Guarda central de autenticação para comandos protegidos do Tauri.
///
/// Todos os comandos não públicos obrigatoriamente passam por esta guarda.
/// Ela garante:
/// 1. Existência de sessão ativa;
/// 2. Validação de expiração por inatividade;
/// 3. Atualização automática do timestamp de última atividade (touch).
///
/// Nota: A autoridade e validação de permissões RBAC pertencem exclusivamente ao Core.
pub async fn require_session(state: &AppState) -> Result<SessionInfo, String> {
    state.require_session().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use eventrent_core::CoreServices;

    #[test]
    fn test_all_protected_commands_call_require_session_guard() {
        let commands_dir = std::path::Path::new("src/commands");
        let commands_path = if commands_dir.exists() {
            commands_dir.to_path_buf()
        } else {
            std::path::Path::new("src-tauri/src/commands").to_path_buf()
        };

        let mut all_code = String::new();
        for entry in std::fs::read_dir(&commands_path).expect("Falha ao ler diretório de comandos")
        {
            let entry = entry.expect("Entrada inválida");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let code = std::fs::read_to_string(&path).expect("Falha ao ler arquivo de comando");
                all_code.push_str(&code);
                all_code.push('\n');
            }
        }

        for cmd in COMMAND_REGISTRY {
            if !cmd.is_public {
                let fn_pattern = format!("pub async fn {}(", cmd.name);
                let fn_pos = all_code.find(&fn_pattern).unwrap_or_else(|| {
                    panic!(
                        "Handler para comando protegido '{}' não encontrado em {:?}",
                        cmd.name, commands_path
                    )
                });
                let slice = &all_code[fn_pos..];
                let fn_end = slice.find("}\n").unwrap_or(slice.len());
                let body = &slice[..fn_end];

                assert!(
                    body.contains("require_session(&state)"),
                    "Comando protegido '{}' não invoca a guarda require_session(&state)!",
                    cmd.name
                );
            }
        }
    }

    #[test]
    fn test_commands_call_only_core_services_never_stores() {
        use std::path::Path;

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let commands_dir = manifest_dir.join("src").join("commands");

        let mut forbidden_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&commands_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                    let content =
                        std::fs::read_to_string(&path).expect("Falha ao ler arquivo de comando");
                    if content.contains(".stores") {
                        forbidden_files.push(path.display().to_string());
                    }
                }
            }
        }

        assert!(
            forbidden_files.is_empty(),
            "Comandos Tauri devem chamar SOMENTE métodos de CoreServices, nunca stores direto! Arquivos com violação: {:?}",
            forbidden_files
        );
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
            auth_epoch: 0,
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

    #[test]
    fn test_public_commands_expose_only_public_branding_without_internal_ids() {
        use crate::commands::setup::AppStatus;
        use eventrent_core::{AppConfig, OperationMode, PublicBranding, TenantBranding};

        let internal_branding = TenantBranding {
            id: "internal-id-12345".to_string(),
            tenant_id: "018f0000-0000-7000-8000-000000000001".to_string(),
            company_name: "Locadora Áudio & Luz".to_string(),
            primary_color: "#1e3a8a".to_string(),
            secondary_color: "#3b82f6".to_string(),
            accent_color: "#f59e0b".to_string(),
            logo_url: Some("data:image/png;base64,AAA".to_string()),
            logo_dark_url: None,
            icon_url: None,
            favicon_url: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
            deleted_at: None,
            created_by: Some("user-admin-999".to_string()),
        };

        let public_branding = PublicBranding::from(internal_branding);
        let app_status = AppStatus {
            config: AppConfig {
                mode: Some(OperationMode::Local),
                setup_completed: true,
                inactivity_timeout_minutes: 30,
            },
            default_tenant_id: "018f0000-0000-7000-8000-000000000001".to_string(),
            has_owner: true,
            session: None,
            branding: Some(public_branding),
        };

        let json_val = serde_json::to_value(&app_status).expect("Serialização falhou");
        let branding_obj = json_val
            .get("branding")
            .and_then(|b| b.as_object())
            .expect("branding deve ser um objeto JSON");

        // Campos permitidos na identidade visual pública
        assert_eq!(
            branding_obj.get("company_name").and_then(|v| v.as_str()),
            Some("Locadora Áudio & Luz")
        );
        assert_eq!(
            branding_obj.get("primary_color").and_then(|v| v.as_str()),
            Some("#1e3a8a")
        );

        // Campos internos/banco NUNCA devem estar presentes no PublicBranding serializado
        assert!(
            !branding_obj.contains_key("id"),
            "PublicBranding não pode expor id interno"
        );
        assert!(
            !branding_obj.contains_key("tenant_id"),
            "PublicBranding não pode expor tenant_id"
        );
        assert!(
            !branding_obj.contains_key("created_by"),
            "PublicBranding não pode expor created_by"
        );
        assert!(
            !branding_obj.contains_key("created_at"),
            "PublicBranding não pode expor created_at"
        );
        assert!(
            !branding_obj.contains_key("updated_at"),
            "PublicBranding não pode expor updated_at"
        );
        assert!(
            !branding_obj.contains_key("deleted_at"),
            "PublicBranding não pode expor deleted_at"
        );
    }

    #[test]
    fn test_tauri_capabilities_enforce_least_privilege() {
        use std::path::Path;

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cap_dir = manifest_dir.join("capabilities");
        assert!(
            cap_dir.exists(),
            "Diretório de capabilities deve existir em {}",
            cap_dir.display()
        );

        let entries = std::fs::read_dir(&cap_dir).expect("Falha ao ler diretório de capabilities");

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Falha ao ler {}: {e}", path.display()));
                let val: serde_json::Value = serde_json::from_str(&content)
                    .unwrap_or_else(|e| panic!("JSON inválido em {}: {e}", path.display()));

                let permissions = val
                    .get("permissions")
                    .and_then(|p| p.as_array())
                    .expect("Campo 'permissions' deve ser um array");

                for perm in permissions {
                    let perm_str = perm.as_str().unwrap_or("");
                    assert!(
                        !perm_str.starts_with("opener:"),
                        "Violação de menor privilégio em {}: janela principal não pode receber permissão de opener ({perm_str})",
                        path.display()
                    );
                    assert!(
                        !perm_str.starts_with("fs:"),
                        "Violação de menor privilégio em {}: janela principal não pode receber permissão de fs ({perm_str})",
                        path.display()
                    );
                    assert!(
                        !perm_str.starts_with("http:"),
                        "Violação de menor privilégio em {}: janela principal não pode receber permissão de http ({perm_str})",
                        path.display()
                    );
                    assert!(
                        !perm_str.starts_with("dialog:"),
                        "Violação de menor privilégio em {}: janela principal não pode receber permissão direta de dialog ({perm_str})",
                        path.display()
                    );
                }
            }
        }
    }

    #[test]
    fn test_tauri_csp_is_strict_and_free_of_unsafe_eval() {
        use std::path::Path;

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let conf_path = manifest_dir.join("tauri.conf.json");
        assert!(
            conf_path.exists(),
            "tauri.conf.json deve existir em {}",
            conf_path.display()
        );

        let content = std::fs::read_to_string(&conf_path)
            .unwrap_or_else(|e| panic!("Falha ao ler {}: {e}", conf_path.display()));
        let val: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("JSON inválido em {}: {e}", conf_path.display()));

        let csp_value = val
            .get("app")
            .and_then(|a| a.get("security"))
            .and_then(|s| s.get("csp"));

        let csp_str = csp_value
            .and_then(|v| v.as_str())
            .expect("app.security.csp deve ser uma string não-nula e válida");

        assert!(!csp_str.trim().is_empty(), "CSP não pode ser string vazia");

        // Não pode conter 'unsafe-eval'
        assert!(
            !csp_str.contains("'unsafe-eval'"),
            "CSP não pode conter 'unsafe-eval': {csp_str}"
        );

        // script-src não pode ter wildcards (*) nem http:// / https://
        let script_src_part = csp_str
            .split(';')
            .find(|p| p.trim().starts_with("script-src"))
            .expect("CSP deve definir diretiva script-src");

        assert!(
            !script_src_part.contains('*'),
            "script-src não pode conter wildcard (*): {script_src_part}"
        );
        assert!(
            !script_src_part.contains("http:"),
            "script-src não pode permitir http: {script_src_part}"
        );
        assert!(
            !script_src_part.contains("https:"),
            "script-src não pode permitir https: {script_src_part}"
        );
        assert!(
            !script_src_part.contains("'unsafe-inline'"),
            "script-src não pode conter 'unsafe-inline': {script_src_part}"
        );
        assert!(
            script_src_part.contains("'self'"),
            "script-src deve conter 'self': {script_src_part}"
        );

        // style-src permite 'unsafe-inline' para Tailwind/Svelte
        let style_src_part = csp_str
            .split(';')
            .find(|p| p.trim().starts_with("style-src"))
            .expect("CSP deve definir diretiva style-src");
        assert!(
            style_src_part.contains("'unsafe-inline'"),
            "style-src deve permitir 'unsafe-inline' para tokens dinâmicos OKLCH do Tailwind"
        );
    }
}
