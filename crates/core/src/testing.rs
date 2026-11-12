use crate::domain::{
    CreateAddressInput, CreateContactInput, CreateCustomerInput, CreateUserInput, CustomerFilter,
    DocumentType, InitialSetupInput, InitialSetupResult, LoginInput, LoginPolicy, OperationMode,
    PersonType, RecoverPasswordInput, ResetPasswordInput, SessionInfo, UpdateBrandingInput,
    UpdateCustomerInput, UpdateUserInput, UserRecord, UserWithRole, ROLE_SLUG_COMMERCIAL,
    ROLE_SLUG_OWNER,
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
                logo_dark_url: None,
                icon_url: None,
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

    // 0.4: Hierarquia de gestão: Gerente com user:manage não pode agir sobre o Proprietário (permissões superiores)
    let update_owner_attempt = services
        .update_user(
            &limited_manager_session,
            &owner_session.user_id,
            &UpdateUserInput {
                name: "Proprietário Adulterado".to_string(),
                email: "novo.email@luzesom.com.br".to_string(),
                role_id: commercial_role.id.clone(),
            },
        )
        .await;
    assert!(
        matches!(update_owner_attempt, Err(CoreError::Forbidden(_))),
        "Gerente não pode editar usuário Proprietário com permissões superiores"
    );

    let deactivate_owner_attempt = services
        .set_user_active(&limited_manager_session, &owner_session.user_id, false)
        .await;
    assert!(
        matches!(deactivate_owner_attempt, Err(CoreError::Forbidden(_))),
        "Gerente não pode desativar usuário Proprietário com permissões superiores"
    );

    let reset_owner_password_attempt = services
        .reset_user_password(
            &limited_manager_session,
            &owner_session.user_id,
            &ResetPasswordInput {
                new_password: "SenhaInvasora#2026".to_string(),
            },
        )
        .await;
    assert!(
        matches!(reset_owner_password_attempt, Err(CoreError::Forbidden(_))),
        "Gerente não pode redefinir senha do Proprietário com permissões superiores"
    );

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

/// 11. Contrato: Branding suporta logo claro, logo escuro e ícone (migração 0003).
pub async fn contract_branding_dark_logo_and_icon(stores: &StoreBundle, tenant_id: &str) {
    let input = UpdateBrandingInput {
        company_name: "Locadora White Label".to_string(),
        primary_color: "#1e40af".to_string(),
        secondary_color: "#3b82f6".to_string(),
        accent_color: "#f59e0b".to_string(),
        logo_url: Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=".to_string()),
        logo_dark_url: Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=".to_string()),
        icon_url: Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=".to_string()),
    };

    let saved = stores
        .branding
        .save(tenant_id, &input)
        .await
        .expect("Falha ao salvar tenant_branding com logo escuro e ícone");

    assert_eq!(saved.company_name, "Locadora White Label");
    assert_eq!(saved.logo_url, input.logo_url);
    assert_eq!(saved.logo_dark_url, input.logo_dark_url);
    assert_eq!(saved.icon_url, input.icon_url);

    let retrieved = stores
        .branding
        .get_by_tenant_id(tenant_id)
        .await
        .expect("Falha ao buscar tenant_branding")
        .expect("Branding deve existir");

    assert_eq!(retrieved.logo_url, input.logo_url);
    assert_eq!(retrieved.logo_dark_url, input.logo_dark_url);
    assert_eq!(retrieved.icon_url, input.icon_url);
}

/// 12. Contrato: CRUD completo de clientes, contatos, endereços, unicidade ativa e auditoria LGPD mascarada.
pub async fn contract_customer_crud_and_lgpd_audit(
    services: &CoreServices,
    session: &SessionInfo,
    _tenant_id: &str,
) {
    // 1. Cria cliente PF com CPF válido
    let pf_input = CreateCustomerInput {
        person_type: PersonType::Individual,
        name: "Carlos Eduardo da Silva".to_string(),
        trade_name: None,
        document_type: DocumentType::Cpf,
        document_number: Some("529.982.247-25".to_string()),
        state_registration: None,
        municipal_registration: None,
        notes: Some("Cliente VIP de eventos corporativos".to_string()),
        contacts: vec![CreateContactInput {
            id: None,
            name: "Carlos Silva".to_string(),
            email: Some("carlos@silvaeventos.com.br".to_string()),
            phone: Some("(11) 99887-7665".to_string()),
            role: Some("Titular".to_string()),
            is_primary: true,
        }],
        addresses: vec![CreateAddressInput {
            id: None,
            label: "Sede".to_string(),
            street: "Av. Paulista".to_string(),
            number: "1000".to_string(),
            complement: Some("Conjunto 42".to_string()),
            neighborhood: "Bela Vista".to_string(),
            city: "São Paulo".to_string(),
            state: "SP".to_string(),
            postal_code: "01310-100".to_string(),
            country: Some("BR".to_string()),
            is_primary: true,
        }],
    };

    let created_pf = services
        .create_customer(session, &pf_input)
        .await
        .expect("Falha ao criar cliente PF com CPF válido");
    assert_eq!(created_pf.customer.name, "Carlos Eduardo da Silva");
    assert_eq!(created_pf.customer.document_type, DocumentType::Cpf);
    assert_eq!(
        created_pf.customer.document_number.as_deref(),
        Some("52998224725")
    );
    assert!(created_pf.customer.active);
    assert_eq!(created_pf.contacts.len(), 1);
    assert_eq!(created_pf.addresses.len(), 1);

    // 2. Cria cliente PJ com CNPJ alfanumérico válido (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024)
    let pj_input = CreateCustomerInput {
        person_type: PersonType::Legal,
        name: "Prime Audiovisual & Iluminação Ltda.".to_string(),
        trade_name: Some("Prime Eventos".to_string()),
        document_type: DocumentType::Cnpj,
        document_number: Some("12.ABC.345/01DE-35".to_string()),
        state_registration: Some("123.456.789.000".to_string()),
        municipal_registration: Some("987654".to_string()),
        notes: None,
        contacts: vec![CreateContactInput {
            id: None,
            name: "Diretoria Financeira".to_string(),
            email: Some("financeiro@primeeventos.com".to_string()),
            phone: Some("(11) 3322-1100".to_string()),
            role: Some("Financeiro".to_string()),
            is_primary: true,
        }],
        addresses: vec![CreateAddressInput {
            id: None,
            label: "Galpão Principal".to_string(),
            street: "Rua dos Produtores".to_string(),
            number: "500".to_string(),
            complement: None,
            neighborhood: "Vila Leopoldina".to_string(),
            city: "São Paulo".to_string(),
            state: "SP".to_string(),
            postal_code: "05311-000".to_string(),
            country: None,
            is_primary: true,
        }],
    };

    let created_pj = services
        .create_customer(session, &pj_input)
        .await
        .expect("Falha ao criar cliente PJ com CNPJ alfanumérico");
    assert_eq!(
        created_pj.customer.document_number.as_deref(),
        Some("12ABC34501DE35")
    );
    assert_eq!(created_pj.addresses[0].country, "BR");

    // 2.1 Cria cliente sem documento (pré-cadastro: document_type = none) e cliente estrangeiro (foreign)
    let mut no_doc_input = pf_input.clone();
    no_doc_input.name = "Pré Cadastro Evento".to_string();
    no_doc_input.document_type = DocumentType::None;
    no_doc_input.document_number = None;
    let created_no_doc = services
        .create_customer(session, &no_doc_input)
        .await
        .expect("Pré-cadastro sem documento deve ser permitido");
    assert_eq!(created_no_doc.customer.document_type, DocumentType::None);
    assert!(created_no_doc.customer.document_number.is_none());

    let mut foreign_input = pf_input.clone();
    foreign_input.name = "John Doe International".to_string();
    foreign_input.document_type = DocumentType::Foreign;
    foreign_input.document_number = Some("us-pass-998877".to_string());
    let created_foreign = services
        .create_customer(session, &foreign_input)
        .await
        .expect("Cliente estrangeiro com passaporte alfanumérico deve ser aceito");
    assert_eq!(
        created_foreign.customer.document_type,
        DocumentType::Foreign
    );
    assert_eq!(
        created_foreign.customer.document_number.as_deref(),
        Some("USPASS998877")
    );

    // 3. Unicidade ativa: tentativa de criar cliente duplicado com mesmo CPF deve falhar com CoreError::Conflict
    let dup_pf = services.create_customer(session, &pf_input).await;
    assert!(dup_pf.is_err());
    assert!(matches!(dup_pf.unwrap_err(), CoreError::Conflict(_)));

    // 4. Incompatibilidade: PF com CNPJ e PJ com CPF
    let mut invalid_pair1 = pf_input.clone();
    invalid_pair1.document_type = DocumentType::Cnpj;
    invalid_pair1.document_number = Some("12.ABC.345/01DE-35".to_string());
    assert!(services
        .create_customer(session, &invalid_pair1)
        .await
        .is_err());

    let mut invalid_pair2 = pj_input.clone();
    invalid_pair2.document_type = DocumentType::Cpf;
    invalid_pair2.document_number = Some("529.982.247-25".to_string());
    assert!(services
        .create_customer(session, &invalid_pair2)
        .await
        .is_err());

    // 5. Busca e paginação (list_customers)
    // 5.1 Busca sem acento e case-insensitive
    let list_search = services
        .list_customers(
            session,
            &CustomerFilter {
                query: Some("eduardo silva".to_string()),
                person_type: None,
                cursor: None,
                limit: Some(10),
            },
        )
        .await
        .expect("Busca por 'eduardo silva' deve suceder");
    assert_eq!(list_search.items.len(), 1);
    assert_eq!(list_search.items[0].id, created_pf.customer.id);

    // 5.2 Busca por CNPJ alfanumérico com máscara
    let list_cnpj = services
        .list_customers(
            session,
            &CustomerFilter {
                query: Some("12.ABC.345".to_string()),
                person_type: None,
                cursor: None,
                limit: Some(10),
            },
        )
        .await
        .expect("Busca por CNPJ parcial");
    assert_eq!(list_cnpj.items.len(), 1);
    assert_eq!(list_cnpj.items[0].id, created_pj.customer.id);

    // 5.3 Cursor inválido ou adulterado deve retornar erro de validação limpo
    let bad_cursor_res = services
        .list_customers(
            session,
            &CustomerFilter {
                query: None,
                person_type: None,
                cursor: Some("cursor-adulterado-!@#".to_string()),
                limit: None,
            },
        )
        .await;
    assert!(matches!(bad_cursor_res, Err(CoreError::Validation(_))));

    // 6. Atualização de cliente preservando IDs estáveis de contato e endereço
    let original_contact_id = created_pf.contacts[0].id.clone();
    let original_address_id = created_pf.addresses[0].id.clone();

    let mut updated_contacts: Vec<CreateContactInput> = created_pf
        .contacts
        .into_iter()
        .map(|c| CreateContactInput {
            id: Some(c.id),
            name: "Carlos Silva Atualizado".to_string(),
            email: c.email,
            phone: c.phone,
            role: c.role,
            is_primary: false,
        })
        .collect();
    updated_contacts.push(CreateContactInput {
        id: None,
        name: "Segundo Contato".to_string(),
        email: Some("segundo@silvaeventos.com.br".to_string()),
        phone: None,
        role: Some("Produção".to_string()),
        is_primary: true,
    });

    let update_input = UpdateCustomerInput {
        person_type: PersonType::Individual,
        name: "Carlos Eduardo da Silva Atualizado".to_string(),
        trade_name: Some("Carlos VIP".to_string()),
        document_type: DocumentType::Cpf,
        document_number: Some("529.982.247-25".to_string()),
        state_registration: None,
        municipal_registration: None,
        notes: Some("Notas atualizadas".to_string()),
        contacts: updated_contacts,
        addresses: created_pf
            .addresses
            .into_iter()
            .map(|a| CreateAddressInput {
                id: Some(a.id),
                label: a.label,
                street: a.street,
                number: a.number,
                complement: a.complement,
                neighborhood: a.neighborhood,
                city: a.city,
                state: a.state,
                postal_code: a.postal_code,
                country: Some(a.country),
                is_primary: a.is_primary,
            })
            .collect(),
    };

    let updated = services
        .update_customer(session, &created_pf.customer.id, &update_input)
        .await
        .expect("Falha ao atualizar cliente");
    assert_eq!(updated.customer.name, "Carlos Eduardo da Silva Atualizado");
    assert!(
        updated
            .contacts
            .iter()
            .any(|c| c.id == original_contact_id && c.name == "Carlos Silva Atualizado"),
        "ID do contato existente deve ser preservado após atualização"
    );
    assert_eq!(updated.contacts.len(), 2);
    assert_eq!(updated.addresses[0].id, original_address_id);

    // 6.1 Ativação e desativação (set_customer_active)
    let deactivated = services
        .set_customer_active(session, &created_pf.customer.id, false)
        .await
        .expect("Falha ao desativar cliente");
    assert!(!deactivated.customer.active);
    let reactivated = services
        .set_customer_active(session, &created_pf.customer.id, true)
        .await
        .expect("Falha ao reativar cliente");
    assert!(reactivated.customer.active);

    // 7. Soft delete e reativação de documento
    services
        .delete_customer(session, &created_pf.customer.id)
        .await
        .expect("Falha ao excluir cliente");

    // Cliente excluído não deve aparecer na busca ativa
    let after_delete = services
        .get_customer(session, &created_pf.customer.id)
        .await;
    assert!(after_delete.is_err());
    assert!(matches!(after_delete.unwrap_err(), CoreError::NotFound(_)));

    // Índice único parcial permite re-cadastrar o mesmo CPF após exclusão lógica
    let recreate_res = services.create_customer(session, &pf_input).await;
    assert!(
        recreate_res.is_ok(),
        "Deve ser possível cadastrar cliente com mesmo documento após exclusão lógica do anterior"
    );

    // 8. Proteção LGPD na Auditoria: inspeciona logs e garante mascaramento
    let audits = services
        .stores
        .audit
        .list_by_tenant(&session.tenant_id, 20)
        .await
        .unwrap();
    let customer_audits: Vec<_> = audits
        .iter()
        .filter(|a| a.action.starts_with("customer."))
        .collect();
    assert!(!customer_audits.is_empty());

    for entry in &customer_audits {
        // NUNCA deve conter o CPF completo, CNPJ completo, e-mail completo ou telefone completo
        assert!(
            !entry.summary.contains("52998224725") && !entry.summary.contains("529.982.247-25"),
            "Violação LGPD: CPF cru no log de auditoria!"
        );
        assert!(
            !entry.summary.contains("12ABC34501DE35")
                && !entry.summary.contains("12.ABC.345/01DE-35"),
            "Violação LGPD: CNPJ cru no log de auditoria!"
        );
        assert!(
            !entry.summary.contains("carlos@silvaeventos.com.br")
                && !entry.summary.contains("financeiro@primeeventos.com"),
            "Violação LGPD: e-mail completo no log de auditoria!"
        );
        assert!(
            !entry.summary.contains("99887-7665") && !entry.summary.contains("3322-1100"),
            "Violação LGPD: telefone completo no log de auditoria!"
        );
        println!("[AUDIT SUMMARY] {} => {}", entry.action, entry.summary);
    }

    // 9. Prova LGPD em {:?} (Debug manual)
    let debug_details = format!("{deactivated:?}");
    let debug_filter = format!(
        "{:?}",
        CustomerFilter {
            query: Some("529.982.247-25".to_string()),
            person_type: Some(PersonType::Individual),
            cursor: Some("abc".to_string()),
            limit: Some(50),
        }
    );
    let debug_create = format!("{pf_input:?}");
    let debug_update = format!("{update_input:?}");
    for dbg in [&debug_details, &debug_filter, &debug_create, &debug_update] {
        assert!(!dbg.contains("52998224725") && !dbg.contains("529.982.247-25"));
        assert!(!dbg.contains("carlos@silvaeventos.com.br"));
        assert!(!dbg.contains("99887-7665"));
        assert!(!dbg.contains("Av. Paulista"));
        assert!(!dbg.contains("01310-100"));
    }
    assert!(debug_details.contains("***.***.***-25"));
    assert!(debug_details.contains("c***@silvaeventos.com.br"));
    assert!(debug_details.contains("(11) *****-7665"));
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
    contract_branding_dark_logo_and_icon(&stores, &tenant_id).await;
    contract_customer_crud_and_lgpd_audit(&services, &setup_result.session, &tenant_id).await;
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
                logo_dark_url: None,
                icon_url: None,
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
        // IDN / Punycode TLD
        assert!(validate_email("usuario@dominio.xn--p1ai").is_ok());
        // Limite máximo exato de 64 caracteres na parte local
        let local_64 = format!("{}@dominio.com", "a".repeat(64));
        assert!(validate_email(&local_64).is_ok());

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
        // Espaços em branco e caracteres de controle
        assert!(validate_email("usuario @dominio.com").is_err());
        assert!(validate_email("usuario@ dominio.com").is_err());
        assert!(validate_email(" usuario@dominio.com ").is_err());
        assert!(validate_email("usuario\n@dominio.com").is_err());
        // Parte local excedendo 64 caracteres
        let local_65 = format!("{}@dominio.com", "a".repeat(65));
        assert!(validate_email(&local_65).is_err());
        // TLD inválido terminando em hífen
        assert!(validate_email("usuario@dominio.com-").is_err());
        // Rótulos de domínio inválidos e caracteres proibidos (ex: exclamação)
        assert!(validate_email("usuario@exa!mple.com").is_err());
        assert!(validate_email("usuario@sub.-empresa.com").is_err());
        assert!(validate_email("usuario@empresa-.com").is_err());
        // Parte local com caracteres proibidos
        assert!(validate_email("usuá[rio]@dominio.com").is_err());
    }

    #[test]
    fn test_customer_domain_validations_and_limits() {
        use crate::domain::customer::*;

        // 1. CEP: BR normaliza para 8 dígitos, rejeita formato inválido
        assert_eq!(
            normalize_and_validate_postal_code("01310-100", "BR").unwrap(),
            "01310100"
        );
        assert_eq!(
            normalize_and_validate_postal_code("01310100", "br").unwrap(),
            "01310100"
        );
        assert_eq!(
            normalize_and_validate_postal_code(" 01310.100 ", "BR").unwrap(),
            "01310100"
        );
        assert!(normalize_and_validate_postal_code("01310-10", "BR").is_err());
        assert!(normalize_and_validate_postal_code("01310-1000", "BR").is_err());
        assert!(normalize_and_validate_postal_code("01310-abc", "BR").is_err());
        // Não-BR aceita códigos postais internacionais até 20 caracteres
        assert_eq!(
            normalize_and_validate_postal_code("90210", "US").unwrap(),
            "90210"
        );
        assert_eq!(
            normalize_and_validate_postal_code("SW1A 1AA", "GB").unwrap(),
            "SW1A 1AA"
        );
        assert!(normalize_and_validate_postal_code(&"A".repeat(21), "US").is_err());

        // 2. UF: 27 siglas válidas e rejeição de inválidas
        assert_eq!(validate_brazilian_state("SP").unwrap(), "SP");
        assert_eq!(validate_brazilian_state("rj").unwrap(), "RJ");
        assert_eq!(validate_brazilian_state(" mg ").unwrap(), "MG");
        assert!(validate_brazilian_state("XX").is_err());
        assert!(validate_brazilian_state("Sao Paulo").is_err());

        // 3. País: 2 letras ISO
        assert_eq!(validate_country_code("BR").unwrap(), "BR");
        assert_eq!(validate_country_code("us").unwrap(), "US");
        assert!(validate_country_code("BRA").is_err());
        assert!(validate_country_code("B1").is_err());

        // 4. Telefone: BR 10 ou 11 dígitos, remove +55 ou 55; Outros 8-15
        assert_eq!(
            normalize_and_validate_phone("(11) 98765-4321", Some("BR")).unwrap(),
            "11987654321"
        );
        assert_eq!(
            normalize_and_validate_phone("+55 11 98765-4321", Some("BR")).unwrap(),
            "11987654321"
        );
        assert_eq!(
            normalize_and_validate_phone("5511987654321", Some("BR")).unwrap(),
            "11987654321"
        );
        assert_eq!(
            normalize_and_validate_phone("(11) 2345-6789", Some("BR")).unwrap(),
            "1123456789"
        );
        assert!(normalize_and_validate_phone("119876", Some("BR")).is_err());
        assert!(normalize_and_validate_phone("1198765432100", Some("BR")).is_err());
        assert!(normalize_and_validate_phone("1198765abc", Some("BR")).is_err());
        // Internacional
        assert_eq!(
            normalize_and_validate_phone("+1 202 555 0123", Some("US")).unwrap(),
            "12025550123"
        );
        assert!(normalize_and_validate_phone("12345", Some("US")).is_err());

        // 5. Contatos: telefone OU e-mail, exatamente 1 principal, IDs repetidos
        let mut contacts = vec![
            CreateContactInput {
                id: Some("c1".to_string()),
                name: "Contato 1".to_string(),
                email: Some("c1@ex.com".to_string()),
                phone: None,
                role: Some("Gerente".to_string()),
                is_primary: false,
            },
            CreateContactInput {
                id: Some("c2".to_string()),
                name: "Contato 2".to_string(),
                email: None,
                phone: Some("(11) 98765-4321".to_string()),
                role: None,
                is_primary: false,
            },
        ];
        assert!(validate_contacts(&mut contacts).is_ok());
        // Primeiro virou principal automaticamente
        assert!(contacts[0].is_primary);
        assert!(!contacts[1].is_primary);
        assert_eq!(contacts[1].phone.as_deref(), Some("11987654321"));

        // Dois principais = erro
        let mut two_primaries = contacts.clone();
        two_primaries[1].is_primary = true;
        assert!(validate_contacts(&mut two_primaries).is_err());

        // Sem telefone e sem email = erro
        let mut no_contact_info = vec![CreateContactInput {
            id: None,
            name: "Sem Nada".to_string(),
            email: None,
            phone: None,
            role: None,
            is_primary: true,
        }];
        assert!(validate_contacts(&mut no_contact_info).is_err());

        // IDs repetidos na entrada = erro
        let mut dup_ids = vec![
            CreateContactInput {
                id: Some("rep".to_string()),
                name: "A".to_string(),
                email: Some("a@ex.com".to_string()),
                phone: None,
                role: None,
                is_primary: true,
            },
            CreateContactInput {
                id: Some("rep".to_string()),
                name: "B".to_string(),
                email: Some("b@ex.com".to_string()),
                phone: None,
                role: None,
                is_primary: false,
            },
        ];
        assert!(validate_contacts(&mut dup_ids).is_err());

        // Limite de 20 contatos
        let mut over_20 = (0..21)
            .map(|i| CreateContactInput {
                id: Some(format!("c{i}")),
                name: format!("Nome {i}"),
                email: Some(format!("c{i}@ex.com")),
                phone: None,
                role: None,
                is_primary: i == 0,
            })
            .collect::<Vec<_>>();
        assert!(validate_contacts(&mut over_20).is_err());

        // 6. Endereços: limites e principal
        let mut addresses = vec![CreateAddressInput {
            id: Some("a1".to_string()),
            label: "Sede".to_string(),
            street: "Av Paulista".to_string(),
            number: "1000".to_string(),
            complement: Some("Andar 10".to_string()),
            neighborhood: "Bela Vista".to_string(),
            city: "São Paulo".to_string(),
            state: "sp".to_string(),
            postal_code: "01310-100".to_string(),
            country: Some("BR".to_string()),
            is_primary: false,
        }];
        assert!(validate_addresses(&mut addresses).is_ok());
        assert!(addresses[0].is_primary);
        assert_eq!(addresses[0].state, "SP");
        assert_eq!(addresses[0].postal_code, "01310100");

        // 7. Limites gerais
        assert!(validate_customer_common_fields("Cliente OK", None, None, None, None).is_ok());
        assert!(validate_customer_common_fields(&"A".repeat(151), None, None, None, None).is_err());
        assert!(
            validate_customer_common_fields("OK", Some(&"T".repeat(151)), None, None, None)
                .is_err()
        );
        assert!(
            validate_customer_common_fields("OK", None, Some(&"N".repeat(2001)), None, None)
                .is_err()
        );
        assert!(
            validate_customer_common_fields("OK", None, None, Some(&"S".repeat(31)), None).is_err()
        );
        assert!(
            validate_customer_common_fields("OK", None, None, None, Some(&"M".repeat(31))).is_err()
        );
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
            run_in_tx, AuditStore, BrandingStore, CustomerStore, RoleStore, TenantStore,
            TransactionManager, TransactionPolicy, TxStoreBundle, UserStore,
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
            fn customers(&self) -> &dyn CustomerStore {
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

    #[tokio::test]
    async fn test_run_in_tx_retry_on_begin_and_commit() {
        use crate::error::{CoreError, Result};
        use crate::store::{
            run_in_tx, AuditStore, BrandingStore, CustomerStore, RoleStore, TenantStore,
            TransactionManager, TransactionPolicy, TxStoreBundle, UserStore,
        };
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        struct FailingCommitTxBundle {
            commit_fails_remaining: Arc<AtomicUsize>,
            commit_attempts: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl TxStoreBundle for FailingCommitTxBundle {
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
            fn customers(&self) -> &dyn CustomerStore {
                unreachable!()
            }
            async fn commit(self: Box<Self>) -> Result<()> {
                self.commit_attempts.fetch_add(1, Ordering::SeqCst);
                let remaining = self.commit_fails_remaining.load(Ordering::SeqCst);
                if remaining > 0 {
                    self.commit_fails_remaining.fetch_sub(1, Ordering::SeqCst);
                    Err(CoreError::TransientConflict("Commit busy/locked".into()))
                } else {
                    Ok(())
                }
            }
            async fn rollback(self: Box<Self>) -> Result<()> {
                Ok(())
            }
        }

        struct FlakyTxManager {
            begin_fails_remaining: Arc<AtomicUsize>,
            begin_attempts: Arc<AtomicUsize>,
            commit_fails_remaining: Arc<AtomicUsize>,
            commit_attempts: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl TransactionManager for FlakyTxManager {
            async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>> {
                self.begin_attempts.fetch_add(1, Ordering::SeqCst);
                let remaining = self.begin_fails_remaining.load(Ordering::SeqCst);
                if remaining > 0 {
                    self.begin_fails_remaining.fetch_sub(1, Ordering::SeqCst);
                    Err(CoreError::TransientConflict("BEGIN IMMEDIATE busy".into()))
                } else {
                    Ok(Box::new(FailingCommitTxBundle {
                        commit_fails_remaining: self.commit_fails_remaining.clone(),
                        commit_attempts: self.commit_attempts.clone(),
                    }))
                }
            }
        }

        let policy = TransactionPolicy {
            max_retries: 3,
            initial_backoff_ms: 1,
        };

        // Cenário 1: Falha transitória no BEGIN (2 falhas de BEGIN, sucesso no 3º)
        let begin_fails = Arc::new(AtomicUsize::new(2));
        let begin_attempts = Arc::new(AtomicUsize::new(0));
        let commit_fails = Arc::new(AtomicUsize::new(0));
        let commit_attempts = Arc::new(AtomicUsize::new(0));
        let mgr1 = FlakyTxManager {
            begin_fails_remaining: begin_fails,
            begin_attempts: begin_attempts.clone(),
            commit_fails_remaining: commit_fails,
            commit_attempts: commit_attempts.clone(),
        };

        let exec_count = Arc::new(AtomicUsize::new(0));
        let exec_count_clone = exec_count.clone();
        let res1: Result<&str> = run_in_tx(&mgr1, &policy, move |_| {
            let exec_count = exec_count_clone.clone();
            Box::pin(async move {
                exec_count.fetch_add(1, Ordering::SeqCst);
                Ok("ok_apos_begin_retry")
            })
        })
        .await;

        assert_eq!(res1.unwrap(), "ok_apos_begin_retry");
        assert_eq!(begin_attempts.load(Ordering::SeqCst), 3);
        assert_eq!(exec_count.load(Ordering::SeqCst), 1);
        assert_eq!(commit_attempts.load(Ordering::SeqCst), 1);

        // Cenário 2: Falha transitória no COMMIT (2 falhas de COMMIT, sucesso no 3º)
        let begin_fails2 = Arc::new(AtomicUsize::new(0));
        let begin_attempts2 = Arc::new(AtomicUsize::new(0));
        let commit_fails2 = Arc::new(AtomicUsize::new(2));
        let commit_attempts2 = Arc::new(AtomicUsize::new(0));
        let mgr2 = FlakyTxManager {
            begin_fails_remaining: begin_fails2,
            begin_attempts: begin_attempts2.clone(),
            commit_fails_remaining: commit_fails2,
            commit_attempts: commit_attempts2.clone(),
        };

        let exec_count2 = Arc::new(AtomicUsize::new(0));
        let exec_count2_clone = exec_count2.clone();
        let res2: Result<&str> = run_in_tx(&mgr2, &policy, move |_| {
            let exec_count = exec_count2_clone.clone();
            Box::pin(async move {
                exec_count.fetch_add(1, Ordering::SeqCst);
                Ok("ok_apos_commit_retry")
            })
        })
        .await;

        assert_eq!(res2.unwrap(), "ok_apos_commit_retry");
        assert_eq!(begin_attempts2.load(Ordering::SeqCst), 3);
        assert_eq!(exec_count2.load(Ordering::SeqCst), 3);
        assert_eq!(commit_attempts2.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_branding_validator_company_name() {
        use crate::domain::BrandingValidator;

        assert_eq!(
            BrandingValidator::validate_company_name("  Locadora Luz & Som  ").unwrap(),
            "Locadora Luz & Som"
        );
        assert!(BrandingValidator::validate_company_name("").is_err());
        assert!(BrandingValidator::validate_company_name("   ").is_err());

        let exactly_80 = "A".repeat(80);
        assert_eq!(
            BrandingValidator::validate_company_name(&exactly_80).unwrap(),
            exactly_80
        );

        let over_80 = "A".repeat(81);
        assert!(BrandingValidator::validate_company_name(&over_80).is_err());

        assert!(BrandingValidator::validate_company_name("Locadora\nLtda").is_err());
        assert!(BrandingValidator::validate_company_name("Locadora\0Ltda").is_err());
    }

    #[test]
    fn test_branding_validator_hex_color() {
        use crate::domain::BrandingValidator;

        assert_eq!(
            BrandingValidator::validate_hex_color("primária", "#2563EB").unwrap(),
            "#2563eb"
        );
        assert_eq!(
            BrandingValidator::validate_hex_color("secundária", "#000000").unwrap(),
            "#000000"
        );

        // Rejeições de formato
        assert!(BrandingValidator::validate_hex_color("cor", "2563eb").is_err());
        assert!(BrandingValidator::validate_hex_color("cor", "#fff").is_err());
        assert!(BrandingValidator::validate_hex_color("cor", "#12345678").is_err());
        assert!(BrandingValidator::validate_hex_color("cor", "#gggggg").is_err());

        // Rejeições de CSS injection
        assert!(BrandingValidator::validate_hex_color("cor", "red; background: blue").is_err());
        assert!(BrandingValidator::validate_hex_color("cor", "#2563eb; color: red").is_err());
        assert!(BrandingValidator::validate_hex_color("cor", "rgb(0,0,0)").is_err());
    }

    #[test]
    fn test_branding_validator_images_magic_bytes_and_formats() {
        use crate::domain::{BrandingValidator, UpdateBrandingInput};
        use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
        use base64::Engine as _;

        // 1. Rejeição estrita a formato não suportado (ex: SVG)
        let svg_input = UpdateBrandingInput {
            company_name: "Locadora".to_string(),
            primary_color: "#2563eb".to_string(),
            secondary_color: "#475569".to_string(),
            accent_color: "#7c3aed".to_string(),
            logo_url: Some("data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=".to_string()),
            logo_dark_url: None,
            icon_url: None,
        };
        let svg_err = BrandingValidator::validate_input(&svg_input);
        assert!(svg_err.is_err());
        assert!(svg_err.unwrap_err().to_string().contains("prefixo exato"));

        // 2. PNG válido
        let valid_png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY44YAAAAASUVORK5CYII=";
        let valid_png_uri = format!("data:image/png;base64,{valid_png_b64}");
        let png_res = BrandingValidator::validate_image_data_uri("Logo", &valid_png_uri, false);
        assert!(png_res.is_ok());

        // 3. PNG com assinatura mágica inválida
        let invalid_png_uri = "data:image/png;base64,QUJDREVGR0hJSktMTU5PUA=="; // "ABCDEFGHIJKLMNOP"
        let invalid_png_res =
            BrandingValidator::validate_image_data_uri("Logo", invalid_png_uri, false);
        assert!(invalid_png_res.is_err());
        assert!(invalid_png_res.unwrap_err().to_string().contains("PNG"));

        // 4. JPEG válido com cabeçalho completo lido por imagesize
        let valid_jpeg_b64 = "/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAP//////////////////////////////////////////////////////////////////////////////////////wgALCAABAAEBAREA/8QAFBABAAAAAAAAAAAAAAAAAAAAAP/aAAgBAQABPxA=";
        let valid_jpeg_uri = format!("data:image/jpeg;base64,{valid_jpeg_b64}");
        let jpeg_res = BrandingValidator::validate_image_data_uri("Logo", &valid_jpeg_uri, false);
        assert!(jpeg_res.is_ok());

        // 5. WebP válido com cabeçalho completo lido por imagesize
        let valid_webp_b64 = "UklGRjoAAABXRUJQVlA4IC4AAACyAgCdASoCAAIALmk0mk0iIiIiIgBoSygABc6WWgAA/veff/0PP8bA//LwYAAA";
        let valid_webp_uri = format!("data:image/webp;base64,{valid_webp_b64}");
        let webp_res = BrandingValidator::validate_image_data_uri("Logo", &valid_webp_uri, false);
        assert!(webp_res.is_ok());

        // 5.1. Dimensão excessiva: PNG forjado com cabeçalho 30000x30000 pixels (deve falhar)
        let mut forged_png = Vec::new();
        forged_png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]); // Magic
        forged_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]); // IHDR length = 13
        forged_png.extend_from_slice(b"IHDR");
        forged_png.extend_from_slice(&[0x00, 0x00, 0x75, 0x30]); // width = 30000
        forged_png.extend_from_slice(&[0x00, 0x00, 0x75, 0x30]); // height = 30000
        forged_png.extend_from_slice(&[0x08, 0x06, 0x00, 0x00, 0x00]); // 8-bit RGBA
        forged_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // CRC
        let forged_png_b64 = BASE64_STANDARD.encode(&forged_png);
        let forged_png_uri = format!("data:image/png;base64,{forged_png_b64}");
        let forged_res = BrandingValidator::validate_image_data_uri("Logo", &forged_png_uri, false);
        assert!(forged_res.is_err());
        assert!(forged_res
            .unwrap_err()
            .to_string()
            .contains("30000x30000 pixels"));

        // 5.2. Dimensão nula (0x0) (deve falhar)
        let mut zero_png = Vec::new();
        zero_png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        zero_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
        zero_png.extend_from_slice(b"IHDR");
        zero_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // width = 0
        zero_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // height = 0
        zero_png.extend_from_slice(&[0x08, 0x06, 0x00, 0x00, 0x00]);
        zero_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        let zero_png_b64 = BASE64_STANDARD.encode(&zero_png);
        let zero_png_uri = format!("data:image/png;base64,{zero_png_b64}");
        let zero_res = BrandingValidator::validate_image_data_uri("Logo", &zero_png_uri, false);
        assert!(zero_res.is_err());
        assert!(zero_res.unwrap_err().to_string().contains("nula"));

        // 5.3. Ícone excedendo 512x512 pixels (ex: 600x600 pixels) (deve falhar para ícone)
        let mut icon_large = Vec::new();
        icon_large.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        icon_large.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
        icon_large.extend_from_slice(b"IHDR");
        icon_large.extend_from_slice(&[0x00, 0x00, 0x02, 0x58]); // width = 600
        icon_large.extend_from_slice(&[0x00, 0x00, 0x02, 0x58]); // height = 600
        icon_large.extend_from_slice(&[0x08, 0x06, 0x00, 0x00, 0x00]);
        icon_large.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        let icon_large_b64 = BASE64_STANDARD.encode(&icon_large);
        let icon_large_uri = format!("data:image/png;base64,{icon_large_b64}");
        let icon_res = BrandingValidator::validate_image_data_uri("Ícone", &icon_large_uri, true);
        assert!(icon_res.is_err());
        assert!(icon_res.unwrap_err().to_string().contains("512x512 pixels"));

        // 6. Formato não suportado (ex: GIF)
        let gif_uri =
            "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
        let gif_res = BrandingValidator::validate_image_data_uri("Logo", gif_uri, false);
        assert!(gif_res.is_err());

        // 7. Alfabeto inválido
        let corrupt_b64_uri = "data:image/png;base64,***NaoEB64***";
        assert!(
            BrandingValidator::validate_image_data_uri("Logo", corrupt_b64_uri, false).is_err()
        );

        // 8. Padding errado
        let invalid_padding_uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAE===";
        assert!(
            BrandingValidator::validate_image_data_uri("Logo", invalid_padding_uri, false).is_err()
        );

        // 9. Espaço ou quebra de linha
        let whitespace_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEA\nAAABAQMAAAAl21bKAAAA";
        let whitespace_uri = format!("data:image/png;base64,{whitespace_b64}");
        assert!(
            BrandingValidator::validate_image_data_uri("Logo", &whitespace_uri, false).is_err()
        );

        let leading_space_uri = format!("data:image/png;base64, {valid_png_b64}");
        assert!(
            BrandingValidator::validate_image_data_uri("Logo", &leading_space_uri, false).is_err()
        );

        // 10. MIME declarado diferente dos magic bytes (magic bytes de JPEG com prefixo de PNG)
        let mismatch_uri = "data:image/png;base64,/9j/4AAQSkZJRg==";
        let mismatch_res = BrandingValidator::validate_image_data_uri("Logo", mismatch_uri, false);
        assert!(mismatch_res.is_err());
        assert!(mismatch_res
            .unwrap_err()
            .to_string()
            .contains("PNG inválida"));

        // 11. Dados truncados (menos que o tamanho mínimo de magic bytes)
        let truncated_uri = "data:image/png;base64,iVBO";
        let truncated_res =
            BrandingValidator::validate_image_data_uri("Logo", truncated_uri, false);
        assert!(truncated_res.is_err());

        // 12. SVG disfarçado de PNG (inicia com <svg ou contém <svg)
        let svg_disguised_uri =
            "data:image/png;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjwvc3ZnPg==";
        let svg_disguised_res =
            BrandingValidator::validate_image_data_uri("Logo", svg_disguised_uri, false);
        assert!(svg_disguised_res.is_err());

        // 13. Prefixo com parâmetros adicionais
        let param_prefix_uri = format!("data:image/png;charset=utf-8;base64,{valid_png_b64}");
        let param_res =
            BrandingValidator::validate_image_data_uri("Logo", &param_prefix_uri, false);
        assert!(param_res.is_err());
        assert!(param_res.unwrap_err().to_string().contains("prefixo exato"));

        // 14. Limites de tamanho: exatamente no limite e 1 byte acima
        const VALID_1X1_PNG_HEADER: [u8; 33] = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // Magic
            0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D', b'R', // IHDR len 13
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 px
            0x08, 0x06, 0x00, 0x00, 0x00, // 8-bit RGBA
            0x1F, 0x15, 0xC4, 0x89, // CRC
        ];

        // 14.1. Ícone: exatamente 64 KB (65.536 bytes) -> PASSA
        let mut exact_icon_bytes = Vec::with_capacity(64 * 1024);
        exact_icon_bytes.extend_from_slice(&VALID_1X1_PNG_HEADER);
        exact_icon_bytes.resize(64 * 1024, 0x00);
        let exact_icon_b64 = BASE64_STANDARD.encode(&exact_icon_bytes);
        let exact_icon_uri = format!("data:image/png;base64,{exact_icon_b64}");
        assert!(BrandingValidator::validate_image_data_uri("Ícone", &exact_icon_uri, true).is_ok());

        // 14.2. Ícone: 1 byte acima de 64 KB (65.537 bytes) -> FALHA
        let mut over_icon_bytes = Vec::with_capacity(64 * 1024 + 1);
        over_icon_bytes.extend_from_slice(&VALID_1X1_PNG_HEADER);
        over_icon_bytes.resize(64 * 1024 + 1, 0x00);
        let over_icon_b64 = BASE64_STANDARD.encode(&over_icon_bytes);
        let over_icon_uri = format!("data:image/png;base64,{over_icon_b64}");
        let over_icon_res =
            BrandingValidator::validate_image_data_uri("Ícone", &over_icon_uri, true);
        assert!(over_icon_res.is_err());
        assert!(over_icon_res.unwrap_err().to_string().contains("64 KB"));

        // 14.3. Logo: exatamente 256 KB (262.144 bytes) -> PASSA
        let mut exact_logo_bytes = Vec::with_capacity(256 * 1024);
        exact_logo_bytes.extend_from_slice(&VALID_1X1_PNG_HEADER);
        exact_logo_bytes.resize(256 * 1024, 0x00);
        let exact_logo_b64 = BASE64_STANDARD.encode(&exact_logo_bytes);
        let exact_logo_uri = format!("data:image/png;base64,{exact_logo_b64}");
        assert!(BrandingValidator::validate_image_data_uri("Logo", &exact_logo_uri, false).is_ok());

        // 14.4. Logo: 1 byte acima de 256 KB (262.145 bytes) -> FALHA
        let mut over_logo_bytes = Vec::with_capacity(256 * 1024 + 1);
        over_logo_bytes.extend_from_slice(&VALID_1X1_PNG_HEADER);
        over_logo_bytes.resize(256 * 1024 + 1, 0x00);
        let over_logo_b64 = BASE64_STANDARD.encode(&over_logo_bytes);
        let over_logo_uri = format!("data:image/png;base64,{over_logo_b64}");
        let over_logo_res =
            BrandingValidator::validate_image_data_uri("Logo", &over_logo_uri, false);
        assert!(over_logo_res.is_err());
        assert!(over_logo_res.unwrap_err().to_string().contains("256 KB"));
    }

    #[test]
    fn test_document_validation_cpf_and_cnpj_alphanumeric() {
        use crate::domain::document::{
            format_cnpj, format_cpf, mask_document_for_audit, normalize_document, parse_document,
            testing_utils::{generate_valid_cnpj_from_base, generate_valid_cpf_from_base},
            validate_cnpj, validate_cpf, DocumentType,
        };

        // 1. Normalização de documentos (remove somente '.', '-', '/' e espaço; rejeita o resto)
        assert_eq!(
            normalize_document(" 12.ABC.345/01DE-35 ").expect("normaliza CNPJ"),
            "12ABC34501DE35"
        );
        assert_eq!(
            normalize_document("529.982.247-25").expect("normaliza CPF"),
            "52998224725"
        );
        assert_eq!(
            normalize_document("12abc34501de35").expect("normaliza minusculas"),
            "12ABC34501DE35"
        );
        assert!(normalize_document("12.ÁBC.345/01DE-35").is_err());
        assert!(normalize_document("12_ABC_345_01DE_35").is_err());
        assert!(normalize_document("529.982.247-25!").is_err());

        // 2. Validação de CPF
        // 2.1 CPFs válidos conhecidos
        assert!(validate_cpf("529.982.247-25").is_ok());
        assert!(validate_cpf("52998224725").is_ok());
        assert_eq!(format_cpf("52998224725"), "529.982.247-25");

        // 2.2 Rejeição de CPFs repetidos
        for digit in 0..=9 {
            let repeated = digit.to_string().repeat(11);
            let res = validate_cpf(&repeated);
            assert!(
                res.is_err(),
                "CPF repetido {repeated} deveria ter sido rejeitado"
            );
            assert!(res.unwrap_err().to_string().contains("repetidos"));
        }

        // 2.3 Rejeição de DVs incorretos
        let invalid_dv1 = "52998224735"; // DV1 esperado é 2, fornecido 3
        assert!(validate_cpf(invalid_dv1)
            .unwrap_err()
            .to_string()
            .contains("primeiro dígito verificador"));

        let invalid_dv2 = "52998224726"; // DV2 esperado é 5, fornecido 6
        assert!(validate_cpf(invalid_dv2)
            .unwrap_err()
            .to_string()
            .contains("segundo dígito verificador"));

        // 2.4 Tamanho e caracteres inválidos
        assert!(validate_cpf("1234567890").is_err()); // 10 dígitos
        assert!(validate_cpf("123456789012").is_err()); // 12 dígitos
        assert!(validate_cpf("5299822472A").is_err()); // contém letra

        // 3. Validação de CNPJ Tradicional (14 numéricos)
        // 3.1 Fixtures reais
        assert!(validate_cnpj("00.000.000/0001-91").is_ok()); // Banco do Brasil
        assert!(validate_cnpj("33.000.167/0001-01").is_ok()); // Petrobras
        assert_eq!(format_cnpj("00000000000191"), "00.000.000/0001-91");

        // 3.2 Rejeição de CNPJs repetidos
        for digit in 0..=9 {
            let repeated = digit.to_string().repeat(14);
            let res = validate_cnpj(&repeated);
            assert!(
                res.is_err(),
                "CNPJ repetido {repeated} deveria ter sido rejeitado"
            );
            assert!(res.unwrap_err().to_string().contains("idênticos"));
        }

        // 3.3 Rejeição de DVs numéricos incorretos
        assert!(validate_cnpj("00000000000181")
            .unwrap_err()
            .to_string()
            .contains("primeiro dígito verificador"));
        assert!(validate_cnpj("00000000000192")
            .unwrap_err()
            .to_string()
            .contains("segundo dígito verificador"));

        // 4. Validação de CNPJ Alfanumérico (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024)
        // Fonte oficial: Nota Técnica Conjunta Cocad/Suara/RFB nº 49/2024 e Simulador Oficial da Receita Federal
        // 4.1 Exemplo oficial da documentação RFB/ENCAT: 12.ABC.345/01DE-35
        let official_alpha_1 = "12.ABC.345/01DE-35";
        let valid_official_1 =
            validate_cnpj(official_alpha_1).expect("CNPJ alfanumérico oficial válido");
        assert_eq!(valid_official_1, "12ABC34501DE35");
        assert_eq!(format_cnpj(&valid_official_1), "12.ABC.345/01DE-35");

        // 4.2 Outros exemplos validados pela especificação ASCII - 48 (Simulador RFB)
        assert_eq!(
            validate_cnpj("12.ABC.345/0001-88").expect("exemplo RFB #2"),
            "12ABC345000188"
        );
        assert_eq!(
            validate_cnpj("AB.CDE.123/0001-00").expect("exemplo RFB #3"),
            "ABCDE123000100"
        );
        assert_eq!(
            validate_cnpj("A0.B1C.2D3/0001-79").expect("exemplo RFB #4"),
            "A0B1C2D3000179"
        );

        // 4.3 Minúsculas devem ser normalizadas para maiúsculas
        let valid_alpha_lower =
            validate_cnpj("12.abc.345/01de-35").expect("normaliza minusculas para maiusculas");
        assert_eq!(valid_alpha_lower, "12ABC34501DE35");

        // 4.4 DVs devem ser estritamente numéricos (letra nos dígitos verificadores rejeitada)
        assert!(validate_cnpj("12ABC34501DE3A")
            .unwrap_err()
            .to_string()
            .contains("dígitos verificadores) devem ser numéricos"));
        assert!(validate_cnpj("12ABC34501DEA5").is_err());

        // 4.5 Caracteres fora de A-Z, acentuados e tamanho errado rejeitados
        assert!(validate_cnpj("12.ÁBC.345/01DE-35").is_err());
        assert!(validate_cnpj("12.ABC.345/01DE-350").is_err()); // 15 caracteres
        assert!(validate_cnpj("12.ABC.345/01DE-3").is_err()); // 13 caracteres

        // 4.6 Máscara LGPD nos dois formatos (CPF e CNPJ numérico/alfanumérico)
        assert_eq!(mask_document_for_audit("529.982.247-25"), "***.***.***-25");
        assert_eq!(
            mask_document_for_audit("00.000.000/0001-91"),
            "**.***.***/0001-**"
        );
        assert_eq!(
            mask_document_for_audit("12.ABC.345/01DE-35"),
            "**.***.***/01DE-**"
        );

        // 4.4 DVs matematicamente incorretos em CNPJ alfanumérico
        let wrong_alpha_dv1 = "12ABC345000178"; // DV1 esperado 8, fornecido 7
        assert!(validate_cnpj(wrong_alpha_dv1)
            .unwrap_err()
            .to_string()
            .contains("primeiro dígito verificador"));

        let wrong_alpha_dv2 = "12ABC345000189"; // DV2 esperado 8, fornecido 9
        assert!(validate_cnpj(wrong_alpha_dv2)
            .unwrap_err()
            .to_string()
            .contains("segundo dígito verificador"));

        // 4.5 Caracteres especiais ilegais
        assert!(validate_cnpj("12ABC3450001$8").is_err());

        // 5. parse_document (Detecção automática de CPF vs CNPJ)
        let parsed_cpf = parse_document("529.982.247-25").expect("detecta CPF");
        assert_eq!(parsed_cpf.doc_type, DocumentType::Cpf);
        assert_eq!(parsed_cpf.raw, "52998224725");
        assert_eq!(parsed_cpf.formatted, "529.982.247-25");

        let parsed_cnpj_num = parse_document("00.000.000/0001-91").expect("detecta CNPJ numérico");
        assert_eq!(parsed_cnpj_num.doc_type, DocumentType::Cnpj);
        assert_eq!(parsed_cnpj_num.raw, "00000000000191");
        assert_eq!(parsed_cnpj_num.formatted, "00.000.000/0001-91");

        let parsed_cnpj_alpha =
            parse_document("12.ABC.345/0001-88").expect("detecta CNPJ alfanumérico");
        assert_eq!(parsed_cnpj_alpha.doc_type, DocumentType::Cnpj);
        assert_eq!(parsed_cnpj_alpha.raw, "12ABC345000188");
        assert_eq!(parsed_cnpj_alpha.formatted, "12.ABC.345/0001-88");

        // Documento com tamanho inválido
        assert!(parse_document("123456789").is_err());

        // 6. Testes com gerador determinístico
        let sample_bases: [&[u32; 9]; 3] = [
            &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            &[9, 8, 7, 6, 5, 4, 3, 2, 1],
            &[2, 4, 6, 8, 0, 1, 3, 5, 7],
        ];
        for base in sample_bases {
            let generated_cpf = generate_valid_cpf_from_base(base);
            assert!(
                validate_cpf(&generated_cpf).is_ok(),
                "CPF gerado {generated_cpf} deve ser válido"
            );
        }

        let sample_cnpj_bases = [
            "123456780001",
            "ABC123XYZ001",
            "99ZZ88YY0002",
            "BR01SAOPAULO",
        ];
        for base in sample_cnpj_bases {
            let generated_cnpj = generate_valid_cnpj_from_base(base);
            assert!(
                validate_cnpj(&generated_cnpj).is_ok(),
                "CNPJ gerado {generated_cnpj} deve ser válido"
            );
        }
    }

    #[test]
    fn test_customer_search_normalization_and_cursor() {
        use crate::domain::customer::{
            build_customer_search_text, build_name_sort, normalize_for_search, remove_diacritics,
            CustomerCursor,
        };
        use crate::error::CoreError;

        // 1. Remoção de acentos em português (NFC e NFD com combinantes U+0300..U+036F)
        let accented = "José da Conceição & Filhos — Produções Audiovisuais Ltda.";
        let unaccented = remove_diacritics(accented);
        assert_eq!(
            unaccented,
            "jose da conceicao & filhos — producoes audiovisuais ltda."
        );
        assert_eq!(normalize_for_search("José"), "jose");
        assert_eq!(normalize_for_search("Jose\u{0301}"), "jose");
        assert_eq!(normalize_for_search("Sa\u{0303}o Joa\u{0303}o"), "sao joao");

        // 2. Colapso de múltiplos espaços e trim
        let raw_spacing = "   Som   &   Luz    Avançada   ";
        assert_eq!(normalize_for_search(raw_spacing), "som & luz avancada");

        // 3. build_name_sort
        assert_eq!(build_name_sort("  Álvaro  Silveira "), "alvaro silveira");
        assert_eq!(build_name_sort("Bruno Alves"), "bruno alves");
        assert!(build_name_sort("Álvaro") < build_name_sort("Bruno"));

        // 4. build_customer_search_text
        let search_text = build_customer_search_text(
            "Empresa de Eventos São Paulo",
            Some("SP Som e Luz"),
            "12345678000195",
            "12.345.678/0001-95",
            &[
                "contato@spsom.com".to_string(),
                "(11) 98765-4321".to_string(),
            ],
        );
        assert!(search_text.contains("empresa de eventos sao paulo"));
        assert!(search_text.contains("sp som e luz"));
        assert!(search_text.contains("12345678000195"));
        assert!(search_text.contains("12.345.678/0001-95"));
        assert!(search_text.contains("contato@spsom.com"));
        assert!(search_text.contains("(11) 98765-4321"));

        // 5. CustomerCursor (URL_SAFE_NO_PAD encode / decode)
        let cursor = CustomerCursor::new("carlos silva", "018f0000-0000-7000-8000-000000000042");
        let encoded = cursor.encode();
        assert!(!encoded.is_empty());
        assert!(!encoded.contains('='));
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));

        let decoded =
            CustomerCursor::decode(&encoded).expect("cursor deve decodificar com sucesso");
        assert_eq!(decoded, cursor);

        // Cursor inválido ou adulterado deve retornar CoreError::Validation
        assert!(matches!(
            CustomerCursor::decode("cursor-invalido-!@#"),
            Err(CoreError::Validation(_))
        ));
        assert!(matches!(
            CustomerCursor::decode(""),
            Err(CoreError::Validation(_))
        ));
        assert!(matches!(
            CustomerCursor::decode("e30"), // "{}" sem campos obrigatórios
            Err(CoreError::Validation(_))
        ));
    }

    #[test]
    fn test_system_roles_fixture_is_up_to_date() {
        use crate::domain::permission::SYSTEM_ROLES;
        use std::path::Path;

        #[derive(serde::Serialize)]
        struct RoleFixture<'a> {
            slug: &'a str,
            name: &'a str,
            description: &'a str,
            permissions: &'a [&'a str],
        }

        let expected: Vec<RoleFixture<'_>> = SYSTEM_ROLES
            .iter()
            .map(|r| RoleFixture {
                slug: r.slug,
                name: r.name,
                description: r.description,
                permissions: r.permissions,
            })
            .collect();
        let expected_json =
            serde_json::to_value(&expected).expect("serialize SYSTEM_ROLES fixture");

        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src/lib/data/fixtures/system-roles.json");
        let raw = std::fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
            panic!(
                "Fixture {:?} ausente ({}). Gere-o com SYSTEM_ROLES.",
                fixture_path, e
            )
        });
        let actual_json: serde_json::Value =
            serde_json::from_str(&raw).expect("system-roles.json inválido");

        assert_eq!(
            actual_json,
            expected_json,
            "src/lib/data/fixtures/system-roles.json está desatualizado em relação a SYSTEM_ROLES do core! Esperado:\n{}",
            serde_json::to_string_pretty(&expected_json).unwrap_or_default()
        );
    }
}
