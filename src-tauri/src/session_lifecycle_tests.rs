use crate::{AppConfig, AppState};
use chrono::Utc;
use eventrent_core::{
    ensure_default_tenant, CoreError, CoreServices, CreateUserInput, InitialSetupInput, LoginInput,
    LoginPolicy, OperationMode, ResetPasswordInput, Role,
};
use eventrent_store_sqlite::{init_db, SqliteStore};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

pub(crate) struct TempDirGuard(pub(crate) PathBuf);

impl TempDirGuard {
    #[allow(dead_code)]
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) async fn setup_real_app_state() -> (
    AppState,
    TempDirGuard,
    String,
    eventrent_core::SessionInfo,
    PathBuf,
) {
    let temp_dir = std::env::temp_dir().join(format!("eventrent_appstate_s3_{}", Uuid::now_v7()));
    std::fs::create_dir_all(&temp_dir).expect("Falha ao criar diretório temporário para teste");

    let db_path = temp_dir.join("eventrent_test.db");
    let pool = init_db(&db_path)
        .await
        .expect("Falha ao inicializar SQLite em arquivo temporário");

    let stores = SqliteStore::new(pool.clone()).into_bundle();
    let tenant_id = ensure_default_tenant(stores.tenants.as_ref(), stores.branding.as_ref())
        .await
        .expect("Falha ao inicializar tenant padrão");

    let services = CoreServices::new(stores);

    let setup = services
        .complete_initial_setup(
            &tenant_id,
            &InitialSetupInput {
                company_name: "Locadora Real AppState".to_string(),
                owner_name: "Proprietário Real".to_string(),
                owner_email: "owner@real.local".to_string(),
                owner_password: "SenhaForteOwner123!".to_string(),
                mode: OperationMode::Local,
            },
            30,
        )
        .await
        .expect("Falha no setup inicial");

    let config_path = temp_dir.join("config.json");
    let _ = std::fs::write(&config_path, "{}");
    let media_dir = temp_dir.join("media");
    std::fs::create_dir_all(&media_dir).expect("Falha ao criar pasta de mídia");
    let temp_media_dir = temp_dir.join("temp-media");
    std::fs::create_dir_all(&temp_media_dir).expect("Falha ao criar pasta de mídia temporária");

    let backup_service = eventrent_backup_fs::BackupFsService::new(
        temp_dir.clone(),
        db_path.clone(),
        temp_dir.join("media"),
        config_path.clone(),
    );

    let app_state = AppState {
        services,
        default_tenant_id: tenant_id.clone(),
        config_path,
        temp_media_dir,
        backup_service,
        db_pool: pool,
        app_version: "0.1.0".to_string(),
        config: Mutex::new(AppConfig::default()),
        session: Mutex::new(Some(setup.session.clone())),
    };

    (
        app_state,
        TempDirGuard(temp_dir),
        tenant_id,
        setup.session,
        db_path,
    )
}

#[tokio::test]
async fn test_appstate_deactivating_user_makes_next_command_unauthorized_and_clears_session() {
    let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    // 1. Obtém papel Técnico
    let tech_role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "technician")
        .await
        .unwrap()
        .expect("Papel technician deve existir");

    let tech_user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "João Técnico".to_string(),
                email: "joao@real.local".to_string(),
                password: "SenhaSegura123!".to_string(),
                role_id: tech_role.id.clone(),
            },
        )
        .await
        .unwrap();

    // 2. Técnico faz login e sua sessão é configurada no AppState
    let tech_session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "joao@real.local".to_string(),
                password: "SenhaSegura123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    app_state.set_session(Some(tech_session.clone()));

    // 3. Primeiro comando protegido com usuário ativo é autorizado
    let verified = app_state.require_session().await;
    assert!(
        verified.is_ok(),
        "Usuário ativo deve passar na guarda de sessão"
    );

    // 4. Proprietário desativa o usuário Técnico no banco
    app_state
        .services
        .set_user_active(&owner_session, &tech_user.id, false)
        .await
        .unwrap();

    // 5. O PRÓXIMO comando protegido no AppState falha com Unauthorized
    let next_command_result = app_state.require_session().await;
    assert!(
        next_command_result.is_err(),
        "Próximo comando protegido DEVE ser negado após desativação"
    );
    let err_msg = next_command_result.unwrap_err();
    assert!(
        err_msg.contains("inativa") || err_msg.contains("desativada"),
        "Mensagem de erro deve indicar conta inativa/desativada: {err_msg}"
    );

    // 6. A sessão no AppState é encerrada (fail-closed imediato)
    assert!(
        app_state.get_session_without_touch().is_none(),
        "Sessão no AppState DEVE ser limpa imediatamente"
    );
}

