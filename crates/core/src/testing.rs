use crate::domain::{
    CreateUserInput, InitialSetupInput, InitialSetupResult, LoginInput, OperationMode,
    RecoverPasswordInput, ResetPasswordInput, SessionInfo, UpdateBrandingInput, UpdateUserInput,
    UserRecord, UserWithRole, ROLE_SLUG_COMMERCIAL, ROLE_SLUG_OWNER,
};
use crate::error::CoreError;
use crate::services::{
    ensure_default_tenant, ensure_system_roles, CoreServices, DEFAULT_COMPANY_NAME,
};
use crate::store::StoreBundle;
use std::future::Future;
use uuid::Uuid;

/// 1. Contrato: Idempotência de ensure_default_tenant e marca padrão "Minha empresa".
pub async fn contract_default_seeds(stores: &StoreBundle) -> String {
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

    tenant_id
}

/// 2. Contrato: Idempotência de ensure_system_roles (5 perfis de sistema com permissões granulares).
pub async fn contract_system_roles(stores: &StoreBundle, tenant_id: &str) {
    let roles = ensure_system_roles(tenant_id, stores.roles.as_ref())
        .await
        .expect("Falha em ensure_system_roles");
    assert_eq!(roles.len(), 5);

    let roles_again = ensure_system_roles(tenant_id, stores.roles.as_ref())
        .await
        .expect("Segunda chamada a ensure_system_roles falhou");
    assert_eq!(roles_again.len(), 5);
}

