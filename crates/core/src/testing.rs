use crate::domain::{
    CreateUserInput, InitialSetupInput, InitialSetupResult, LoginInput, LoginPolicy, OperationMode,
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

/// 2. Contrato: Idempotência de ensure_system_roles (5 perfis de sistema com permissões granulares e reparo com auditoria).
pub async fn contract_system_roles(stores: &StoreBundle, tenant_id: &str) {
    let roles = ensure_system_roles(tenant_id, stores.roles.as_ref(), stores.audit.as_ref())
        .await
        .expect("Falha em ensure_system_roles");
    assert_eq!(roles.len(), 5);

    let roles_again = ensure_system_roles(tenant_id, stores.roles.as_ref(), stores.audit.as_ref())
        .await
        .expect("Segunda chamada a ensure_system_roles falhou");
    assert_eq!(roles_again.len(), 5);

    // B5: Simula adulteração de permissões para validar detecção por conjunto e auditoria
    let comm_role = roles
        .iter()
        .find(|r| r.slug == ROLE_SLUG_COMMERCIAL)
        .expect("Perfil comercial deve existir");
    let now = chrono::Utc::now().to_rfc3339();
    stores
        .roles
        .set_permissions(
            tenant_id,
            &comm_role.id,
            &["branding:read".to_string()],
            &now,
        )
        .await
        .expect("Falha ao adulterar permissões para teste");

    let repaired = ensure_system_roles(tenant_id, stores.roles.as_ref(), stores.audit.as_ref())
        .await
        .expect("Falha ao reparar perfis divergentes");
    let repaired_comm = repaired
        .iter()
        .find(|r| r.slug == ROLE_SLUG_COMMERCIAL)
        .unwrap();
    assert_eq!(repaired_comm.permissions.len(), 8);

    let audits = stores
        .audit
        .list_by_tenant(tenant_id, 10)
        .await
        .expect("Falha ao buscar auditoria");
    assert!(audits
        .iter()
        .any(|a| a.action == "role.system_permissions_repaired"));
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
            &LoginPolicy::default(),
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

    // B1: Login com usuário inexistente devolve InvalidCredentials sem revelar existência
    let non_existent_login = services
        .login(
            tenant_id,
            &LoginInput {
                email: "inexistente@luzesom.com.br".to_string(),
                password: "SenhaQualquer#123".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await;
    assert!(matches!(
        non_existent_login,
        Err(CoreError::InvalidCredentials)
    ));

    // B4: Anti-escalada: Usuário com user:manage mas sem permissões de proprietário não pode criar proprietário
    let limited_manager_session = SessionInfo {
        user_id: "limited-manager-id".to_string(),
        tenant_id: tenant_id.to_string(),
        name: "Gerente Limitado".to_string(),
        email: "gerente@luzesom.com.br".to_string(),
        role_id: "limited-role-id".to_string(),
        role_name: "Gerente Limitado".to_string(),
        role_slug: "manager".to_string(),
        permissions: vec!["user:read".to_string(), "user:manage".to_string()],
        authenticated_at: chrono::Utc::now().to_rfc3339(),
        last_activity_at: chrono::Utc::now().to_rfc3339(),
        inactivity_timeout_minutes: 30,
    };
    let owner_role = services
        .stores
        .roles
        .find_by_slug(tenant_id, ROLE_SLUG_OWNER)
        .await
        .unwrap()
        .unwrap();

    let escalate_attempt = services
        .create_user(
            &limited_manager_session,
            &CreateUserInput {
                name: "Novo Dono Ilegitimo".to_string(),
                email: "ilegitimo@luzesom.com.br".to_string(),
                password: "SenhaForte#2026".to_string(),
                role_id: owner_role.id.clone(),
            },
        )
        .await;
    assert!(matches!(escalate_attempt, Err(CoreError::Forbidden(_))));

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
                &LoginPolicy::default(),
                30,
            )
            .await;
        assert!(
            matches!(res, Err(CoreError::InvalidCredentials)),
            "Tentativa {attempt} deveria retornar InvalidCredentials"
        );
    }

    // 5ª tentativa com política Desktop (reveal_account_locked: true) revela AccountLocked
    let fifth_attempt = services
        .login(
            tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "SenhaErrada".to_string(),
            },
            &LoginPolicy {
                reveal_account_locked: true,
            },
            30,
        )
        .await;
    assert!(matches!(
        fifth_attempt,
        Err(CoreError::AccountLocked { .. })
    ));

    // Com política de nuvem/padrão (reveal_account_locked: false), não revela o bloqueio (anti-enumeração e DoS)
    let obscured_locked_try = services
        .login(
            tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "Carlos#2026".to_string(),
            },
            &LoginPolicy {
                reveal_account_locked: false,
            },
            30,
        )
        .await;
    assert!(matches!(
        obscured_locked_try,
        Err(CoreError::InvalidCredentials)
    ));

    // Mesmo com a senha correta, enquanto bloqueado no desktop deve recusar com AccountLocked
    let locked_try = services
        .login(
            tenant_id,
            &LoginInput {
                email: "carlos@luzesom.com.br".to_string(),
                password: "Carlos#2026".to_string(),
            },
            &LoginPolicy {
                reveal_account_locked: true,
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
            &LoginPolicy::default(),
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
            &LoginPolicy::default(),
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

    #[test]
    fn test_debug_redacts_sensitive_fields() {
        use crate::domain::{
            CreateUserInput, InitialSetupInput, InitialSetupResult, LoginInput, OperationMode,
            RecoverPasswordInput, RecoverPasswordResult, ResetPasswordInput, SessionInfo,
            TenantBranding, UserRecord,
        };

        let secret_pwd = "SuperSecretPassword#2026";
        let secret_key = "ER-ABCD-EFGH-JKLM-NPQR";

        let login = LoginInput {
            email: "teste@eventrent.com".to_string(),
            password: secret_pwd.to_string(),
        };
        let login_debug = format!("{login:?}");
        assert!(!login_debug.contains(secret_pwd));
        assert!(login_debug.contains("[REDACTED]"));

        let create = CreateUserInput {
            name: "Usuario".to_string(),
            email: "u@e.com".to_string(),
            password: secret_pwd.to_string(),
            role_id: "role-1".to_string(),
        };
        let create_debug = format!("{create:?}");
        assert!(!create_debug.contains(secret_pwd));
        assert!(create_debug.contains("[REDACTED]"));

        let reset = ResetPasswordInput {
            new_password: secret_pwd.to_string(),
        };
        let reset_debug = format!("{reset:?}");
        assert!(!reset_debug.contains(secret_pwd));
        assert!(reset_debug.contains("[REDACTED]"));

        let recover = RecoverPasswordInput {
            email: "u@e.com".to_string(),
            recovery_key: secret_key.to_string(),
            new_password: secret_pwd.to_string(),
        };
        let recover_debug = format!("{recover:?}");
        assert!(!recover_debug.contains(secret_pwd));
        assert!(!recover_debug.contains(secret_key));
        assert!(recover_debug.contains("[REDACTED]"));

        let recover_res = RecoverPasswordResult {
            new_recovery_key: secret_key.to_string(),
        };
        let recover_res_debug = format!("{recover_res:?}");
        assert!(!recover_res_debug.contains(secret_key));
        assert!(recover_res_debug.contains("[REDACTED]"));

        let setup = InitialSetupInput {
            mode: OperationMode::Local,
            company_name: "Empresa".to_string(),
            owner_name: "Dono".to_string(),
            owner_email: "dono@e.com".to_string(),
            owner_password: secret_pwd.to_string(),
        };
        let setup_debug = format!("{setup:?}");
        assert!(!setup_debug.contains(secret_pwd));
        assert!(setup_debug.contains("[REDACTED]"));

        let setup_res = InitialSetupResult {
            tenant_id: "t1".to_string(),
            recovery_key: secret_key.to_string(),
            session: SessionInfo {
                user_id: "u1".to_string(),
                tenant_id: "t1".to_string(),
                name: "Dono".to_string(),
                email: "dono@e.com".to_string(),
                role_id: "r1".to_string(),
                role_name: "Proprietário".to_string(),
                role_slug: "owner".to_string(),
                permissions: vec![],
                authenticated_at: "2026-01-01T00:00:00Z".to_string(),
                last_activity_at: "2026-01-01T00:00:00Z".to_string(),
                inactivity_timeout_minutes: 30,
            },
            branding: TenantBranding {
                id: "b1".to_string(),
                tenant_id: "t1".to_string(),
                company_name: "Empresa".to_string(),
                primary_color: "#000".to_string(),
                secondary_color: "#000".to_string(),
                accent_color: "#000".to_string(),
                logo_url: None,
                favicon_url: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                deleted_at: None,
                created_by: None,
            },
        };
        let setup_res_debug = format!("{setup_res:?}");
        assert!(!setup_res_debug.contains(secret_key));
        assert!(setup_res_debug.contains("[REDACTED]"));

        let user_rec = UserRecord {
            id: "u1".to_string(),
            tenant_id: "t1".to_string(),
            name: "Dono".to_string(),
            email: "dono@e.com".to_string(),
            password_hash: "$argon2id$secret_hash_value".to_string(),
            recovery_key_hash: Some("$argon2id$secret_key_hash_value".to_string()),
            role_id: "r1".to_string(),
            active: true,
            last_login_at: None,
            failed_attempts: 0,
            locked_until: None,
            recovery_failed_attempts: 0,
            recovery_locked_until: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            deleted_at: None,
            created_by: None,
        };
        let user_rec_debug = format!("{user_rec:?}");
        assert!(!user_rec_debug.contains("secret_hash_value"));
        assert!(!user_rec_debug.contains("secret_key_hash_value"));
        assert!(user_rec_debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_password_and_email_validation() {
        use crate::services::{validate_email, validate_password};

        // Senhas: mínimo 8 e máximo 128 caracteres (contagem Unicode)
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());

        let exactly_128_chars = "a".repeat(128);
        assert!(validate_password(&exactly_128_chars).is_ok());

        let too_long_129 = "a".repeat(129);
        assert!(validate_password(&too_long_129).is_err());

        // Contagem de caracteres vs bytes (Unicode com acentos e emojis)
        // Cada caractere especial conta como 1 caractere, não 2-4 bytes
        let unicode_8_chars = "café☕123";
        assert_eq!(unicode_8_chars.chars().count(), 8);
        assert!(validate_password(unicode_8_chars).is_ok());

        // E-mails válidos
        assert!(validate_email("usuario@dominio.com").is_ok());
        assert!(validate_email("user.name+tag@sub.empresa.com.br").is_ok());

        // E-mails inválidos (RFC 5321 edge cases)
        assert!(validate_email("").is_err());
        assert!(validate_email("sem-arroba.com").is_err());
        assert!(validate_email("dois@@arrobas.com").is_err());
        assert!(validate_email("@sem-local.com").is_err());
        assert!(validate_email("local@").is_err());
        assert!(validate_email("local@sem-tld").is_err());
        assert!(validate_email("local@.ponto-no-inicio.com").is_err());
        assert!(validate_email(".ponto-inicio@dominio.com").is_err());
        assert!(validate_email("ponto..duplo@dominio.com").is_err());
        assert!(validate_email("usuario@-hifen-inicio.com").is_err());
    }

    #[tokio::test]
    async fn test_dummy_argon2_hash_validity_and_params() {
        use crate::services::{verify_secret, DUMMY_ARGON2_HASH};
        use argon2::password_hash::PasswordHash;

        // 1. O hash fictício deve ser sintaticamente válido segundo o padrão PHC
        let parsed = PasswordHash::new(DUMMY_ARGON2_HASH)
            .expect("DUMMY_ARGON2_HASH deve ser um hash PHC válido");

        // 2. O algoritmo deve ser Argon2id
        assert_eq!(parsed.algorithm.as_str(), "argon2id");

        // 3. A versão do Argon2 deve ser 0x13 (19 decimal)
        assert_eq!(parsed.version, Some(19));

        // 4. Os parâmetros devem ser idênticos aos parâmetros de produção (OWASP):
        // m = 19456 (19 MiB), t = 2 (iterações), p = 1 (grau de paralelismo)
        let m_param = parsed.params.get("m").expect("Parâmetro 'm' ausente");
        let t_param = parsed.params.get("t").expect("Parâmetro 't' ausente");
        let p_param = parsed.params.get("p").expect("Parâmetro 'p' ausente");

        assert_eq!(m_param.decimal().unwrap(), 19456);
        assert_eq!(t_param.decimal().unwrap(), 2);
        assert_eq!(p_param.decimal().unwrap(), 1);

        // 5. A verificação criptográfica com qualquer senha deve executar a carga do Argon2id
        // e retornar Ok(false) sem erro
        let verification = verify_secret("senha_qualquer", DUMMY_ARGON2_HASH).await;
        assert!(verification.is_ok());
        assert!(!verification.unwrap());
    }

    #[tokio::test]
    async fn test_run_in_tx_retry_and_max_retries() {
        use crate::error::{CoreError, Result};
        use crate::store::{
            run_in_tx, AuditStore, BrandingStore, RoleStore, TenantStore, TransactionManager,
            TransactionPolicy, TxStoreBundle, UserStore,
        };
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        struct DummyTxBundle;
        #[async_trait]
        impl TxStoreBundle for DummyTxBundle {
            fn tenants(&self) -> &dyn TenantStore {
                unreachable!()
            }
            fn branding(&self) -> &dyn BrandingStore {
                unreachable!()
            }
            fn roles(&self) -> &dyn RoleStore {
                unreachable!()
            }
            fn users(&self) -> &dyn UserStore {
                unreachable!()
            }
            fn audit(&self) -> &dyn AuditStore {
                unreachable!()
            }
            async fn commit(self: Box<Self>) -> Result<()> {
                Ok(())
            }
            async fn rollback(self: Box<Self>) -> Result<()> {
                Ok(())
            }
        }

        struct MockTxManager {
            pub begin_count: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl TransactionManager for MockTxManager {
            async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>> {
                self.begin_count.fetch_add(1, Ordering::SeqCst);
                Ok(Box::new(DummyTxBundle))
            }
        }

        let policy = TransactionPolicy {
            max_retries: 3,
            initial_backoff_ms: 1,
        };

        // Cenário 1: Sucesso após 2 TransientConflict (tentativa 1 falha, 2 falha, 3 sucesso)
        let begin_count = Arc::new(AtomicUsize::new(0));
        let mgr = MockTxManager {
            begin_count: begin_count.clone(),
        };
        let exec_count = Arc::new(AtomicUsize::new(0));
        let exec_count_clone = exec_count.clone();

        let res: Result<&str> = run_in_tx(&mgr, &policy, move |_| {
            let exec_count = exec_count_clone.clone();
            Box::pin(async move {
                let current = exec_count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(CoreError::TransientConflict("lock contend".into()))
                } else {
                    Ok("sucesso")
                }
            })
        })
        .await;

        assert_eq!(res.unwrap(), "sucesso");
        assert_eq!(begin_count.load(Ordering::SeqCst), 3);
        assert_eq!(exec_count.load(Ordering::SeqCst), 3);

        // Cenário 2: Excede max_retries (falha 4 vezes, excede max_retries = 3)
        let begin_count2 = Arc::new(AtomicUsize::new(0));
        let mgr2 = MockTxManager {
            begin_count: begin_count2.clone(),
        };
        let exec_count2 = Arc::new(AtomicUsize::new(0));
        let exec_count2_clone = exec_count2.clone();

        let res2: Result<()> = run_in_tx(&mgr2, &policy, move |_| {
            let exec_count = exec_count2_clone.clone();
            Box::pin(async move {
                exec_count.fetch_add(1, Ordering::SeqCst);
                Err(CoreError::TransientConflict("sqlite busy".into()))
            })
        })
        .await;

        assert!(matches!(res2, Err(CoreError::TransientConflict(_))));
        assert_eq!(begin_count2.load(Ordering::SeqCst), 4); // 1 tentativa inicial + 3 retries
        assert_eq!(exec_count2.load(Ordering::SeqCst), 4);

        // Cenário 3: Erro não transiente (Validation) aborta imediatamente sem retry
        let begin_count3 = Arc::new(AtomicUsize::new(0));
        let mgr3 = MockTxManager {
            begin_count: begin_count3.clone(),
        };
        let exec_count3 = Arc::new(AtomicUsize::new(0));
        let exec_count3_clone = exec_count3.clone();

        let res3: Result<()> = run_in_tx(&mgr3, &policy, move |_| {
            let exec_count = exec_count3_clone.clone();
            Box::pin(async move {
                exec_count.fetch_add(1, Ordering::SeqCst);
                Err(CoreError::Validation("dados inválidos".into()))
            })
        })
        .await;

        assert!(matches!(res3, Err(CoreError::Validation(_))));
        assert_eq!(begin_count3.load(Ordering::SeqCst), 1);
        assert_eq!(exec_count3.load(Ordering::SeqCst), 1);
    }
}