#[tokio::test]
async fn test_appstate_changing_role_permissions_updates_session_permissions_immediately_and_denies_command(
) {
    let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    // 1. Cria papel customizado com permissão "catalog:read" e "catalog:manage"
    let custom_role = app_state
        .services
        .stores
        .roles
        .create(
            &Role {
                id: "role-operator-perm".to_string(),
                tenant_id: tenant_id.clone(),
                name: "Operador de Catálogo".to_string(),
                slug: "op_cat".to_string(),
                description: None,
                is_system: false,
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                deleted_at: None,
                created_by: Some(owner_session.user_id.clone()),
            },
            &["catalog:read".to_string(), "catalog:manage".to_string()],
        )
        .await
        .unwrap();

    let _user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Operador Silva".to_string(),
                email: "silva@real.local".to_string(),
                password: "SenhaValida123!".to_string(),
                role_id: custom_role.id.clone(),
            },
        )
        .await
        .unwrap();

    // 2. Operador faz login com ambas as permissões
    let session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "silva@real.local".to_string(),
                password: "SenhaValida123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    assert!(session.has_permission("catalog:manage"));
    app_state.set_session(Some(session));

    // 3. Execução inicial do comando protegido: permitido
    let active_session = app_state.require_session().await.unwrap();
    assert!(
        app_state
            .services
            .check_session_and_permission(&active_session, "catalog:manage")
            .is_ok(),
        "Operador deve ter permissão catalog:manage inicialmente"
    );

    // 4. Administrador atualiza o perfil (remove 'catalog:manage' da role no banco)
    app_state
        .services
        .stores
        .roles
        .set_permissions(
            &tenant_id,
            &custom_role.id,
            &["catalog:read".to_string()],
            &Utc::now().to_rfc3339(),
        )
        .await
        .unwrap();

    // 5. No PRÓXIMO comando protegido, a sessão recarrega as permissões na hora
    let revalidated_session = app_state
        .require_session()
        .await
        .expect("Sessão continua autenticada");

    assert!(
        revalidated_session.has_permission("catalog:read"),
        "catalog:read ainda deve existir"
    );
    assert!(
        !revalidated_session.has_permission("catalog:manage"),
        "catalog:manage NÃO deve mais existir após atualização de perfil"
    );

    // 6. O comando antes permitido passa a ser negado com Forbidden
    let permission_check = app_state
        .services
        .check_session_and_permission(&revalidated_session, "catalog:manage");

    assert!(
        matches!(permission_check, Err(CoreError::Forbidden(_))),
        "Comando exigindo catalog:manage DEVE ser negado com Forbidden na hora"
    );
}

#[tokio::test]
async fn test_appstate_resetting_password_invalidates_session() {
    let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    // 1. Cria usuário
    let role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "technician")
        .await
        .unwrap()
        .expect("Papel technician deve existir");

    let user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Maria Luz".to_string(),
                email: "maria@real.local".to_string(),
                password: "SenhaAtual123!".to_string(),
                role_id: role.id,
            },
        )
        .await
        .unwrap();

    // 2. Maria loga
    let session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "maria@real.local".to_string(),
                password: "SenhaAtual123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    app_state.set_session(Some(session));
    assert!(app_state.require_session().await.is_ok());

    // 3. Proprietário redefine a senha da usuária
    app_state
        .services
        .reset_user_password(
            &owner_session,
            &user.id,
            &ResetPasswordInput {
                new_password: "NovaSenhaSegura456!".to_string(),
            },
        )
        .await
        .unwrap();

    // 4. Próxima chamada de require_session falha e encerra a sessão
    let err = app_state.require_session().await.unwrap_err();
    assert!(
        err.contains("revogada") || err.contains("alteração"),
        "Erro deve indicar sessão revogada: {err}"
    );
    assert!(
        app_state.get_session_without_touch().is_none(),
        "Sessão deve ser descartada do AppState"
    );
}

