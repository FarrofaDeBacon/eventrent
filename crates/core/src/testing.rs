use crate::domain::{
    CreateUserInput, InitialSetupInput, LoginInput, OperationMode, RecoverPasswordInput,
    ResetPasswordInput, UpdateBrandingInput, UpdateUserInput, ROLE_SLUG_COMMERCIAL,
    ROLE_SLUG_OWNER,
};
use crate::error::CoreError;
use crate::services::{
    ensure_default_tenant, ensure_system_roles, CoreServices, DEFAULT_COMPANY_NAME,
};
use crate::store::StoreBundle;
use std::future::Future;

/// Suíte genérica de testes de contrato que recebe uma fábrica assíncrona de `StoreBundle`
/// e valida todas as traits (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`)
/// e as regras de negócio de `CoreServices` (setup inicial, Argon2id, chave de recuperação,
/// bloqueio após 5 falhas, permissões, último proprietário, autodesativação e auditoria).
///
/// Na Fase 1A executa contra `store-sqlite`; na Fase 2.5 executará contra `store-postgres` sem alterações.
pub async fn run_store_contract_suite<F, Fut>(factory: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = StoreBundle>,
{
    let stores = factory().await;
    let services = CoreServices::new(stores.clone());

    // 1. Idempotência de ensure_default_tenant e marca padrão "Minha empresa"
    let tenant_id = ensure_default_tenant(stores.tenants.as_ref(), stores.branding.as_ref())
        .await
        .expect("Falha em ensure_default_tenant");
    let tenant_id_again = ensure_default_tenant(stores.tenants.as_ref(), stores.branding.as_ref())
        .await
        .expect("Segunda chamada a ensure_default_tenant falhou");
    assert_eq!(tenant_id, tenant_id_again);

    let initial_branding = stores
        .branding
        .get_by_tenant_id(&tenant_id)
        .await
        .expect("Falha ao buscar branding")
        .expect("Branding inicial deve existir");
    assert_eq!(initial_branding.company_name, DEFAULT_COMPANY_NAME);

    // 2. Idempotência de ensure_system_roles (5 perfis de sistema)
    let roles = ensure_system_roles(&tenant_id, stores.roles.as_ref())
        .await
        .expect("Falha em ensure_system_roles");
    assert_eq!(roles.len(), 5);
    let roles_again = ensure_system_roles(&tenant_id, stores.roles.as_ref())
        .await
        .expect("Segunda chamada a ensure_system_roles falhou");
    assert_eq!(roles_again.len(), 5);

    // 3. Setup inicial da conta do Proprietário (modo Local) + Chave de Recuperação
    let setup_result = services
        .complete_initial_setup(
            &tenant_id,
            &InitialSetupInput {
                mode: OperationMode::Local,
                company_name: "Locadora Luz & Som".to_string(),
                owner_name: "Ana Proprietária".to_string(),
                owner_email: "ana@luzesom.com.br".to_string(),
                owner_password: "SenhaForte#2026".to_string(),
            },
            30,
        )
        .await
        .expect("Falha no setup inicial");

    assert!(setup_result.recovery_key.starts_with("ER-"));
    assert_eq!(setup_result.branding.company_name, "Locadora Luz & Som");
    assert_eq!(setup_result.session.role_slug, ROLE_SLUG_OWNER);

    // Impede segundo setup inicial no mesmo tenant
    let second_setup = services
        .complete_initial_setup(
            &tenant_id,
            &InitialSetupInput {
                mode: OperationMode::Local,
                company_name: "Outra".to_string(),
                owner_name: "Outro".to_string(),
                owner_email: "outro@luzesom.com.br".to_string(),
                owner_password: "SenhaForte#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(matches!(second_setup, Err(CoreError::Conflict(_))));

    let owner_session = setup_result.session;

    // 4. Regra do Último Proprietário e Autodesativação
    let self_deactivate = services
        .set_user_active(&owner_session, &owner_session.user_id, false)
        .await;
    assert!(matches!(self_deactivate, Err(CoreError::Validation(_))));

    let commercial_role = stores
        .roles
        .find_by_slug(&tenant_id, ROLE_SLUG_COMMERCIAL)
        .await
        .expect("Erro ao buscar perfil comercial")
        .expect("Perfil comercial deve existir");

    let demote_last_owner = services
        .update_user(
            &owner_session,
            &owner_session.user_id,
            &UpdateUserInput {
                name: "Ana Proprietária".to_string(),
                email: "ana@luzesom.com.br".to_string(),
                role_id: commercial_role.id.clone(),
            },
        )
        .await;
    assert!(matches!(demote_last_owner, Err(CoreError::Validation(_))));

    // 5. Criação de usuário Comercial e verificação de permissões no Core
    let commercial_user = services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Carlos Comercial".to_string(),
                email: "carlos@luzesom.com.br".to_string(),
                password: "Carlos#2026".to_string(),
                role_id: commercial_role.id.clone(),
            },
        )
        .await
        .expect("Falha ao criar usuário comercial");

    assert_eq!(commercial_user.role_slug, ROLE_SLUG_COMMERCIAL);

    let commercial_session = services
        .login(
            &tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "Carlos#2026".to_string(),
            },
            30,
        )
        .await
        .expect("Login do usuário comercial deve funcionar");

    // Usuário comercial NÃO tem permissão user:manage nem branding:manage
    let forbidden_create = services
        .create_user(
            &commercial_session,
            &CreateUserInput {
                name: "Intruso".to_string(),
                email: "intruso@luzesom.com.br".to_string(),
                password: "Intruso#2026".to_string(),
                role_id: commercial_role.id.clone(),
            },
        )
        .await;
    assert!(matches!(forbidden_create, Err(CoreError::Forbidden(_))));

    let forbidden_branding = services
        .save_branding(
            &commercial_session,
            &UpdateBrandingInput {
                company_name: "Invasão".to_string(),
                primary_color: "#000000".to_string(),
                secondary_color: "#000000".to_string(),
                accent_color: "#000000".to_string(),
                logo_url: None,
            },
        )
        .await;
    assert!(matches!(forbidden_branding, Err(CoreError::Forbidden(_))));

    // 6. Bloqueio temporário após 5 tentativas falhas de login
    for attempt in 1..=4 {
        let res = services
            .login(
                &tenant_id,
                &LoginInput {
                    email: "carlos@luzesom.com.br".to_string(),
                    password: "SenhaErrada".to_string(),
                },
                30,
            )
            .await;
        assert!(
            matches!(res, Err(CoreError::InvalidCredentials)),
            "Tentativa {attempt} deveria retornar InvalidCredentials"
        );
    }

    let fifth_attempt = services
        .login(
            &tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "SenhaErrada".to_string(),
            },
            30,
        )
        .await;
    assert!(matches!(
        fifth_attempt,
        Err(CoreError::AccountLocked { .. })
    ));

    // Mesmo com a senha correta, enquanto bloqueado deve recusar com AccountLocked
    let locked_try = services
        .login(
            &tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "Carlos#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(matches!(locked_try, Err(CoreError::AccountLocked { .. })));

    // 7. Redefinição administrativa de senha limpa o bloqueio
    services
        .reset_user_password(
            &owner_session,
            &commercial_user.id,
            &ResetPasswordInput {
                new_password: "NovaSenhaCarlos#2026".to_string(),
            },
        )
        .await
        .expect("Reset administrativo de senha falhou");

    let unlocked_login = services
        .login(
            &tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "NovaSenhaCarlos#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(unlocked_login.is_ok());

    // 8. Recuperação de senha do proprietário usando a Chave de Recuperação
    services
        .recover_password(
            &tenant_id,
            &RecoverPasswordInput {
                email: "ana@luzesom.com.br".to_string(),
                recovery_key: setup_result.recovery_key.clone(),
                new_password: "AnaRecuperada#2026".to_string(),
            },
        )
        .await
        .expect("Recuperação via chave de recuperação deve funcionar");

    let owner_relogin = services
        .login(
            &tenant_id,
            &LoginInput {
                email: "ana@luzesom.com.br".to_string(),
                password: "AnaRecuperada#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(owner_relogin.is_ok());

    // 9. Verifica registros no audit_log
    let audits = stores
        .audit
        .list_by_tenant(&tenant_id, 50)
        .await
        .expect("Falha ao listar audit_log");
    assert!(audits.len() >= 6);
    assert!(audits.iter().any(|a| a.action == "user.created"));
    assert!(audits.iter().any(|a| a.action == "auth.account_locked"));
    assert!(audits.iter().any(|a| a.action == "user.password_recovered"));
}

#[cfg(test)]
mod unit_tests {
    use crate::services::{
        calculate_lockout_seconds, generate_recovery_key, hash_secret, verify_secret,
    };

    #[test]
    fn test_argon2id_hash_and_verify() {
        let hash = hash_secret("MinhaSenhaSegura!123").expect("Falha ao gerar hash Argon2id");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_secret("MinhaSenhaSegura!123", &hash));
        assert!(!verify_secret("SenhaIncorreta", &hash));
    }

    #[test]
    fn test_recovery_key_format_and_uniqueness() {
        let key1 = generate_recovery_key();
        let key2 = generate_recovery_key();
        assert_ne!(key1, key2);
        assert!(key1.starts_with("ER-"));
        assert_eq!(key1.len(), 22); // ER-XXXX-XXXX-XXXX-XXXX
    }

    #[test]
    fn test_lockout_backoff_progression() {
        assert_eq!(calculate_lockout_seconds(1), 0);
        assert_eq!(calculate_lockout_seconds(4), 0);
        assert_eq!(calculate_lockout_seconds(5), 300);
        assert_eq!(calculate_lockout_seconds(6), 600);
        assert_eq!(calculate_lockout_seconds(7), 1200);
        assert_eq!(calculate_lockout_seconds(8), 3600);
        assert_eq!(calculate_lockout_seconds(15), 3600);
    }
}