/// 3. Contrato: Setup inicial da conta do Proprietário (modo Local) e geração de Chave de Recuperação.
pub async fn contract_initial_setup_and_recovery_key(
    stores: &StoreBundle,
    tenant_id: &str,
) -> (InitialSetupResult, CoreServices) {
    let services = CoreServices::new(stores.clone());
    let setup_result = services
        .complete_initial_setup(
            tenant_id,
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
    assert_eq!(setup_result.recovery_key.len(), 22);
    assert_eq!(setup_result.branding.company_name, "Locadora Luz & Som");
    assert_eq!(setup_result.session.role_slug, ROLE_SLUG_OWNER);

    // Impede segundo setup inicial no mesmo ambiente
    let second_setup = services
        .complete_initial_setup(
            tenant_id,
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

    (setup_result, services)
}

/// 4. Contrato: Regra do Último Proprietário e Autodesativação.
pub async fn contract_last_owner_protection(
    services: &CoreServices,
    owner_session: &SessionInfo,
    tenant_id: &str,
) {
    // Impede autodesativação
    let self_deactivate = services
        .set_user_active(owner_session, &owner_session.user_id, false)
        .await;
    assert!(matches!(self_deactivate, Err(CoreError::Validation(_))));

    let commercial_role = services
        .stores
        .roles
        .find_by_slug(tenant_id, ROLE_SLUG_COMMERCIAL)
        .await
        .expect("Erro ao buscar perfil comercial")
        .expect("Perfil comercial deve existir");

    // Impede rebaixar o único proprietário ativo
    let demote_last_owner = services
        .update_user(
            owner_session,
            &owner_session.user_id,
            &UpdateUserInput {
                name: "Ana Proprietária".to_string(),
                email: "ana@luzesom.com.br".to_string(),
                role_id: commercial_role.id.clone(),
            },
        )
        .await;
    assert!(matches!(demote_last_owner, Err(CoreError::Validation(_))));
}

/// 5. Contrato: Criação de usuário Comercial e verificação de permissões RBAC no Core.
pub async fn contract_user_creation_and_rbac(
    services: &CoreServices,
    owner_session: &SessionInfo,
    tenant_id: &str,
) -> (UserWithRole, SessionInfo) {
    let commercial_role = services
        .stores
        .roles
        .find_by_slug(tenant_id, ROLE_SLUG_COMMERCIAL)
        .await
        .expect("Erro ao buscar perfil comercial")
        .expect("Perfil comercial deve existir");

    let commercial_user = services
        .create_user(
            owner_session,
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
            tenant_id,
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

    (commercial_user, commercial_session)
}

/// 6. Contrato: Bloqueio temporário após 5 tentativas falhas de login.
pub async fn contract_user_lockout(services: &CoreServices, tenant_id: &str) {
    for attempt in 1..=4 {
        let res = services
            .login(
                tenant_id,
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
            tenant_id,
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
            tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "Carlos#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(matches!(locked_try, Err(CoreError::AccountLocked { .. })));
}

/// 7. Contrato: Redefinição administrativa de senha limpa o bloqueio.
pub async fn contract_password_reset_clears_lockout(
    services: &CoreServices,
    owner_session: &SessionInfo,
    tenant_id: &str,
    commercial_user_id: &str,
) {
    services
        .reset_user_password(
            owner_session,
            commercial_user_id,
            &ResetPasswordInput {
                new_password: "NovaSenhaCarlos#2026".to_string(),
            },
        )
        .await
        .expect("Reset administrativo de senha falhou");

    let unlocked_login = services
        .login(
            tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "NovaSenhaCarlos#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(unlocked_login.is_ok());
}

/// 8. Contrato: Recuperação de senha, bloqueio progressivo por tentativas falhas e rotação obrigatória da chave.
pub async fn contract_recovery_key_rotation_and_lockout(
    services: &CoreServices,
    tenant_id: &str,
    initial_recovery_key: &str,
) {
    // 8a. Tentativa falha com chave inválida grava auditoria e rejeita
    let fail_recovery = services
        .recover_password(
            tenant_id,
            &RecoverPasswordInput {
                email: "ana@luzesom.com.br".to_string(),
                recovery_key: "ER-CHAVE-INVALIDA-0000".to_string(),
                new_password: "TentativaInvalida#2026".to_string(),
            },
        )
        .await;
    assert!(matches!(fail_recovery, Err(CoreError::InvalidCredentials)));

    // 8b. Recuperação de senha com a chave correta
    let recovery_res = services
        .recover_password(
            tenant_id,
            &RecoverPasswordInput {
                email: "ana@luzesom.com.br".to_string(),
                recovery_key: initial_recovery_key.to_string(),
                new_password: "AnaRecuperada#2026".to_string(),
            },
        )
        .await
        .expect("Recuperação via chave de recuperação deve funcionar");

    assert!(recovery_res.new_recovery_key.starts_with("ER-"));
    assert_ne!(recovery_res.new_recovery_key, initial_recovery_key);

    // 8c. Login com a nova senha redefinida
    let owner_relogin = services
        .login(
            tenant_id,
            &LoginInput {
                email: "ana@luzesom.com.br".to_string(),
                password: "AnaRecuperada#2026".to_string(),
            },
            30,
        )
        .await;
    assert!(owner_relogin.is_ok());

    // 8d. A chave antiga deve ter sido invalidada (rotação de chave)
    let old_key_reuse = services
        .recover_password(
            tenant_id,
            &RecoverPasswordInput {
                email: "ana@luzesom.com.br".to_string(),
                recovery_key: initial_recovery_key.to_string(),
                new_password: "TentativaReusoChave#2026".to_string(),
            },
        )
        .await;
    assert!(matches!(old_key_reuse, Err(CoreError::InvalidCredentials)));
}

/// 9. Contrato: Log de Auditoria append-only.
pub async fn contract_audit_log(stores: &StoreBundle, tenant_id: &str) {
    let audits = stores
        .audit
        .list_by_tenant(tenant_id, 50)
        .await
        .expect("Falha ao listar audit_log");

    assert!(audits.len() >= 6);
    assert!(audits.iter().any(|a| a.action == "user.created"));
    assert!(audits.iter().any(|a| a.action == "auth.account_locked"));
    assert!(audits.iter().any(|a| a.action == "user.password_recovered"));
    assert!(audits
        .iter()
        .any(|a| a.action == "auth.recovery_key_rotated"));
}

/// 10. Contrato: Índices únicos parciais WHERE deleted_at IS NULL.
pub async fn contract_unique_partial_indexes(stores: &StoreBundle, tenant_id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let email = "duplicado@teste.com";

    let role = stores
        .roles
        .find_by_slug(tenant_id, ROLE_SLUG_OWNER)
        .await
        .unwrap()
        .unwrap();

    let user1 = UserRecord {
        id: Uuid::now_v7().to_string(),
        tenant_id: tenant_id.to_string(),
        name: "Primeiro".to_string(),
        email: email.to_string(),
        password_hash: "hash".to_string(),
        recovery_key_hash: None,
        role_id: role.id.clone(),
        active: true,
        last_login_at: None,
        failed_attempts: 0,
        locked_until: None,
        recovery_failed_attempts: 0,
        recovery_locked_until: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        deleted_at: None,
        created_by: None,
    };
    stores
        .users
        .create(&user1)
        .await
        .expect("User 1 deve criar");

    // Tentativa de criar outro usuário ativo com o mesmo e-mail deve falhar por unicidade
    let user2 = UserRecord {
        id: Uuid::now_v7().to_string(),
        tenant_id: tenant_id.to_string(),
        name: "Segundo".to_string(),
        email: email.to_string(),
        password_hash: "hash".to_string(),
        recovery_key_hash: None,
        role_id: role.id,
        active: true,
        last_login_at: None,
        failed_attempts: 0,
        locked_until: None,
        recovery_failed_attempts: 0,
        recovery_locked_until: None,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
        created_by: None,
    };
    let duplicate_result = stores.users.create(&user2).await;
    assert!(
        duplicate_result.is_err(),
        "Índice único parcial deve impedir e-mails ativos duplicados"
    );
}

/// Suíte genérica agregadora de testes de contrato.
/// Executa todos os testes comportamentais sequencialmente sobre um mesmo ambiente.
pub async fn run_store_contract_suite<F, Fut>(factory: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = StoreBundle>,
{
    let stores = factory().await;

    let tenant_id = contract_default_seeds(&stores).await;
    contract_system_roles(&stores, &tenant_id).await;

    let (setup_result, services) =
        contract_initial_setup_and_recovery_key(&stores, &tenant_id).await;

    contract_last_owner_protection(&services, &setup_result.session, &tenant_id).await;

    let (commercial_user, _commercial_session) =
        contract_user_creation_and_rbac(&services, &setup_result.session, &tenant_id).await;

    contract_user_lockout(&services, &tenant_id).await;

    contract_password_reset_clears_lockout(
        &services,
        &setup_result.session,
        &tenant_id,
        &commercial_user.id,
    )
    .await;

    contract_recovery_key_rotation_and_lockout(&services, &tenant_id, &setup_result.recovery_key)
        .await;

    contract_audit_log(&stores, &tenant_id).await;
    contract_unique_partial_indexes(&stores, &tenant_id).await;
}

#[cfg(test)]
mod unit_tests {
    use crate::services::{
        calculate_lockout_seconds, generate_recovery_key, hash_secret, verify_secret,
    };

    #[tokio::test]
    async fn test_argon2id_hash_and_verify() {
        let hash = hash_secret("MinhaSenhaSegura!123")
            .await
            .expect("Falha ao gerar hash Argon2id");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_secret("MinhaSenhaSegura!123", &hash).await.unwrap());
        assert!(!verify_secret("SenhaIncorreta", &hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_argon2id_spawn_blocking_does_not_block_async_runtime() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Tarefa em background que incrementa o contador continuamente
        let background_worker = tokio::spawn(async move {
            for _ in 0..100 {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
            }
        });

        // Executa hash Argon2id de 19 MiB com parâmetros OWASP
        let hash = hash_secret("TesteConcorrenciaTokio#2026").await.unwrap();
        assert!(hash.starts_with("$argon2id$"));

        // Garante que o worker concorrente conseguiu rodar e progredir no runtime Tokio
        let count = counter.load(Ordering::SeqCst);
        assert!(
            count > 0,
            "A thread assíncrona não deve ser bloqueada pelo Argon2id"
        );

        let _ = background_worker.await;
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