#[tokio::test]
async fn test_appstate_deleting_user_invalidates_session() {
    let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    let role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "commercial")
        .await
        .unwrap()
        .expect("Papel commercial deve existir");

    let user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Pedro Vendas".to_string(),
                email: "pedro@real.local".to_string(),
                password: "SenhaPedro123!".to_string(),
                role_id: role.id,
            },
        )
        .await
        .unwrap();

    let session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "pedro@real.local".to_string(),
                password: "SenhaPedro123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    app_state.set_session(Some(session));
    assert!(app_state.require_session().await.is_ok());

    // 3. Usuário sofre soft_delete no banco (exclusão lógica)
    let now = chrono::Utc::now().to_rfc3339();
    app_state
        .services
        .stores
        .users
        .soft_delete(&tenant_id, &user.id, &now)
        .await
        .unwrap();

    // 4. Próxima chamada é rejeitada imediatamente
    let err = app_state.require_session().await.unwrap_err();
    assert!(
        err.contains("inativa") || err.contains("desativada") || err.contains("não encontrado"),
        "Erro deve indicar exclusão/desativação: {err}"
    );
    assert!(app_state.get_session_without_touch().is_none());
}

#[tokio::test]
async fn test_appstate_regenerating_recovery_key_invalidates_other_sessions() {
    let (app_state1, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    let role = app_state1
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "technician")
        .await
        .unwrap()
        .expect("Papel technician deve existir");

    let _tech_user = app_state1
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Ana Técnica".to_string(),
                email: "ana@real.local".to_string(),
                password: "SenhaAnaForte123!".to_string(),
                role_id: role.id,
            },
        )
        .await
        .unwrap();

    // 1. Ana loga no Dispositivo 1 (Sessão 1)
    let session_dev1 = app_state1
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "ana@real.local".to_string(),
                password: "SenhaAnaForte123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    // 2. Ana loga no Dispositivo 2 (Sessão 2)
    let session_dev2 = app_state1
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "ana@real.local".to_string(),
                password: "SenhaAnaForte123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    app_state1.set_session(Some(session_dev1.clone()));

    // Simula AppState 2 (Dispositivo 2)
    let app_state2 = AppState {
        services: app_state1.services.clone(),
        default_tenant_id: tenant_id.clone(),
        config_path: app_state1.config_path.clone(),
        temp_media_dir: app_state1.temp_media_dir.clone(),
        backup_service: app_state1.backup_service.clone(),
        db_pool: app_state1.db_pool.clone(),
        app_version: app_state1.app_version.clone(),
        config: Mutex::new(AppConfig::default()),
        session: Mutex::new(Some(session_dev2.clone())),
    };

    assert!(app_state1.require_session().await.is_ok());
    assert!(app_state2.require_session().await.is_ok());

    // 3. No Dispositivo 1, Ana regenera sua chave de recuperação
    let regen_res = app_state1
        .services
        .regenerate_recovery_key(&session_dev1, "SenhaAnaForte123!", &LoginPolicy::default())
        .await
        .unwrap();

    assert!(!regen_res.new_recovery_key.is_empty());

    // O AppState sincroniza a sessão atual para que ela não seja derrubada pela própria operação
    app_state1.sync_current_session_epoch().await.unwrap();

    // 4. A sessão ATUAL (Dispositivo 1) CONTINUA VÁLIDA
    assert!(
        app_state1.require_session().await.is_ok(),
        "Sessão atual no Dispositivo 1 DEVE continuar válida após regenerar a chave"
    );

    // 5. O Dispositivo 2 (que possui auth_epoch antigo) tenta executar um comando protegido
    let dev2_result = app_state2.require_session().await;
    assert!(
        dev2_result.is_err(),
        "Dispositivo 2 DEVE ter sua sessão invalidada após regeneração de chave"
    );
    let dev2_err = dev2_result.unwrap_err();
    assert!(
        dev2_err.contains("revogada"),
        "Erro deve indicar sessão revogada: {dev2_err}"
    );
    assert!(
        app_state2.get_session_without_touch().is_none(),
        "Sessão no Dispositivo 2 DEVE ser encerrada"
    );
}

#[tokio::test]
async fn test_appstate_auth_epoch_increments_only_on_specified_five_security_events() {
    use eventrent_core::UpdateUserInput;

    let (app_state, _guard, tenant_id, owner_session, db_path) = setup_real_app_state().await;

    let role1 = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "technician")
        .await
        .unwrap()
        .expect("Papel technician deve existir");
    let role2 = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "commercial")
        .await
        .unwrap()
        .expect("Papel commercial deve existir");

    let user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Lucas Teste".to_string(),
                email: "lucas@real.local".to_string(),
                password: "SenhaBase123!".to_string(),
                role_id: role1.id,
            },
        )
        .await
        .unwrap();

    // Helper para ler auth_epoch atual no SQLite
    let get_current_epoch = || {
        let db_path = db_path.clone();
        let tenant_id = tenant_id.clone();
        let user_id = user.id.clone();
        async move {
            let pool = eventrent_store_sqlite::init_db(&db_path).await.unwrap();
            let epoch: i64 = eventrent_store_sqlite::sqlx::query_scalar(
                "SELECT auth_epoch FROM app_user WHERE tenant_id = ? AND id = ?",
            )
            .bind(&tenant_id)
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
            epoch
        }
    };

    assert_eq!(
        get_current_epoch().await,
        0,
        "auth_epoch inicial deve ser 0"
    );

    // Operações não-críticas NÃO devem incrementar auth_epoch:
    // A) Login repetido
    let session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "lucas@real.local".to_string(),
                password: "SenhaBase123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();
    assert_eq!(get_current_epoch().await, 0, "Login não altera auth_epoch");

    // B) Polling e touch
    app_state.set_session(Some(session.clone()));
    let _ = app_state.get_session_without_touch();
    let _ = app_state.require_session().await.unwrap();
    assert_eq!(
        get_current_epoch().await,
        0,
        "Polling/touch não altera auth_epoch"
    );

    // C) Evento de Segurança 1: Mudar perfil do usuário (UpdateUserInput com novo role_id)
    app_state
        .services
        .update_user(
            &owner_session,
            &user.id,
            &UpdateUserInput {
                name: "Lucas Silva".to_string(),
                email: "lucas@real.local".to_string(),
                role_id: role2.id,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        get_current_epoch().await,
        1,
        "auth_epoch DEVE incrementar para 1 ao mudar perfil"
    );

    // D) Evento de Segurança 2: Redefinição de senha
    app_state
        .services
        .reset_user_password(
            &owner_session,
            &user.id,
            &ResetPasswordInput {
                new_password: "NovaSenhaLucas456!".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        get_current_epoch().await,
        2,
        "auth_epoch DEVE incrementar para 2 ao redefinir senha"
    );

    // Novo login com credencial atualizada
    let session_v2 = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "lucas@real.local".to_string(),
                password: "NovaSenhaLucas456!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    // E) Evento de Segurança 3: Regenerar chave de recuperação
    app_state
        .services
        .regenerate_recovery_key(&session_v2, "NovaSenhaLucas456!", &LoginPolicy::default())
        .await
        .unwrap();
    assert_eq!(
        get_current_epoch().await,
        3,
        "auth_epoch DEVE incrementar para 3 ao regenerar chave de recuperação"
    );

    // F) Evento de Segurança 4: Desativação do usuário
    app_state
        .services
        .set_user_active(&owner_session, &user.id, false)
        .await
        .unwrap();
    assert_eq!(
        get_current_epoch().await,
        4,
        "auth_epoch DEVE incrementar para 4 ao desativar usuário"
    );

    // G) Evento de Segurança 5: Exclusão lógica (soft delete)
    let now = chrono::Utc::now().to_rfc3339();
    app_state
        .services
        .stores
        .users
        .soft_delete(&tenant_id, &user.id, &now)
        .await
        .unwrap();
    assert_eq!(
        get_current_epoch().await,
        5,
        "auth_epoch DEVE incrementar para 5 ao excluir usuário"
    );
}

#[tokio::test]
async fn test_backup_commands_rbac_technician_and_commercial_forbidden_and_owner_allowed() {
    let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    // 1. Cria usuário Técnico
    let tech_role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "technician")
        .await
        .unwrap()
        .expect("Papel technician deve existir");

    let _tech_user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Técnico Silva".to_string(),
                email: "tecnico@real.local".to_string(),
                password: "SenhaTech123!".to_string(),
                role_id: tech_role.id,
            },
        )
        .await
        .unwrap();

    let tech_session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "tecnico@real.local".to_string(),
                password: "SenhaTech123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    // 2. Cria usuário Comercial
    let comm_role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "commercial")
        .await
        .unwrap()
        .expect("Papel commercial deve existir");

    let _comm_user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Comercial Santos".to_string(),
                email: "comercial@real.local".to_string(),
                password: "SenhaComm123!".to_string(),
                role_id: comm_role.id,
            },
        )
        .await
        .unwrap();

    let comm_session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "comercial@real.local".to_string(),
                password: "SenhaComm123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    // 3. Testa os 8 comandos com a sessão do Técnico -> DEVE retornar Forbidden em todos
    // 1) get_backup_status
    let res = app_state.services.get_backup_status(&tech_session).await;
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "get_backup_status deve ser proibido para técnico"
    );

    // 2) set_backup_settings
    let res = app_state
        .services
        .set_backup_settings(&tech_session, &eventrent_core::BackupSettings::default())
        .await;
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "set_backup_settings deve ser proibido para técnico"
    );

    // 3) run_backup_now (autorização no core)
    let res = app_state.services.authorize_backup_operation(&tech_session);
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "run_backup_now deve ser proibido para técnico"
    );

    // 4) list_backups (autorização no core)
    let res = app_state.services.authorize_backup_operation(&tech_session);
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "list_backups deve ser proibido para técnico"
    );

    // 5) verify_backup (autorização no core)
    let res = app_state.services.authorize_backup_operation(&tech_session);
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "verify_backup deve ser proibido para técnico"
    );

    // 6) schedule_restore (autorização no core)
    let res = app_state.services.authorize_backup_operation(&tech_session);
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "schedule_restore deve ser proibido para técnico"
    );
    let res_restore = app_state
        .services
        .authorize_restore_operation(&tech_session, "SenhaQualquer", &LoginPolicy::default())
        .await;
    assert!(
        matches!(res_restore, Err(CoreError::Forbidden(_))),
        "authorize_restore_operation deve ser proibido para técnico"
    );

    // 7) choose_backup_folder (autorização no core)
    let res = app_state.services.authorize_backup_operation(&tech_session);
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "choose_backup_folder deve ser proibido para técnico"
    );

    // 8) choose_backup_file (autorização no core)
    let res = app_state.services.authorize_backup_operation(&tech_session);
    assert!(
        matches!(res, Err(CoreError::Forbidden(_))),
        "choose_backup_file deve ser proibido para técnico"
    );

    // 4. Testa os 8 comandos com a sessão do Comercial -> DEVE retornar Forbidden em todos
    assert!(
        matches!(
            app_state.services.get_backup_status(&comm_session).await,
            Err(CoreError::Forbidden(_))
        ),
        "get_backup_status deve ser proibido para comercial"
    );
    assert!(
        matches!(
            app_state
                .services
                .set_backup_settings(&comm_session, &eventrent_core::BackupSettings::default())
                .await,
            Err(CoreError::Forbidden(_))
        ),
        "set_backup_settings deve ser proibido para comercial"
    );
    assert!(
        matches!(
            app_state.services.authorize_backup_operation(&comm_session),
            Err(CoreError::Forbidden(_))
        ),
        "authorize_backup_operation deve ser proibido para comercial"
    );

    let res_comm_restore = app_state
        .services
        .authorize_restore_operation(&comm_session, "SenhaQualquer", &LoginPolicy::default())
        .await;
    assert!(
        matches!(res_comm_restore, Err(CoreError::Forbidden(_))),
        "authorize_restore_operation deve ser proibido para comercial"
    );

    // 5. Testa com o Proprietário -> DEVE ter permissão
    assert!(
        app_state
            .services
            .get_backup_status(&owner_session)
            .await
            .is_ok(),
        "Proprietário deve poder consultar backup status"
    );
    assert!(
        app_state
            .services
            .authorize_backup_operation(&owner_session)
            .is_ok(),
        "Proprietário deve ter autorização para operações de backup"
    );
    let owner_wrong_pw = app_state
        .services
        .authorize_restore_operation(
            &owner_session,
            "SenhaIncorreta#999",
            &LoginPolicy::default(),
        )
        .await;
    assert!(
        matches!(owner_wrong_pw, Err(CoreError::InvalidCredentials)),
        "schedule_restore com senha incorreta deve retornar InvalidCredentials"
    );
    let owner_right_pw = app_state
        .services
        .authorize_restore_operation(
            &owner_session,
            "SenhaForteOwner123!",
            &LoginPolicy::default(),
        )
        .await;
    assert!(
        owner_right_pw.is_ok(),
        "schedule_restore com senha correta do proprietário deve ser autorizado com sucesso"
    );
}

#[tokio::test]
async fn test_inventory_permissions_enforced_at_core_level_and_financial_masking() {
    let (app_state, _guard, tenant_id, owner_session, _db_path) = setup_real_app_state().await;

    // 1. Cria usuário Técnico
    let tech_role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "technician")
        .await
        .unwrap()
        .expect("Papel technician deve existir");

    let _tech_user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Técnico Silva".to_string(),
                email: "tecnico_inv@real.local".to_string(),
                password: "SenhaTech123!".to_string(),
                role_id: tech_role.id,
            },
        )
        .await
        .unwrap();

    let tech_session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "tecnico_inv@real.local".to_string(),
                password: "SenhaTech123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    // 2. Cria usuário Comercial
    let comm_role = app_state
        .services
        .stores
        .roles
        .find_by_slug(&tenant_id, "commercial")
        .await
        .unwrap()
        .expect("Papel commercial deve existir");

    let _comm_user = app_state
        .services
        .create_user(
            &owner_session,
            &CreateUserInput {
                name: "Comercial Santos".to_string(),
                email: "comercial_inv@real.local".to_string(),
                password: "SenhaComm123!".to_string(),
                role_id: comm_role.id,
            },
        )
        .await
        .unwrap();

    let comm_session = app_state
        .services
        .login(
            &tenant_id,
            &LoginInput {
                email: "comercial_inv@real.local".to_string(),
                password: "SenhaComm123!".to_string(),
            },
            &LoginPolicy::default(),
            30,
        )
        .await
        .unwrap();

    // 3. Usuário Comercial NÃO possui warehouse:read nem warehouse:operate
    assert!(
        matches!(
            app_state
                .services
                .list_warehouses(&comm_session, false)
                .await,
            Err(CoreError::Forbidden(_))
        ),
        "Comercial não deve poder listar depósitos"
    );

    let create_wh_input = eventrent_core::domain::inventory::CreateWarehouseInput {
        name: "Depósito Proibido".to_string(),
        code: "PROIB-01".to_string(),
        description: None,
        address_line1: None,
        address_line2: None,
        city: None,
        state: None,
        postal_code: None,
        is_default: None,
    };

    assert!(
        matches!(
            app_state
                .services
                .create_warehouse(&comm_session, &create_wh_input)
                .await,
            Err(CoreError::Forbidden(_))
        ),
        "Comercial não deve poder criar depósitos"
    );

    assert!(
        matches!(
            app_state
                .services
                .list_equipment_units(&comm_session, &Default::default())
                .await,
            Err(CoreError::Forbidden(_))
        ),
        "Comercial não deve poder listar unidades físicas"
    );

    // 4. Usuário Técnico e Proprietário POSSUEM warehouse:read e warehouse:operate
    let tech_warehouses = app_state
        .services
        .list_warehouses(&tech_session, false)
        .await;
    assert!(
        tech_warehouses.is_ok(),
        "Técnico deve poder listar depósitos"
    );
    assert!(
        !tech_warehouses.unwrap().is_empty(),
        "Depósito padrão deve estar presente"
    );

    let created_wh = app_state
        .services
        .create_warehouse(
            &tech_session,
            &eventrent_core::domain::inventory::CreateWarehouseInput {
                name: "Galpão Técnico".to_string(),
                code: "GALP-TECH".to_string(),
                description: Some("Criado pelo técnico".to_string()),
                address_line1: None,
                address_line2: None,
                city: Some("São Paulo".to_string()),
                state: Some("SP".to_string()),
                postal_code: None,
                is_default: Some(false),
            },
        )
        .await;
    assert!(created_wh.is_ok(), "Técnico deve poder criar depósitos");

    // 5. Teste de confidencialidade financeira: Técnico tem acesso a modelo mas valores financeiros devem ser None
    let category = app_state
        .services
        .create_category(
            &owner_session,
            &eventrent_core::CreateCategoryInput {
                parent_id: None,
                name: "Áudio".to_string(),
                description: None,
                sort_order: 1,
                active: true,
            },
        )
        .await
        .expect("Proprietário deve conseguir cadastrar categoria");

    let cat_input = eventrent_core::CreateEquipmentModelInput {
        category_id: category.id,
        brand_id: None,
        name: "Microfone Shure SM58".to_string(),
        model_code: None,
        sku: None,
        ean: None,
        ncm: None,
        description: None,
        control_mode: eventrent_core::ControlMode::Serialized,
        weight_kg: None,
        length_cm: None,
        width_cm: None,
        height_cm: None,
        power_watts: None,
        voltage: None,
        amperage: None,
        requires_case: false,
        units_per_case: None,
        daily_rate_cents: Some(5000),
        replacement_value_cents: Some(120000),
        cost_price_cents: Some(85000),
        active: true,
        attribute_values: vec![],
    };

    let model = app_state
        .services
        .create_equipment_model(&owner_session, &cat_input)
        .await
        .expect("Proprietário deve conseguir cadastrar modelo");

    // Consulta pelo Proprietário -> financeiro presente
    let owner_view = app_state
        .services
        .get_equipment_model(&owner_session, &model.model.id)
        .await
        .unwrap();
    assert_eq!(owner_view.model.cost_price_cents, Some(85000));
    assert_eq!(owner_view.model.replacement_value_cents, Some(120000));

    // Consulta pelo Técnico -> financeiro sanitizado (None)
    let tech_view = app_state
        .services
        .get_equipment_model(&tech_session, &model.model.id)
        .await
        .unwrap();
    assert_eq!(
        tech_view.model.cost_price_cents, None,
        "Preço de custo deve ser mascarado para técnico sem finance:read"
    );
    assert_eq!(
        tech_view.model.replacement_value_cents, None,
        "Valor de reposição deve ser mascarado para técnico sem finance:read"
    );

    // 6. Técnico consegue tombar unidade física para este modelo
    let unit = app_state
        .services
        .create_equipment_unit(
            &tech_session,
            &eventrent_core::domain::inventory::CreateEquipmentUnitInput {
                equipment_model_id: model.model.id.clone(),
                warehouse_id: Some(created_wh.unwrap().id),
                serial_number: Some("SHURE-SN-999".to_string()),
                physical_condition: Some(eventrent_core::domain::inventory::PhysicalCondition::New),
                notes: Some("Unidade nova na caixa".to_string()),
                acquisition_date: None,
                warranty_expires_at: None,
            },
        )
        .await;
    assert!(unit.is_ok(), "Técnico deve conseguir tombar unidade física");
    let unit = unit.unwrap();
    assert_eq!(
        unit.patrimony_tag, "EQ000001",
        "Primeiro patrimônio sequencial deve ser EQ000001"
    );
}
