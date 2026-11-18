use crate::domain::{
    build_customer_search_text, build_name_sort, build_venue_name_sort, build_venue_search_text,
    mask_email_for_audit, mask_individual_name_for_audit, normalize_for_search,
    parse_document_with_type, validate_addresses, validate_contacts,
    validate_customer_common_fields, validate_email, validate_roles_and_categories,
    validate_user_name, validate_venue_contacts, validate_venue_fields, validate_venue_spaces,
    AllocationSettings, AuditLogEntry, BrandingValidator, CreateCustomerInput, CreateUserInput,
    CreateVenueInput, Customer, CustomerAddress, CustomerContact, CustomerCursor, CustomerFilter,
    CustomerPage, CustomerRole, CustomerRoleFilter, CustomerWithDetails, DocumentType,
    InitialSetupInput, InitialSetupResult, LoginInput, LoginPolicy, OperationMode, PersonType,
    PublicBranding, RecoverPasswordInput, RecoverPasswordResult, ResetPasswordInput, Role,
    RoleWithPermissions, SessionInfo, Tenant, TenantBranding, TenantSetting,
    UpdateAllocationSettingsInput, UpdateBrandingInput, UpdateCustomerInput, UpdateUserInput,
    UpdateVenueInput, UserRecord, UserWithRole, Venue, VenueContact, VenueCursor, VenueFilter,
    VenuePage, VenueSpace, VenueWithDetails, MAX_CHANGEOVER_MINUTES, MAX_EXTERNAL_TRANSFER_MINUTES,
    MAX_INTERNAL_TRANSFER_SETTING_MINUTES, PERM_BRANDING_MANAGE, PERM_BRANDING_READ,
    PERM_CUSTOMER_MANAGE, PERM_CUSTOMER_READ, PERM_ROLE_READ, PERM_SETTINGS_MANAGE,
    PERM_SETTINGS_READ, PERM_SUPPLIER_MANAGE, PERM_SUPPLIER_READ, PERM_USER_MANAGE, PERM_USER_READ,
    PERM_VENUE_MANAGE, PERM_VENUE_READ, ROLE_SLUG_OWNER, SETTING_CHANGEOVER_MINUTES,
    SETTING_EXTERNAL_TRANSFER_MINUTES, SETTING_INTERNAL_TRANSFER_MINUTES, SYSTEM_ROLES,
};
use crate::error::{CoreError, Result};
use crate::store::{
    run_in_tx, BrandingStore, RoleStore, StoreBundle, TenantStore, TransactionPolicy, TxBoxFuture,
    TxStoreBundle,
};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use std::collections::HashSet;
use uuid::Uuid;

pub const DEFAULT_COMPANY_NAME: &str = "Minha empresa";
pub const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;
pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_PASSWORD_LEN: usize = 128;

/// Hash fictício constante com formato e custo idênticos ao Argon2id OWASP.
/// Executado quando um usuário não existe ou está inativo para eliminar timing side-channels
/// e mitigar ataques de enumeração de e-mails.
pub const DUMMY_ARGON2_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$ZXZlbnRyZW50c2FsdDEyMw$U29tZVVua25vd25QYXNzd29yZEhhc2hEYXRhMTIzNDU";

/// Valida tamanho de senha em contagem de caracteres Unicode (RFC/OWASP: 8 a 128 caracteres).
pub fn validate_password(password: &str) -> Result<()> {
    let char_count = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&char_count) {
        return Err(CoreError::Validation(format!(
            "A senha deve ter entre {MIN_PASSWORD_LEN} e {MAX_PASSWORD_LEN} caracteres."
        )));
    }
    Ok(())
}

/// Configuração recomendada pelo OWASP Password Storage Cheat Sheet para Argon2id:
/// - Memória (m): 19 MiB (19.456 KiB = 19 * 1024)
/// - Iterações de tempo (t): 2 passes
/// - Paralelismo (p): 1 thread (lançado em tokio::task::spawn_blocking)
/// - Tamanho do Hash (output): padrão de 32 bytes
pub fn create_owasp_argon2() -> Result<Argon2<'static>> {
    let params = Params::new(19 * 1024, 2, 1, None)
        .map_err(|e| CoreError::Internal(format!("Parâmetros Argon2id inválidos: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Hash de senha ou chave de recuperação utilizando Argon2id com parâmetros OWASP (m=19 MiB, t=2, p=1).
/// Executa estritamente dentro de `tokio::task::spawn_blocking` para não bloquear a thread
/// do runtime assíncrono nem congelar a interface do Tauri.
pub async fn hash_secret(plain: &str) -> Result<String> {
    let plain = plain.to_string();
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = create_owasp_argon2()?;
        argon2
            .hash_password(plain.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| CoreError::Internal("Falha ao processar hash criptográfico.".to_string()))
    })
    .await
    .map_err(|e| CoreError::Internal(format!("Falha ao despachar spawn_blocking para hash: {e}")))?
}

/// Verifica uma senha ou chave de recuperação contra um hash Argon2id.
/// Executa estritamente dentro de `tokio::task::spawn_blocking` para manter a thread assíncrona responsiva.
pub async fn verify_secret(plain: &str, password_hash: &str) -> Result<bool> {
    let plain = plain.to_string();
    let hash = password_hash.to_string();
    tokio::task::spawn_blocking(move || {
        let Ok(parsed_hash) = PasswordHash::new(&hash) else {
            return Ok(false);
        };
        let argon2 = create_owasp_argon2().unwrap_or_default();
        Ok(argon2
            .verify_password(plain.as_bytes(), &parsed_hash)
            .is_ok())
    })
    .await
    .map_err(|e| {
        CoreError::Internal(format!(
            "Falha ao despachar spawn_blocking para verificação: {e}"
        ))
    })?
}

/// Gera uma chave de recuperação de alta entropia no formato `ER-XXXX-XXXX-XXXX-XXXX`.
///
/// ### Modelo de Entropia e Alfabeto Criptográfico:
/// - Alfabeto: 32 caracteres seguros (`ABCDEFGHJKLMNPQRSTUVWXYZ23456789`), excluindo
///   os caracteres ambíguos `0`, `O`, `1` e `I` para prevenir confusão visual em anotações.
/// - Comprimento: 4 blocos de 4 caracteres = 16 símbolos aleatórios independentes gerados via `OsRng`.
/// - Entropia por símbolo: $\log_2(32) = 5$ bits.
/// - Entropia total: $16 \times 5 = 80$ bits de entropia criptográfica.
/// - Camadas complementares: Hash persistido com Argon2id (parâmetros OWASP) e bloqueio
///   progressivo com backoff exponencial por usuário mitigam tentativas automatizadas.
pub fn generate_recovery_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = OsRng;
    let mut groups = Vec::with_capacity(4);

    for _ in 0..4 {
        let mut group = String::with_capacity(4);
        for _ in 0..4 {
            let idx = rng.gen_range(0..CHARSET.len());
            group.push(CHARSET[idx] as char);
        }
        groups.push(group);
    }

    format!("ER-{}", groups.join("-"))
}

pub fn normalize_recovery_key(key: &str) -> String {
    key.trim().to_uppercase()
}

/// Calcula o tempo de bloqueio temporário em segundos com backoff exponencial a partir de 5 falhas.
pub fn calculate_lockout_seconds(failed_attempts: i32) -> i64 {
    if failed_attempts < MAX_FAILED_LOGIN_ATTEMPTS {
        return 0;
    }
    match failed_attempts {
        5 => 5 * 60,  // 5 minutos
        6 => 10 * 60, // 10 minutos
        7 => 20 * 60, // 20 minutos
        _ => 60 * 60, // 60 minutos (teto)
    }
}

/// Garante de forma idempotente a existência do tenant padrão (`slug = 'default'`)
/// e de sua configuração inicial de marca ("Minha empresa").
pub async fn ensure_default_tenant(
    tenants: &dyn TenantStore,
    branding: &dyn BrandingStore,
) -> Result<String> {
    let now = Utc::now().to_rfc3339();

    let tenant_id = if let Some(existing) = tenants.find_by_slug("default").await? {
        existing.id
    } else {
        let new_id = Uuid::now_v7().to_string();
        let tenant = Tenant {
            id: new_id.clone(),
            tenant_id: new_id.clone(),
            name: DEFAULT_COMPANY_NAME.to_string(),
            slug: "default".to_string(),
            document_number: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: None,
        };
        tenants.create(&tenant).await?.id
    };

    if branding.get_by_tenant_id(&tenant_id).await?.is_none() {
        let default_branding = UpdateBrandingInput {
            company_name: DEFAULT_COMPANY_NAME.to_string(),
            primary_color: "#2563eb".to_string(),
            secondary_color: "#475569".to_string(),
            accent_color: "#7c3aed".to_string(),
            logo_url: None,
            logo_dark_url: None,
            icon_url: None,
        };
        branding.save(&tenant_id, &default_branding).await?;
    }

    Ok(tenant_id)
}

/// Garante de forma idempotente a criação dos 5 perfis de sistema para um tenant.
/// Compara conjuntos de permissões (em vez de mera contagem) e registra auditoria ao corrigir divergências.
pub async fn ensure_system_roles(
    tenant_id: &str,
    roles: &dyn RoleStore,
    audit: &dyn crate::store::AuditStore,
) -> Result<Vec<RoleWithPermissions>> {
    let now = Utc::now().to_rfc3339();

    for spec in SYSTEM_ROLES {
        let perms: Vec<String> = spec.permissions.iter().map(|s| (*s).to_string()).collect();

        if let Some(existing) = roles.find_by_slug(tenant_id, spec.slug).await? {
            let expected_set: HashSet<&str> = spec.permissions.iter().copied().collect();
            let current_set: HashSet<&str> =
                existing.permissions.iter().map(|s| s.as_str()).collect();

            if expected_set != current_set {
                roles
                    .set_permissions(tenant_id, &existing.id, &perms, &now)
                    .await?;

                let entry = AuditLogEntry {
                    id: Uuid::now_v7().to_string(),
                    tenant_id: tenant_id.to_string(),
                    actor_user_id: None,
                    action: "role.system_permissions_repaired".to_string(),
                    entity_type: "role".to_string(),
                    entity_id: existing.id.clone(),
                    summary: format!(
                        "Permissões do perfil de sistema '{}' sincronizadas com o catálogo canônico",
                        spec.name
                    ),
                    created_at: now.clone(),
                    created_by: None,
                };
                audit.append(&entry).await?;
            }
        } else {
            let role = Role {
                id: Uuid::now_v7().to_string(),
                tenant_id: tenant_id.to_string(),
                name: spec.name.to_string(),
                slug: spec.slug.to_string(),
                description: Some(spec.description.to_string()),
                is_system: true,
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
                created_by: None,
            };
            roles.create(&role, &perms).await?;
        }
    }

    roles.list_by_tenant(tenant_id).await
}

pub fn build_audit_entry(
    tenant_id: &str,
    actor_user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    summary: &str,
) -> AuditLogEntry {
    let now = Utc::now().to_rfc3339();
    AuditLogEntry {
        id: Uuid::now_v7().to_string(),
        tenant_id: tenant_id.to_string(),
        actor_user_id: actor_user_id.map(str::to_string),
        action: action.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        summary: summary.to_string(),
        created_at: now,
        created_by: actor_user_id.map(str::to_string),
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub struct PassthroughTxStoreBundle<'a> {
    pub stores: &'a StoreBundle,
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait::async_trait]
impl<'a> TxStoreBundle for PassthroughTxStoreBundle<'a> {
    fn tenants(&self) -> &dyn TenantStore {
        self.stores.tenants.as_ref()
    }
    fn branding(&self) -> &dyn BrandingStore {
        self.stores.branding.as_ref()
    }
    fn roles(&self) -> &dyn RoleStore {
        self.stores.roles.as_ref()
    }
    fn users(&self) -> &dyn crate::store::UserStore {
        self.stores.users.as_ref()
    }
    fn audit(&self) -> &dyn crate::store::AuditStore {
        self.stores.audit.as_ref()
    }
    fn customers(&self) -> &dyn crate::store::CustomerStore {
        self.stores.customers.as_ref()
    }
    fn venues(&self) -> &dyn crate::store::VenueStore {
        self.stores.venues.as_ref()
    }
    fn settings(&self) -> &dyn crate::store::TenantSettingStore {
        self.stores.settings.as_ref()
    }
    async fn commit(self: Box<Self>) -> Result<()> {
        Ok(())
    }
    async fn rollback(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct CoreServices {
    pub stores: StoreBundle,
}

fn validate_customer_document_for_person_type(
    person_type: PersonType,
    doc_type: DocumentType,
    doc_number: Option<&str>,
) -> Result<(DocumentType, Option<String>, String)> {
    let (parsed_type, parsed_raw, parsed_formatted) =
        parse_document_with_type(doc_type, doc_number)?;
    match (person_type, parsed_type) {
        (PersonType::Individual, DocumentType::Cnpj) => {
            return Err(CoreError::Validation(
                "Pessoa Física não pode usar CNPJ; utilize CPF, documento estrangeiro ou sem documento.".to_string(),
            ));
        }
        (PersonType::Legal, DocumentType::Cpf) => {
            return Err(CoreError::Validation(
                "Pessoa Jurídica não pode usar CPF; utilize CNPJ, documento estrangeiro ou sem documento.".to_string(),
            ));
        }
        _ => {}
    }
    Ok((parsed_type, parsed_raw, parsed_formatted))
}

impl CoreServices {
    pub fn new(stores: StoreBundle) -> Self {
        Self { stores }
    }

    pub async fn run_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'a> Fn(&'a dyn TxStoreBundle) -> TxBoxFuture<'a, T>,
    {
        #[cfg(not(any(test, feature = "test-utils")))]
        {
            let mgr = self.stores.tx_manager.as_ref().ok_or_else(|| {
                CoreError::Configuration(
                    "TransactionManager (tx_manager) é obrigatório no StoreBundle para executar transações.".to_string(),
                )
            })?;
            run_in_tx(mgr.as_ref(), &TransactionPolicy::default(), f).await
        }
        #[cfg(any(test, feature = "test-utils"))]
        {
            if let Some(ref mgr) = self.stores.tx_manager {
                run_in_tx(mgr.as_ref(), &TransactionPolicy::default(), f).await
            } else {
                let bundle = PassthroughTxStoreBundle {
                    stores: &self.stores,
                };
                f(&bundle).await
            }
        }
    }

    /// Verifica se a sessão expirou por inatividade e valida a permissão exigida.
    pub fn check_session_and_permission(
        &self,
        session: &SessionInfo,
        required_permission: &str,
    ) -> Result<()> {
        self.verify_session_active(session)?;
        if !session.has_permission(required_permission) {
            return Err(CoreError::Forbidden(required_permission.to_string()));
        }
        Ok(())
    }

    pub fn verify_session_active(&self, session: &SessionInfo) -> Result<()> {
        Self::verify_session_active_at(session, Utc::now())
    }

    /// Valida se a sessão expirou por inatividade contra um instante de referência.
    /// Utilizado pela produção (`verify_session_active` com `Utc::now()`) e por testes
    /// com relógio injetável sem sleeps reais.
    pub fn verify_session_active_at(session: &SessionInfo, now: DateTime<Utc>) -> Result<()> {
        let last_activity = DateTime::parse_from_rfc3339(&session.last_activity_at)
            .map_err(|_| CoreError::Unauthorized("Timestamp de sessão inválido.".to_string()))?
            .with_timezone(&Utc);

        let timeout = Duration::minutes(i64::from(session.inactivity_timeout_minutes.max(1)));
        if now - last_activity > timeout {
            return Err(CoreError::Unauthorized(
                "Sessão expirada por inatividade.".to_string(),
            ));
        }
        Ok(())
    }

    /// Executa o assistente de primeira execução no modo Local:
    /// configura nome da empresa, perfis de sistema, conta do Proprietário e retorna a chave de recuperação única.
    /// Todas as operações de banco e auditoria ocorrem de forma atômica sob a mesma transação.
    pub async fn complete_initial_setup(
        &self,
        tenant_id: &str,
        input: &InitialSetupInput,
        inactivity_timeout_minutes: u32,
    ) -> Result<InitialSetupResult> {
        if input.mode != OperationMode::Local {
            return Err(CoreError::Validation(
                "O modo Nuvem estará disponível em breve (Fase 2.5). Selecione o modo Local."
                    .to_string(),
            ));
        }

        let company_name = BrandingValidator::validate_company_name(&input.company_name)?;
        let owner_name_str = validate_user_name(&input.owner_name)?;
        let owner_email = validate_email(&input.owner_email)?;
        validate_password(&input.owner_password)?;

        let recovery_key = generate_recovery_key();
        let password_hash = hash_secret(&input.owner_password).await?;
        let recovery_key_hash = hash_secret(&normalize_recovery_key(&recovery_key)).await?;

        let tenant_id_str = tenant_id.to_string();
        let company_name_str = company_name.to_string();
        let recovery_key_clone = recovery_key.clone();

        self.run_transaction(|stores| {
            let tenant_id = tenant_id_str.clone();
            let company_name = company_name_str.clone();
            let owner_name = owner_name_str.clone();
            let owner_email = owner_email.clone();
            let password_hash = password_hash.clone();
            let recovery_key_hash = recovery_key_hash.clone();
            let recovery_key = recovery_key_clone.clone();

            Box::pin(async move {
                let existing_users = stores.users().count_by_tenant(&tenant_id).await?;
                if existing_users > 0 {
                    return Err(CoreError::Conflict(
                        "A configuração inicial já foi realizada para este ambiente.".to_string(),
                    ));
                }

                let now = Utc::now().to_rfc3339();
                stores
                    .tenants()
                    .update_name(&tenant_id, &company_name, &now)
                    .await?;

                let current_branding = stores.branding().get_by_tenant_id(&tenant_id).await?;
                let branding_input = UpdateBrandingInput {
                    company_name: company_name.clone(),
                    primary_color: current_branding
                        .as_ref()
                        .map(|b| b.primary_color.clone())
                        .unwrap_or_else(|| "#2563eb".to_string()),
                    secondary_color: current_branding
                        .as_ref()
                        .map(|b| b.secondary_color.clone())
                        .unwrap_or_else(|| "#475569".to_string()),
                    accent_color: current_branding
                        .as_ref()
                        .map(|b| b.accent_color.clone())
                        .unwrap_or_else(|| "#7c3aed".to_string()),
                    logo_url: current_branding.as_ref().and_then(|b| b.logo_url.clone()),
                    logo_dark_url: current_branding
                        .as_ref()
                        .and_then(|b| b.logo_dark_url.clone()),
                    icon_url: current_branding.as_ref().and_then(|b| b.icon_url.clone()),
                };
                let branding = stores.branding().save(&tenant_id, &branding_input).await?;

                ensure_system_roles(&tenant_id, stores.roles(), stores.audit()).await?;

                let owner_role = stores
                    .roles()
                    .find_by_slug(&tenant_id, ROLE_SLUG_OWNER)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Internal("Perfil Proprietário não encontrado.".to_string())
                    })?;

                let owner_id = Uuid::now_v7().to_string();
                let owner_record = UserRecord {
                    id: owner_id.clone(),
                    tenant_id: tenant_id.clone(),
                    name: owner_name,
                    email: owner_email.clone(),
                    password_hash,
                    recovery_key_hash: Some(recovery_key_hash),
                    role_id: owner_role.id.clone(),
                    active: true,
                    last_login_at: Some(now.clone()),
                    failed_attempts: 0,
                    locked_until: None,
                    recovery_failed_attempts: 0,
                    recovery_locked_until: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    deleted_at: None,
                    created_by: Some(owner_id.clone()),
                };

                let created_owner = stores.users().create(&owner_record).await?;

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&created_owner.id),
                        "tenant.setup_completed",
                        "tenant",
                        &tenant_id,
                        &format!(
                            "Configuração inicial concluída para '{company_name}' em modo local"
                        ),
                    ))
                    .await?;

                let masked_owner_email = mask_email_for_audit(&owner_email);
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&created_owner.id),
                        "user.owner_created",
                        "user",
                        &created_owner.id,
                        &format!("Conta de proprietário criada para {masked_owner_email}"),
                    ))
                    .await?;

                let session = SessionInfo {
                    user_id: created_owner.id,
                    tenant_id: tenant_id.clone(),
                    name: created_owner.name,
                    email: created_owner.email,
                    role_id: created_owner.role_id,
                    role_name: created_owner.role_name,
                    role_slug: created_owner.role_slug,
                    permissions: created_owner.permissions,
                    authenticated_at: now.clone(),
                    last_activity_at: now,
                    inactivity_timeout_minutes,
                };

                Ok(InitialSetupResult {
                    tenant_id,
                    recovery_key,
                    session,
                    branding,
                })
            })
        })
        .await
    }

    /// Realiza autenticação por e-mail e senha com proteção Argon2id e bloqueio progressivo.
    /// Em caso de falha, o incremento do contador de tentativas, bloqueio e auditoria são
    /// COMITADOS no banco antes de retornar o erro, garantindo persistência sem rollback da segurança.
    pub async fn login(
        &self,
        tenant_id: &str,
        input: &LoginInput,
        policy: &LoginPolicy,
        inactivity_timeout_minutes: u32,
    ) -> Result<SessionInfo> {
        let email = input.email.trim().to_lowercase();
        if email.is_empty() || input.password.is_empty() {
            let _ = verify_secret(&input.password, DUMMY_ARGON2_HASH).await;
            return Err(CoreError::InvalidCredentials);
        }

        let maybe_user = self.stores.users.find_by_email(tenant_id, &email).await?;
        let user = match maybe_user {
            Some(u) if u.active => u,
            _ => {
                let _ = verify_secret(&input.password, DUMMY_ARGON2_HASH).await;
                return Err(CoreError::InvalidCredentials);
            }
        };

        let now = Utc::now();
        if let Some(ref locked_str) = user.locked_until {
            if let Ok(locked_dt) = DateTime::parse_from_rfc3339(locked_str) {
                if locked_dt.with_timezone(&Utc) > now {
                    if policy.reveal_account_locked {
                        return Err(CoreError::AccountLocked {
                            locked_until: locked_str.clone(),
                        });
                    } else {
                        let _ = verify_secret(&input.password, DUMMY_ARGON2_HASH).await;
                        return Err(CoreError::InvalidCredentials);
                    }
                }
            }
        }

        if !verify_secret(&input.password, &user.password_hash).await? {
            enum LoginFailureOutcome {
                Locked { locked_until: String },
                Failed,
            }

            let tenant_id_str = tenant_id.to_string();
            let user_id_str = user.id.clone();
            let email_str = email.clone();

            let outcome = self
                .run_transaction(|stores| {
                    let tenant_id = tenant_id_str.clone();
                    let user_id = user_id_str.clone();
                    let email = email_str.clone();

                    Box::pin(async move {
                        let now = Utc::now();
                        let now_str = now.to_rfc3339();
                        let new_failures = stores
                            .users()
                            .increment_login_failure(&tenant_id, &user_id, &now_str)
                            .await?;
                        let lockout_secs = calculate_lockout_seconds(new_failures);
                        let locked_until = if lockout_secs > 0 {
                            Some((now + Duration::seconds(lockout_secs)).to_rfc3339())
                        } else {
                            None
                        };

                        if let Some(ref until) = locked_until {
                            stores
                                .users()
                                .set_login_lockout(
                                    &tenant_id,
                                    &user_id,
                                    Some(until.as_str()),
                                    &now_str,
                                )
                                .await?;

                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.account_locked",
                                    "user",
                                    &user_id,
                                    &format!(
                                        "Conta bloqueada temporariamente após {new_failures} tentativas falhas"
                                    ),
                                ))
                                .await?;

                            Ok(LoginFailureOutcome::Locked {
                                locked_until: until.clone(),
                            })
                        } else {
                            let masked_email = mask_email_for_audit(&email);
                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.login_failed",
                                    "user",
                                    &user_id,
                                    &format!("Tentativa de login falha ({new_failures}) para {masked_email}"),
                                ))
                                .await?;

                            Ok(LoginFailureOutcome::Failed)
                        }
                    })
                })
                .await?;

            match outcome {
                LoginFailureOutcome::Locked { locked_until } => {
                    if policy.reveal_account_locked {
                        return Err(CoreError::AccountLocked { locked_until });
                    } else {
                        return Err(CoreError::InvalidCredentials);
                    }
                }
                LoginFailureOutcome::Failed => {
                    return Err(CoreError::InvalidCredentials);
                }
            }
        }

        // Sucesso de autenticação: persiste registro de sucesso e auditoria em transação atômica
        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user.id.clone();
        let user_email_str = user.email.clone();

        let session = self
            .run_transaction(|stores| {
                let tenant_id = tenant_id_str.clone();
                let user_id = user_id_str.clone();
                let user_email = user_email_str.clone();

                Box::pin(async move {
                    let now = Utc::now();
                    let now_str = now.to_rfc3339();
                    stores
                        .users()
                        .record_login_success(&tenant_id, &user_id, &now_str)
                        .await?;

                    let user_with_role = stores
                        .users()
                        .find_with_role(&tenant_id, &user_id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("Usuário autenticado não encontrado.".to_string())
                        })?;

                    let masked_user_email = mask_email_for_audit(&user_email);
                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &tenant_id,
                            Some(&user_id),
                            "auth.login_success",
                            "session",
                            &user_id,
                            &format!("Login realizado com sucesso por {masked_user_email}"),
                        ))
                        .await?;

                    Ok(SessionInfo {
                        user_id: user_with_role.id,
                        tenant_id: user_with_role.tenant_id,
                        name: user_with_role.name,
                        email: user_with_role.email,
                        role_id: user_with_role.role_id,
                        role_name: user_with_role.role_name,
                        role_slug: user_with_role.role_slug,
                        permissions: user_with_role.permissions,
                        authenticated_at: now_str.clone(),
                        last_activity_at: now_str,
                        inactivity_timeout_minutes,
                    })
                })
            })
            .await?;

        Ok(session)
    }

    /// Redefine a senha do proprietário usando a Chave de Recuperação offline.
    /// Em caso de falha de chave, o incremento e bloqueio são persistidos com commit.
    /// Em sucesso, a nova chave rotacionada e a senha são salvas de forma atômica.
    /// Protegido contra enumeração de usuários (timing constante e política de bloqueio).
    pub async fn recover_password(
        &self,
        tenant_id: &str,
        input: &RecoverPasswordInput,
        policy: &LoginPolicy,
    ) -> Result<RecoverPasswordResult> {
        let email = input.email.trim().to_lowercase();
        let normalized_key = normalize_recovery_key(&input.recovery_key);

        validate_password(&input.new_password)?;

        let maybe_user = self.stores.users.find_by_email(tenant_id, &email).await?;
        let user = match maybe_user {
            Some(u) if u.active && u.recovery_key_hash.is_some() => u,
            _ => {
                let _ = verify_secret(&normalized_key, DUMMY_ARGON2_HASH).await;
                return Err(CoreError::InvalidCredentials);
            }
        };

        let key_hash = match user.recovery_key_hash.as_deref() {
            Some(h) => h,
            None => {
                let _ = verify_secret(&normalized_key, DUMMY_ARGON2_HASH).await;
                return Err(CoreError::InvalidCredentials);
            }
        };

        let now = Utc::now();
        if let Some(ref locked_str) = user.recovery_locked_until {
            if let Ok(locked_dt) = DateTime::parse_from_rfc3339(locked_str) {
                if locked_dt.with_timezone(&Utc) > now {
                    if policy.reveal_account_locked {
                        return Err(CoreError::AccountLocked {
                            locked_until: locked_str.clone(),
                        });
                    } else {
                        let _ = verify_secret(&normalized_key, DUMMY_ARGON2_HASH).await;
                        return Err(CoreError::InvalidCredentials);
                    }
                }
            }
        }

        if !verify_secret(&normalized_key, key_hash).await? {
            enum RecoveryFailureOutcome {
                Locked { locked_until: String },
                Failed,
            }

            let tenant_id_str = tenant_id.to_string();
            let user_id_str = user.id.clone();
            let email_str = email.clone();

            let outcome = self
                .run_transaction(|stores| {
                    let tenant_id = tenant_id_str.clone();
                    let user_id = user_id_str.clone();
                    let email = email_str.clone();

                    Box::pin(async move {
                        let now = Utc::now();
                        let now_str = now.to_rfc3339();
                        let new_failures = stores
                            .users()
                            .increment_recovery_failure(&tenant_id, &user_id, &now_str)
                            .await?;
                        let lockout_secs = calculate_lockout_seconds(new_failures);
                        let locked_until = if lockout_secs > 0 {
                            Some((now + Duration::seconds(lockout_secs)).to_rfc3339())
                        } else {
                            None
                        };

                        if let Some(ref until) = locked_until {
                            stores
                                .users()
                                .set_recovery_lockout(
                                    &tenant_id,
                                    &user_id,
                                    Some(until.as_str()),
                                    &now_str,
                                )
                                .await?;

                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.recovery_locked",
                                    "user",
                                    &user_id,
                                    &format!(
                                        "Chave de recuperação bloqueada temporariamente após {new_failures} falhas consecutivas"
                                    ),
                                ))
                                .await?;

                            Ok(RecoveryFailureOutcome::Locked {
                                locked_until: until.clone(),
                            })
                        } else {
                            let masked_email = mask_email_for_audit(&email);
                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.recovery_failed",
                                    "user",
                                    &user_id,
                                    &format!(
                                        "Tentativa inválida de chave de recuperação ({new_failures}) para {masked_email}"
                                    ),
                                ))
                                .await?;

                            Ok(RecoveryFailureOutcome::Failed)
                        }
                    })
                })
                .await?;

            match outcome {
                RecoveryFailureOutcome::Locked { locked_until } => {
                    if policy.reveal_account_locked {
                        return Err(CoreError::AccountLocked { locked_until });
                    } else {
                        return Err(CoreError::InvalidCredentials);
                    }
                }
                RecoveryFailureOutcome::Failed => {
                    return Err(CoreError::InvalidCredentials);
                }
            }
        }

        let new_recovery_key = generate_recovery_key();
        let new_recovery_key_hash = hash_secret(&normalize_recovery_key(&new_recovery_key)).await?;
        let new_password_hash = hash_secret(&input.new_password).await?;

        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user.id.clone();
        let email_str = email.clone();
        let return_key = new_recovery_key.clone();

        self.run_transaction(|stores| {
            let tenant_id = tenant_id_str.clone();
            let user_id = user_id_str.clone();
            let email = email_str.clone();
            let new_password_hash = new_password_hash.clone();
            let new_recovery_key_hash = new_recovery_key_hash.clone();

            Box::pin(async move {
                let now_str = Utc::now().to_rfc3339();
                stores
                    .users()
                    .update_password_and_recovery_key(
                        &tenant_id,
                        &user_id,
                        &new_password_hash,
                        &new_recovery_key_hash,
                        &now_str,
                    )
                    .await?;

                let masked_email = mask_email_for_audit(&email);
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&user_id),
                        "user.password_recovered",
                        "user",
                        &user_id,
                        &format!(
                            "Senha redefinida com sucesso via chave de recuperação para {masked_email}"
                        ),
                    ))
                    .await?;

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&user_id),
                        "auth.recovery_key_rotated",
                        "user",
                        &user_id,
                        &format!(
                            "Chave de recuperação rotacionada com 80 bits de entropia para {masked_email}"
                        ),
                    ))
                    .await?;

                Ok(())
            })
        })
        .await?;

        Ok(RecoverPasswordResult {
            new_recovery_key: return_key,
        })
    }

    pub async fn list_users(&self, session: &SessionInfo) -> Result<Vec<UserWithRole>> {
        self.check_session_and_permission(session, PERM_USER_READ)?;
        self.stores.users.list_by_tenant(&session.tenant_id).await
    }

    pub async fn create_user(
        &self,
        session: &SessionInfo,
        input: &CreateUserInput,
    ) -> Result<UserWithRole> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;

        let name_str = validate_user_name(&input.name)?;
        let email = validate_email(&input.email)?;
        validate_password(&input.password)?;

        let password_hash = hash_secret(&input.password).await?;
        let session = session.clone();
        let role_id = input.role_id.clone();

        self.run_transaction(|stores| {
            let session = session.clone();
            let name = name_str.clone();
            let email = email.clone();
            let password_hash = password_hash.clone();
            let role_id = role_id.clone();

            Box::pin(async move {
                if stores
                    .users()
                    .find_by_email(&session.tenant_id, &email)
                    .await?
                    .is_some()
                {
                    return Err(CoreError::Conflict(
                        "Já existe um usuário ativo com este e-mail.".to_string(),
                    ));
                }

                let role = stores
                    .roles()
                    .find_by_id(&session.tenant_id, &role_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Perfil de acesso não encontrado.".to_string())
                    })?;

                // Anti-escalada (B4): o perfil atribuído deve ter permissões contidas na sessão atual
                for perm in &role.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido atribuir um perfil com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }

                let now = Utc::now().to_rfc3339();
                let record = UserRecord {
                    id: Uuid::now_v7().to_string(),
                    tenant_id: session.tenant_id.clone(),
                    name: name.clone(),
                    email: email.clone(),
                    password_hash,
                    recovery_key_hash: None,
                    role_id: role.id.clone(),
                    active: true,
                    last_login_at: None,
                    failed_attempts: 0,
                    locked_until: None,
                    recovery_failed_attempts: 0,
                    recovery_locked_until: None,
                    created_at: now.clone(),
                    updated_at: now,
                    deleted_at: None,
                    created_by: Some(session.user_id.clone()),
                };

                let created = stores.users().create(&record).await?;

                let masked_name = mask_individual_name_for_audit(&created.name);
                let masked_email = mask_email_for_audit(&created.email);
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "user.created",
                        "user",
                        &created.id,
                        &format!(
                            "Usuário '{}' ({}) criado com perfil '{}'",
                            masked_name, masked_email, role.name
                        ),
                    ))
                    .await?;

                Ok(created)
            })
        })
        .await
    }

    pub async fn update_user(
        &self,
        session: &SessionInfo,
        user_id: &str,
        input: &UpdateUserInput,
    ) -> Result<UserWithRole> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;

        let name_str = validate_user_name(&input.name)?;
        let email = validate_email(&input.email)?;

        let session = session.clone();
        let user_id_str = user_id.to_string();
        let input_role_id = input.role_id.clone();

        self.run_transaction(|stores| {
            let session = session.clone();
            let user_id = user_id_str.clone();
            let name = name_str.clone();
            let email = email.clone();
            let input_role_id = input_role_id.clone();

            Box::pin(async move {
                let target = stores
                    .users()
                    .find_with_role(&session.tenant_id, &user_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

                // Hierarquia de gestão (0.4): o usuário só pode editar alvos cujas permissões sejam subconjunto das suas
                for perm in &target.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido editar um usuário com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }

                if let Some(existing_email_user) = stores
                    .users()
                    .find_by_email(&session.tenant_id, &email)
                    .await?
                {
                    if existing_email_user.id != user_id {
                        return Err(CoreError::Conflict(
                            "Já existe outro usuário com este e-mail.".to_string(),
                        ));
                    }
                }

                let new_role = stores
                    .roles()
                    .find_by_id(&session.tenant_id, &input_role_id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Perfil de acesso não encontrado.".to_string())
                    })?;

                // Anti-escalada (B4):
                for perm in &new_role.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido atribuir um perfil com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }

                if target.role_slug == ROLE_SLUG_OWNER
                    && target.active
                    && new_role.slug != ROLE_SLUG_OWNER
                {
                    let active_owners = stores
                        .users()
                        .count_active_by_role_slug(&session.tenant_id, ROLE_SLUG_OWNER)
                        .await?;
                    if active_owners <= 1 {
                        return Err(CoreError::Validation(
                            "Não é possível alterar o perfil do último Proprietário ativo."
                                .to_string(),
                        ));
                    }
                }

                let now = Utc::now().to_rfc3339();
                let updated = stores
                    .users()
                    .update(
                        &session.tenant_id,
                        &user_id,
                        &name,
                        &email,
                        &new_role.id,
                        &now,
                    )
                    .await?;

                let masked_name = mask_individual_name_for_audit(&updated.name);
                let masked_email = mask_email_for_audit(&updated.email);
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "user.updated",
                        "user",
                        &user_id,
                        &format!(
                            "Usuário '{}' atualizado (e-mail: {}, perfil: {})",
                            masked_name, masked_email, updated.role_name
                        ),
                    ))
                    .await?;

                Ok(updated)
            })
        })
        .await
    }

    pub async fn set_user_active(
        &self,
        session: &SessionInfo,
        user_id: &str,
        active: bool,
    ) -> Result<UserWithRole> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;

        if !active && session.user_id == user_id {
            return Err(CoreError::Validation(
                "Você não pode desativar sua própria conta.".to_string(),
            ));
        }

        let session = session.clone();
        let user_id_str = user_id.to_string();

        self.run_transaction(|stores| {
            let session = session.clone();
            let user_id = user_id_str.clone();

            Box::pin(async move {
                let target = stores
                    .users()
                    .find_with_role(&session.tenant_id, &user_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

                // Hierarquia de gestão (0.4): o usuário só pode desativar/reativar alvos cujas permissões sejam subconjunto das suas
                for perm in &target.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido alterar o status de um usuário com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }

                if !active && target.active && target.role_slug == ROLE_SLUG_OWNER {
                    let active_owners = stores
                        .users()
                        .count_active_by_role_slug(&session.tenant_id, ROLE_SLUG_OWNER)
                        .await?;
                    if active_owners <= 1 {
                        return Err(CoreError::Validation(
                            "Não é possível desativar o último Proprietário ativo.".to_string(),
                        ));
                    }
                }

                let now = Utc::now().to_rfc3339();
                let updated = stores
                    .users()
                    .set_active(&session.tenant_id, &user_id, active, &now)
                    .await?;

                let (action, verb) = if active {
                    ("user.activated", "reativado")
                } else {
                    ("user.deactivated", "desativado")
                };

                let masked_name = mask_individual_name_for_audit(&updated.name);
                let masked_email = mask_email_for_audit(&updated.email);
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        action,
                        "user",
                        &user_id,
                        &format!("Usuário '{}' ({}) {}", masked_name, masked_email, verb),
                    ))
                    .await?;

                Ok(updated)
            })
        })
        .await
    }

    pub async fn reset_user_password(
        &self,
        session: &SessionInfo,
        user_id: &str,
        input: &ResetPasswordInput,
    ) -> Result<()> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;
        validate_password(&input.new_password)?;

        let password_hash = hash_secret(&input.new_password).await?;
        let session = session.clone();
        let user_id_str = user_id.to_string();

        self.run_transaction(|stores| {
            let session = session.clone();
            let user_id = user_id_str.clone();
            let password_hash = password_hash.clone();

            Box::pin(async move {
                let target = stores
                    .users()
                    .find_with_role(&session.tenant_id, &user_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

                // Hierarquia de gestão (0.4): o usuário só pode redefinir a senha de alvos cujas permissões sejam subconjunto das suas
                for perm in &target.permissions {
                    if !session.has_permission(perm) {
                        return Err(CoreError::Forbidden(format!(
                            "Não é permitido redefinir a senha de um usuário com permissões superiores às suas: '{perm}'"
                        )));
                    }
                }

                let now = Utc::now().to_rfc3339();
                stores
                    .users()
                    .update_password(&session.tenant_id, &user_id, &password_hash, &now)
                    .await?;

                let masked_email = mask_email_for_audit(&target.email);
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "user.password_reset",
                        "user",
                        &user_id,
                        &format!("Senha redefinida administrativamente para {masked_email}"),
                    ))
                    .await?;

                Ok(())
            })
        })
        .await
    }

    pub async fn list_roles(&self, session: &SessionInfo) -> Result<Vec<RoleWithPermissions>> {
        self.verify_session_active(session)?;
        if !session.has_permission(PERM_ROLE_READ) && !session.has_permission(PERM_USER_READ) {
            return Err(CoreError::Forbidden(
                "Permissão insuficiente: requer 'role:read' ou 'user:read'.".to_string(),
            ));
        }
        self.stores.roles.list_by_tenant(&session.tenant_id).await
    }

    pub async fn get_branding(&self, session: &SessionInfo) -> Result<Option<TenantBranding>> {
        self.check_session_and_permission(session, PERM_BRANDING_READ)?;
        self.stores
            .branding
            .get_by_tenant_id(&session.tenant_id)
            .await
    }

    pub async fn save_branding(
        &self,
        session: &SessionInfo,
        input: &UpdateBrandingInput,
    ) -> Result<TenantBranding> {
        self.check_session_and_permission(session, PERM_BRANDING_MANAGE)?;
        let validated = BrandingValidator::validate_input(input)?;
        let session = session.clone();

        self.run_transaction(|stores| {
            let session = session.clone();
            let validated = validated.clone();

            Box::pin(async move {
                let updated = stores.branding().save(&session.tenant_id, &validated).await?;

                let audit_detail = format!(
                    "Identidade visual atualizada para '{}' (primária: {}, secundária: {}, destaque: {}; logo_claro: {}, logo_escuro: {}, icone: {})",
                    updated.company_name,
                    updated.primary_color,
                    updated.secondary_color,
                    updated.accent_color,
                    updated.logo_url.is_some(),
                    updated.logo_dark_url.is_some(),
                    updated.icon_url.is_some()
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "branding.updated",
                        "tenant_branding",
                        &updated.id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(updated)
            })
        })
        .await
    }

    /// Obtém a identidade visual pública de um tenant para telas pré-login (/login, /setup).
    /// Devolve um DTO com lista fechada de campos (nome, cores, logos), sem ids internos.
    pub async fn get_public_branding(&self, tenant_id: &str) -> Result<Option<PublicBranding>> {
        let branding = self.stores.branding.get_by_tenant_id(tenant_id).await?;
        Ok(branding.map(PublicBranding::from))
    }

    /// Verifica se o tenant já possui ao menos um usuário registrado (ex.: proprietário).
    pub async fn has_owner(&self, tenant_id: &str) -> Result<bool> {
        let count = self.stores.users.count_by_tenant(tenant_id).await?;
        Ok(count > 0)
    }

    /// Obtém as configurações de identidade visual do tenant para verificação de status do app.
    pub async fn get_tenant_branding(&self, tenant_id: &str) -> Result<Option<TenantBranding>> {
        self.stores.branding.get_by_tenant_id(tenant_id).await
    }

    // =========================================================================
    // SERVIÇOS DE GESTÃO DE CLIENTES (FASE 1B.2)
    // =========================================================================

    /// Lista clientes/fornecedores com suporte a filtro textual, tipo de pessoa, papel e paginação por cursor.
    /// Requer `customer:read` e/ou `supplier:read` conforme o filtro de papel solicitado.
    pub async fn list_customers(
        &self,
        session: &SessionInfo,
        filter: &CustomerFilter,
    ) -> Result<CustomerPage> {
        self.verify_session_active(session)?;

        let can_read_customer = session.has_permission(PERM_CUSTOMER_READ);
        let can_read_supplier = session.has_permission(PERM_SUPPLIER_READ);
        if !can_read_customer && !can_read_supplier {
            return Err(CoreError::Forbidden(
                "Acesso negado: permissão 'customer:read' ou 'supplier:read' necessária."
                    .to_string(),
            ));
        }

        let effective_role_filter = match filter.role_filter {
            CustomerRoleFilter::Customer => {
                if !can_read_customer {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'customer:read' necessária.".to_string(),
                    ));
                }
                CustomerRoleFilter::Customer
            }
            CustomerRoleFilter::Supplier => {
                if !can_read_supplier {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'supplier:read' necessária.".to_string(),
                    ));
                }
                CustomerRoleFilter::Supplier
            }
            CustomerRoleFilter::Both => CustomerRoleFilter::Both,
            CustomerRoleFilter::All => {
                if can_read_customer && can_read_supplier {
                    CustomerRoleFilter::All
                } else if can_read_customer {
                    CustomerRoleFilter::Customer
                } else {
                    CustomerRoleFilter::Supplier
                }
            }
        };

        if let Some(ref cursor) = filter.cursor {
            CustomerCursor::decode(cursor)?;
        }

        validate_search_query_limits(filter.query.as_deref())?;

        let sanitized_filter = CustomerFilter {
            query: filter.query.clone(),
            person_type: filter.person_type,
            role_filter: effective_role_filter,
            cursor: filter.cursor.clone(),
            limit: Some(filter.limit.unwrap_or(50).clamp(1, 100)),
        };

        self.stores
            .customers
            .list(&session.tenant_id, &sanitized_filter)
            .await
    }

    /// Obtém detalhes completos de um terceiro (cliente/fornecedor).
    /// Um cadastro com os dois papéis é visível a quem tiver `customer:read` OU `supplier:read`.
    pub async fn get_customer(
        &self,
        session: &SessionInfo,
        id: &str,
    ) -> Result<CustomerWithDetails> {
        self.verify_session_active(session)?;
        let can_read_customer = session.has_permission(PERM_CUSTOMER_READ);
        let can_read_supplier = session.has_permission(PERM_SUPPLIER_READ);
        if !can_read_customer && !can_read_supplier {
            return Err(CoreError::Forbidden(
                "Acesso negado: permissão 'customer:read' ou 'supplier:read' necessária."
                    .to_string(),
            ));
        }

        let details = self
            .stores
            .customers
            .find_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Cadastro não encontrado.".to_string()))?;

        let visible = (details.customer.is_customer && can_read_customer)
            || (details.customer.is_supplier && can_read_supplier);
        if !visible {
            return Err(CoreError::Forbidden(
                "Acesso negado para visualizar este cadastro.".to_string(),
            ));
        }
        Ok(details)
    }

    /// Cria um novo terceiro (cliente e/ou fornecedor) com validação estrita, contatos e endereços.
    /// Exige `manage` de CADA papel marcado (`customer:manage` e/ou `supplier:manage`).
    pub async fn create_customer(
        &self,
        session: &SessionInfo,
        input: &CreateCustomerInput,
    ) -> Result<CustomerWithDetails> {
        self.verify_session_active(session)?;
        if input.is_customer {
            self.check_session_and_permission(session, PERM_CUSTOMER_MANAGE)?;
        }
        if input.is_supplier {
            self.check_session_and_permission(session, PERM_SUPPLIER_MANAGE)?;
        }

        let supplier_categories = validate_roles_and_categories(
            input.is_customer,
            input.is_supplier,
            &input.supplier_categories,
        )?;

        let name = input.name.trim();
        validate_customer_common_fields(
            name,
            input.trade_name.as_deref(),
            input.notes.as_deref(),
            input.state_registration.as_deref(),
            input.municipal_registration.as_deref(),
        )?;

        // Validação estrita de documento conforme tipo (cpf, cnpj, foreign ou none)
        let (doc_type, doc_raw_opt, doc_formatted) = validate_customer_document_for_person_type(
            input.person_type,
            input.document_type,
            input.document_number.as_deref(),
        )?;

        // Validação no core de contatos e endereços (contato obrigatório >= 1, endereço opcional >= 0)
        // Em criação, IDs enviados pelo cliente são sempre ignorados e gerados do zero no servidor.
        let mut contacts_input = input.contacts.clone();
        for c in &mut contacts_input {
            c.id = None;
        }
        if contacts_input.is_empty() {
            return Err(CoreError::Validation(
                "Cliente deve possuir pelo menos um contato.".to_string(),
            ));
        }
        validate_contacts(&mut contacts_input)?;

        let mut addresses_input = input.addresses.clone();
        for a in &mut addresses_input {
            a.id = None;
        }
        validate_addresses(&mut addresses_input)?;

        let customer_id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();

        let contacts: Vec<CustomerContact> = contacts_input
            .into_iter()
            .map(|c| CustomerContact {
                id: Uuid::now_v7().to_string(),
                tenant_id: session.tenant_id.clone(),
                customer_id: customer_id.clone(),
                name: c.name.trim().to_string(),
                email: c.email.as_ref().map(|e| e.trim().to_string()),
                phone: c.phone.as_ref().map(|p| p.trim().to_string()),
                role: c.role.as_ref().map(|r| r.trim().to_string()),
                is_primary: c.is_primary,
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
                created_by: Some(session.user_id.clone()),
            })
            .collect();

        let addresses: Vec<CustomerAddress> = addresses_input
            .into_iter()
            .map(|a| CustomerAddress {
                id: Uuid::now_v7().to_string(),
                tenant_id: session.tenant_id.clone(),
                customer_id: customer_id.clone(),
                label: if a.label.trim().is_empty() {
                    "Principal".to_string()
                } else {
                    a.label.trim().to_string()
                },
                street: a.street.trim().to_string(),
                number: a.number.trim().to_string(),
                complement: a.complement.as_ref().map(|c| c.trim().to_string()),
                neighborhood: a.neighborhood.trim().to_string(),
                city: a.city.trim().to_string(),
                state: a.state.trim().to_uppercase(),
                postal_code: a.postal_code.trim().to_string(),
                country: a.country.unwrap_or_else(|| "BR".to_string()),
                is_primary: a.is_primary,
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
                created_by: Some(session.user_id.clone()),
            })
            .collect();

        // Constrói termos de busca incluindo nomes dos contatos e telefones só dígitos
        let mut search_terms = Vec::new();
        for c in &contacts {
            search_terms.push(c.name.clone());
            if let Some(ref e) = c.email {
                search_terms.push(e.clone());
            }
            if let Some(ref p) = c.phone {
                search_terms.push(p.clone());
                let digits: String = p.chars().filter(|ch| ch.is_ascii_digit()).collect();
                if !digits.is_empty() {
                    search_terms.push(digits);
                }
            }
        }
        for a in &addresses {
            search_terms.push(a.city.clone());
            search_terms.push(a.neighborhood.clone());
        }

        let trade_trimmed = input
            .trade_name
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let search_text = build_customer_search_text(
            name,
            trade_trimmed.as_deref(),
            doc_raw_opt.as_deref().unwrap_or(""),
            &doc_formatted,
            &search_terms,
        );
        let name_sort = build_name_sort(name);

        let customer = Customer {
            id: customer_id.clone(),
            tenant_id: session.tenant_id.clone(),
            person_type: input.person_type,
            name: name.to_string(),
            trade_name: trade_trimmed,
            document_type: doc_type,
            document_number: doc_raw_opt.clone(),
            state_registration: input
                .state_registration
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            municipal_registration: input
                .municipal_registration
                .as_ref()
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty()),
            notes: input
                .notes
                .as_ref()
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty()),
            active: true,
            is_customer: input.is_customer,
            is_supplier: input.is_supplier,
            search_text,
            name_sort,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let session_clone = session.clone();
        let customer_clone = customer.clone();
        let contacts_clone = contacts.clone();
        let addresses_clone = addresses.clone();
        let categories_clone = supplier_categories.clone();
        let doc_raw_clone = doc_raw_opt.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let customer = customer_clone.clone();
            let contacts = contacts_clone.clone();
            let addresses = addresses_clone.clone();
            let categories = categories_clone.clone();
            let doc_raw = doc_raw_clone.clone();

            Box::pin(async move {
                // Leitura DENTRO da transação: unicidade do documento ativo ou inativo
                if let Some(ref raw) = doc_raw {
                    if let Some(existing_doc) = stores
                        .customers()
                        .find_by_document(&session.tenant_id, customer.document_type, raw)
                        .await?
                    {
                        return Err(duplicate_document_conflict_error(&session, &existing_doc));
                    }
                }

                stores
                    .customers()
                    .create(&customer, &contacts, &addresses, &categories)
                    .await?;

                let masked_audit_name = if customer.person_type == PersonType::Individual {
                    mask_individual_name_for_audit(&customer.name)
                } else {
                    customer.name.clone()
                };
                let role_desc = match (customer.is_customer, customer.is_supplier) {
                    (true, true) => "Cliente/Fornecedor",
                    (true, false) => "Cliente",
                    (false, true) => "Fornecedor",
                    (false, false) => "Terceiro",
                };
                let audit_detail = format!(
                    "{} '{}' criado com sucesso (id: {})",
                    role_desc, masked_audit_name, customer.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "customer.created",
                        "customer",
                        &customer.id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(())
            })
        })
        .await?;

        Ok(CustomerWithDetails {
            customer,
            contacts,
            addresses,
            supplier_categories,
        })
    }

    /// Atualiza os dados de um terceiro (cliente/fornecedor) existente com validações completas e auditoria mascarada (LGPD).
    /// Exige `manage` de cada papel marcado na entrada e de cada papel removido.
    pub async fn update_customer(
        &self,
        session: &SessionInfo,
        id: &str,
        input: &UpdateCustomerInput,
    ) -> Result<CustomerWithDetails> {
        self.verify_session_active(session)?;
        if input.is_customer {
            self.check_session_and_permission(session, PERM_CUSTOMER_MANAGE)?;
        }
        if input.is_supplier {
            self.check_session_and_permission(session, PERM_SUPPLIER_MANAGE)?;
        }

        let supplier_categories = validate_roles_and_categories(
            input.is_customer,
            input.is_supplier,
            &input.supplier_categories,
        )?;

        let name = input.name.trim();
        validate_customer_common_fields(
            name,
            input.trade_name.as_deref(),
            input.notes.as_deref(),
            input.state_registration.as_deref(),
            input.municipal_registration.as_deref(),
        )?;

        let (doc_type, doc_raw_opt, doc_formatted) = validate_customer_document_for_person_type(
            input.person_type,
            input.document_type,
            input.document_number.as_deref(),
        )?;

        let mut contacts_input = input.contacts.clone();
        if contacts_input.is_empty() {
            return Err(CoreError::Validation(
                "Cliente deve possuir pelo menos um contato.".to_string(),
            ));
        }
        validate_contacts(&mut contacts_input)?;

        let mut addresses_input = input.addresses.clone();
        validate_addresses(&mut addresses_input)?;

        let now = Utc::now().to_rfc3339();

        let session_clone = session.clone();
        let id_str = id.to_string();
        let name_str = name.to_string();
        let doc_raw_clone = doc_raw_opt.clone();
        let input_clone = input.clone();
        let now_clone = now.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let name = name_str.clone();
            let doc_raw_opt = doc_raw_clone.clone();
            let contacts_input = contacts_input.clone();
            let addresses_input = addresses_input.clone();
            let doc_formatted = doc_formatted.clone();
            let input = input_clone.clone();
            let now = now_clone.clone();
            let categories = supplier_categories.clone();

            Box::pin(async move {
                // 1. Leitura DENTRO da transação: cliente existente
                let existing = stores
                    .customers()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Cadastro não encontrado.".to_string()))?;

                if !can_read_party(&session, &existing.customer) {
                    return Err(CoreError::NotFound("Cadastro não encontrado.".to_string()));
                }

                // Remover um papel exige manage desse papel
                if existing.customer.is_customer
                    && !input.is_customer
                    && !session.has_permission(PERM_CUSTOMER_MANAGE)
                {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'customer:manage' necessária para remover o papel Cliente."
                            .to_string(),
                    ));
                }
                if existing.customer.is_supplier
                    && !input.is_supplier
                    && !session.has_permission(PERM_SUPPLIER_MANAGE)
                {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'supplier:manage' necessária para remover o papel Fornecedor."
                            .to_string(),
                    ));
                }

                // 2. Controle de concorrência otimista (expected_updated_at)
                if existing.customer.updated_at != input.expected_updated_at {
                    return Err(CoreError::Conflict(
                        "O cadastro foi alterado por outra pessoa. Recarregue e tente novamente."
                            .to_string(),
                    ));
                }

                // 3. Leitura DENTRO da transação: checa duplicidade de documento se alterado
                if let Some(ref doc_raw) = doc_raw_opt {
                    if Some(doc_raw.as_str()) != existing.customer.document_number.as_deref()
                        || doc_type != existing.customer.document_type
                    {
                        if let Some(other) = stores
                            .customers()
                            .find_by_document(&session.tenant_id, doc_type, doc_raw)
                            .await?
                        {
                            if other.id != id {
                                return Err(duplicate_document_conflict_error(&session, &other));
                            }
                        }
                    }
                }

                let existing_contacts: std::collections::HashMap<&str, &CustomerContact> = existing
                    .contacts
                    .iter()
                    .map(|c| (c.id.as_str(), c))
                    .collect();
                let contacts: Vec<CustomerContact> = contacts_input
                    .into_iter()
                    .map(|c| {
                        let prev = c
                            .id
                            .as_deref()
                            .and_then(|cid| existing_contacts.get(cid).copied());
                        CustomerContact {
                            id: prev
                                .map(|p| p.id.clone())
                                .unwrap_or_else(|| Uuid::now_v7().to_string()),
                            tenant_id: session.tenant_id.clone(),
                            customer_id: id.clone(),
                            name: c.name.trim().to_string(),
                            email: c.email.as_ref().map(|e| e.trim().to_string()),
                            phone: c.phone.as_ref().map(|p| p.trim().to_string()),
                            role: c.role.as_ref().map(|r| r.trim().to_string()),
                            is_primary: c.is_primary,
                            created_at: prev
                                .map(|p| p.created_at.clone())
                                .unwrap_or_else(|| now.clone()),
                            updated_at: now.clone(),
                            deleted_at: None,
                            created_by: prev
                                .and_then(|p| p.created_by.clone())
                                .or_else(|| Some(session.user_id.clone())),
                        }
                    })
                    .collect();

                let existing_addresses: std::collections::HashMap<&str, &CustomerAddress> = existing
                    .addresses
                    .iter()
                    .map(|a| (a.id.as_str(), a))
                    .collect();
                let addresses: Vec<CustomerAddress> = addresses_input
                    .into_iter()
                    .map(|a| {
                        let prev = a
                            .id
                            .as_deref()
                            .and_then(|aid| existing_addresses.get(aid).copied());
                        CustomerAddress {
                            id: prev
                                .map(|p| p.id.clone())
                                .unwrap_or_else(|| Uuid::now_v7().to_string()),
                            tenant_id: session.tenant_id.clone(),
                            customer_id: id.clone(),
                            label: if a.label.trim().is_empty() {
                                "Principal".to_string()
                            } else {
                                a.label.trim().to_string()
                            },
                            street: a.street.trim().to_string(),
                            number: a.number.trim().to_string(),
                            complement: a.complement.as_ref().map(|c| c.trim().to_string()),
                            neighborhood: a.neighborhood.trim().to_string(),
                            city: a.city.trim().to_string(),
                            state: a.state.trim().to_uppercase(),
                            postal_code: a.postal_code.trim().to_string(),
                            country: a.country.unwrap_or_else(|| "BR".to_string()),
                            is_primary: a.is_primary,
                            created_at: prev
                                .map(|p| p.created_at.clone())
                                .unwrap_or_else(|| now.clone()),
                            updated_at: now.clone(),
                            deleted_at: None,
                            created_by: prev
                                .and_then(|p| p.created_by.clone())
                                .or_else(|| Some(session.user_id.clone())),
                        }
                    })
                    .collect();

                let mut search_terms = Vec::new();
                for c in &contacts {
                    search_terms.push(c.name.clone());
                    if let Some(ref e) = c.email {
                        search_terms.push(e.clone());
                    }
                    if let Some(ref p) = c.phone {
                        search_terms.push(p.clone());
                        let digits: String = p.chars().filter(|ch| ch.is_ascii_digit()).collect();
                        if !digits.is_empty() {
                            search_terms.push(digits);
                        }
                    }
                }
                for a in &addresses {
                    search_terms.push(a.city.clone());
                    search_terms.push(a.neighborhood.clone());
                }

                let trade_trimmed = input
                    .trade_name
                    .as_ref()
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty());
                let search_text = build_customer_search_text(
                    &name,
                    trade_trimmed.as_deref(),
                    doc_raw_opt.as_deref().unwrap_or(""),
                    &doc_formatted,
                    &search_terms,
                );
                let name_sort = build_name_sort(&name);

                let updated_customer = Customer {
                    id: id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    person_type: input.person_type,
                    name: name.clone(),
                    trade_name: trade_trimmed,
                    document_type: doc_type,
                    document_number: doc_raw_opt.clone(),
                    state_registration: input
                        .state_registration
                        .as_ref()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty()),
                    municipal_registration: input
                        .municipal_registration
                        .as_ref()
                        .map(|m| m.trim().to_string())
                        .filter(|m| !m.is_empty()),
                    notes: input
                        .notes
                        .as_ref()
                        .map(|n| n.trim().to_string())
                        .filter(|n| !n.is_empty()),
                    active: existing.customer.active,
                    is_customer: input.is_customer,
                    is_supplier: input.is_supplier,
                    search_text,
                    name_sort,
                    created_at: existing.customer.created_at,
                    updated_at: now,
                    deleted_at: None,
                    created_by: existing.customer.created_by,
                };

                stores
                    .customers()
                    .update(&updated_customer, &contacts, &addresses, &categories)
                    .await?;

                let masked_audit_name = if updated_customer.person_type == PersonType::Individual {
                    mask_individual_name_for_audit(&updated_customer.name)
                } else {
                    updated_customer.name.clone()
                };
                let role_desc = match (updated_customer.is_customer, updated_customer.is_supplier) {
                    (true, true) => "Cliente/Fornecedor",
                    (true, false) => "Cliente",
                    (false, true) => "Fornecedor",
                    (false, false) => "Terceiro",
                };
                let audit_detail = format!(
                    "{} '{}' atualizado com sucesso (id: {})",
                    role_desc, masked_audit_name, updated_customer.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "customer.updated",
                        "customer",
                        &updated_customer.id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(CustomerWithDetails {
                    customer: updated_customer,
                    contacts,
                    addresses,
                    supplier_categories: categories,
                })
            })
        })
        .await
    }

    /// Ativa ou desativa um cliente/fornecedor existente sem removê-lo do histórico.
    /// Exige `manage` de cada papel ativo no cadastro.
    pub async fn set_customer_active(
        &self,
        session: &SessionInfo,
        id: &str,
        active: bool,
    ) -> Result<CustomerWithDetails> {
        self.verify_session_active(session)?;
        if !session.has_permission(PERM_CUSTOMER_MANAGE)
            && !session.has_permission(PERM_SUPPLIER_MANAGE)
        {
            return Err(CoreError::Forbidden(
                "Acesso negado: permissão 'customer:manage' ou 'supplier:manage' necessária."
                    .to_string(),
            ));
        }

        let session_clone = session.clone();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();

            Box::pin(async move {
                // Leitura DENTRO da transação
                let mut existing = stores
                    .customers()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Cadastro não encontrado.".to_string()))?;

                if !can_read_party(&session, &existing.customer) {
                    return Err(CoreError::NotFound("Cadastro não encontrado.".to_string()));
                }

                if existing.customer.is_customer && !session.has_permission(PERM_CUSTOMER_MANAGE) {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'customer:manage' necessária.".to_string(),
                    ));
                }
                if existing.customer.is_supplier && !session.has_permission(PERM_SUPPLIER_MANAGE) {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'supplier:manage' necessária.".to_string(),
                    ));
                }

                if existing.customer.active == active {
                    return Ok(existing);
                }

                stores
                    .customers()
                    .set_active(&session.tenant_id, &id, active, &now)
                    .await?;

                existing.customer.active = active;
                existing.customer.updated_at = now;

                let action = if active {
                    "customer.activated"
                } else {
                    "customer.deactivated"
                };
                let status_label = if active { "ativado" } else { "desativado" };
                let masked_audit_name = if existing.customer.person_type == PersonType::Individual {
                    mask_individual_name_for_audit(&existing.customer.name)
                } else {
                    existing.customer.name.clone()
                };
                let role_label =
                    match (existing.customer.is_customer, existing.customer.is_supplier) {
                        (true, true) => "Cliente/Fornecedor",
                        (false, true) => "Fornecedor",
                        _ => "Cliente",
                    };
                let audit_detail = format!(
                    "{} '{}' {} com sucesso (id: {})",
                    role_label, masked_audit_name, status_label, existing.customer.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        action,
                        "customer",
                        &id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(existing)
            })
        })
        .await
    }

    /// Realiza exclusão lógica (soft delete) do terceiro e seus contatos/endereços.
    /// Exige `manage` de cada papel ativo no cadastro.
    pub async fn delete_customer(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.verify_session_active(session)?;
        if !session.has_permission(PERM_CUSTOMER_MANAGE)
            && !session.has_permission(PERM_SUPPLIER_MANAGE)
        {
            return Err(CoreError::Forbidden(
                "Acesso negado: permissão 'customer:manage' ou 'supplier:manage' necessária."
                    .to_string(),
            ));
        }

        let session_clone = session.clone();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();

            Box::pin(async move {
                // Leitura DENTRO da transação
                let existing = stores
                    .customers()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Cadastro não encontrado.".to_string()))?;

                if !can_read_party(&session, &existing.customer) {
                    return Err(CoreError::NotFound("Cadastro não encontrado.".to_string()));
                }

                if existing.customer.is_customer && !session.has_permission(PERM_CUSTOMER_MANAGE) {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'customer:manage' necessária.".to_string(),
                    ));
                }
                if existing.customer.is_supplier && !session.has_permission(PERM_SUPPLIER_MANAGE) {
                    return Err(CoreError::Forbidden(
                        "Acesso negado: permissão 'supplier:manage' necessária.".to_string(),
                    ));
                }

                stores
                    .customers()
                    .soft_delete(&session.tenant_id, &id, &now)
                    .await?;

                let masked_audit_name = if existing.customer.person_type == PersonType::Individual {
                    mask_individual_name_for_audit(&existing.customer.name)
                } else {
                    existing.customer.name.clone()
                };
                let role_label =
                    match (existing.customer.is_customer, existing.customer.is_supplier) {
                        (true, true) => "Cliente/Fornecedor",
                        (false, true) => "Fornecedor",
                        _ => "Cliente",
                    };
                let audit_detail = format!(
                    "{} '{}' excluído logicamente (soft delete, id: {})",
                    role_label, masked_audit_name, existing.customer.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "customer.deleted",
                        "customer",
                        &id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(())
            })
        })
        .await
    }

    /// Ativa um papel (`Customer` ou `Supplier`) em um cadastro de terceiro já existente.
    /// Exige `manage` SOMENTE do papel que está sendo adicionado.
    pub async fn add_customer_role(
        &self,
        session: &SessionInfo,
        id: &str,
        role: CustomerRole,
    ) -> Result<CustomerWithDetails> {
        let required_perm = match role {
            CustomerRole::Customer => PERM_CUSTOMER_MANAGE,
            CustomerRole::Supplier => PERM_SUPPLIER_MANAGE,
        };
        self.check_session_and_permission(session, required_perm)?;

        let session_clone = session.clone();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();

            Box::pin(async move {
                let mut existing = stores
                    .customers()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("Cadastro não encontrado.".to_string()))?;

                if !can_read_party(&session, &existing.customer) {
                    return Err(CoreError::NotFound("Cadastro não encontrado.".to_string()));
                }

                let already_has_role = match role {
                    CustomerRole::Customer => existing.customer.is_customer,
                    CustomerRole::Supplier => existing.customer.is_supplier,
                };
                if already_has_role {
                    return Ok(existing);
                }

                match role {
                    CustomerRole::Customer => existing.customer.is_customer = true,
                    CustomerRole::Supplier => existing.customer.is_supplier = true,
                }
                existing.customer.updated_at = now;

                stores
                    .customers()
                    .update(
                        &existing.customer,
                        &existing.contacts,
                        &existing.addresses,
                        &existing.supplier_categories,
                    )
                    .await?;

                let masked_audit_name = if existing.customer.person_type == PersonType::Individual {
                    mask_individual_name_for_audit(&existing.customer.name)
                } else {
                    existing.customer.name.clone()
                };
                let added_label = match role {
                    CustomerRole::Customer => "Cliente",
                    CustomerRole::Supplier => "Fornecedor",
                };
                let audit_detail = format!(
                    "Papel '{added_label}' adicionado ao cadastro '{}' (id: {})",
                    masked_audit_name, existing.customer.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "customer.role_added",
                        "customer",
                        &existing.customer.id,
                        &audit_detail,
                    ))
                    .await?;

                Ok(existing)
            })
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Locais de Evento, Espaços e Contatos (Fase 1D)
    // -----------------------------------------------------------------------

    pub async fn create_venue(
        &self,
        session: &SessionInfo,
        input: &CreateVenueInput,
    ) -> Result<VenueWithDetails> {
        self.check_session_and_permission(session, PERM_VENUE_MANAGE)?;

        let fields = validate_venue_fields(
            &input.name,
            input.street.clone(),
            input.number.clone(),
            input.complement.clone(),
            input.neighborhood.clone(),
            input.city.clone(),
            input.state.clone(),
            input.postal_code.clone(),
            &input.country,
            input.parking_notes.clone(),
            input.loading_dock_notes.clone(),
            input.access_hours.clone(),
            input.internal_transfer_minutes,
            input.notes.clone(),
        )?;

        let mut space_inputs = input.spaces.clone();
        validate_venue_spaces(&mut space_inputs)?;

        let mut contact_inputs = input.contacts.clone();
        validate_venue_contacts(&mut contact_inputs, &fields.country)?;

        let now = Utc::now().to_rfc3339();
        let venue_id = Uuid::now_v7().to_string();
        let search_text = build_venue_search_text(&fields, &space_inputs, &contact_inputs);
        let name_sort = build_venue_name_sort(&fields.name);

        let venue = Venue {
            id: venue_id.clone(),
            tenant_id: session.tenant_id.clone(),
            name: fields.name,
            street: fields.street,
            number: fields.number,
            complement: fields.complement,
            neighborhood: fields.neighborhood,
            city: fields.city,
            state: fields.state,
            postal_code: fields.postal_code,
            country: fields.country,
            parking_notes: fields.parking_notes,
            loading_dock_notes: fields.loading_dock_notes,
            access_hours: fields.access_hours,
            internal_transfer_minutes: fields.internal_transfer_minutes,
            notes: fields.notes,
            active: true,
            search_text,
            name_sort,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
            created_by: Some(session.user_id.clone()),
        };

        let spaces: Vec<VenueSpace> = space_inputs
            .into_iter()
            .enumerate()
            .map(|(idx, sp)| VenueSpace {
                // Ignora qualquer ID enviado na criação para evitar enumeração/colisão
                id: Uuid::now_v7().to_string(),
                tenant_id: session.tenant_id.clone(),
                venue_id: venue_id.clone(),
                name_normalized: normalize_for_search(&sp.name),
                name: sp.name,
                capacity_people: sp.capacity_people,
                area_sqm: sp.area_sqm,
                ceiling_height_m: sp.ceiling_height_m,
                power_available: sp.power_available,
                loading_access: sp.loading_access,
                rigging_points: sp.rigging_points,
                acoustics_notes: sp.acoustics_notes,
                setup_hours_notes: sp.setup_hours_notes,
                noise_restrictions_notes: sp.noise_restrictions_notes,
                load_dimensions_notes: sp.load_dimensions_notes,
                notes: sp.notes,
                active: sp.active,
                sort_order: if sp.sort_order == 0 {
                    idx as i64
                } else {
                    sp.sort_order
                },
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
                created_by: Some(session.user_id.clone()),
            })
            .collect();

        let contacts: Vec<VenueContact> = contact_inputs
            .into_iter()
            .map(|ct| VenueContact {
                // Ignora qualquer ID enviado na criação
                id: Uuid::now_v7().to_string(),
                tenant_id: session.tenant_id.clone(),
                venue_id: venue_id.clone(),
                name: ct.name,
                email: ct.email,
                phone: ct.phone,
                role: ct.role,
                is_primary: ct.is_primary,
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
                created_by: Some(session.user_id.clone()),
            })
            .collect();

        let session_clone = session.clone();
        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let venue = venue.clone();
            let spaces = spaces.clone();
            let contacts = contacts.clone();

            Box::pin(async move {
                stores.venues().create(&venue, &spaces, &contacts).await?;
                let detail = format!(
                    "Local de evento '{}' criado com {} espaço(s) e {} contato(s) (id: {})",
                    venue.name,
                    spaces.len(),
                    contacts.len(),
                    venue.id
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "venue.created",
                        "venue",
                        &venue.id,
                        &detail,
                    ))
                    .await?;
                Ok(VenueWithDetails {
                    venue,
                    spaces,
                    contacts,
                })
            })
        })
        .await
    }

    pub async fn update_venue(
        &self,
        session: &SessionInfo,
        input: &UpdateVenueInput,
    ) -> Result<VenueWithDetails> {
        self.check_session_and_permission(session, PERM_VENUE_MANAGE)?;

        if input.expected_updated_at.trim().is_empty() {
            return Err(CoreError::Validation(
                "expected_updated_at é obrigatório para atualização concorrente.".to_string(),
            ));
        }

        let fields = validate_venue_fields(
            &input.name,
            input.street.clone(),
            input.number.clone(),
            input.complement.clone(),
            input.neighborhood.clone(),
            input.city.clone(),
            input.state.clone(),
            input.postal_code.clone(),
            &input.country,
            input.parking_notes.clone(),
            input.loading_dock_notes.clone(),
            input.access_hours.clone(),
            input.internal_transfer_minutes,
            input.notes.clone(),
        )?;

        let mut space_inputs = input.spaces.clone();
        validate_venue_spaces(&mut space_inputs)?;

        let mut contact_inputs = input.contacts.clone();
        validate_venue_contacts(&mut contact_inputs, &fields.country)?;

        let now = Utc::now().to_rfc3339();
        let search_text = build_venue_search_text(&fields, &space_inputs, &contact_inputs);
        let name_sort = build_venue_name_sort(&fields.name);

        let session_clone = session.clone();
        let id_str = input.id.clone();
        let expected_updated_at = input.expected_updated_at.clone();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let expected_updated_at = expected_updated_at.clone();
            let now = now.clone();
            let name = fields.name.clone();
            let street = fields.street.clone();
            let number = fields.number.clone();
            let complement = fields.complement.clone();
            let neighborhood = fields.neighborhood.clone();
            let city = fields.city.clone();
            let state = fields.state.clone();
            let postal_code = fields.postal_code.clone();
            let country = fields.country.clone();
            let parking_notes = fields.parking_notes.clone();
            let loading_dock_notes = fields.loading_dock_notes.clone();
            let access_hours = fields.access_hours.clone();
            let internal_transfer_minutes = fields.internal_transfer_minutes;
            let notes = fields.notes.clone();
            let search_text = search_text.clone();
            let name_sort = name_sort.clone();
            let space_inputs = space_inputs.clone();
            let contact_inputs = contact_inputs.clone();

            Box::pin(async move {
                let existing = stores
                    .venues()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Local de evento não encontrado.".to_string())
                    })?;

                if existing.venue.updated_at != expected_updated_at {
                    return Err(CoreError::Conflict(
                        "O cadastro do local foi alterado por outra pessoa. Recarregue os dados antes de salvar."
                            .to_string(),
                    ));
                }

                let existing_space_map: std::collections::HashMap<String, &VenueSpace> = existing
                    .spaces
                    .iter()
                    .map(|s| (s.id.clone(), s))
                    .collect();
                let mut seen_space_ids = std::collections::HashSet::new();

                let mut spaces = Vec::with_capacity(space_inputs.len());
                for (idx, sp) in space_inputs.into_iter().enumerate() {
                    let (space_id, created_at, created_by) = match sp.id {
                        Some(ref raw_id) if !raw_id.trim().is_empty() => {
                            let sid = raw_id.trim().to_string();
                            if !seen_space_ids.insert(sid.clone()) {
                                return Err(CoreError::Validation(
                                    "IDs de espaço duplicados na atualização.".to_string(),
                                ));
                            }
                            let prev = existing_space_map.get(&sid).ok_or_else(|| {
                                CoreError::Validation(format!(
                                    "O espaço informado (id: {sid}) não pertence a este local."
                                ))
                            })?;
                            (sid, prev.created_at.clone(), prev.created_by.clone())
                        }
                        _ => (
                            Uuid::now_v7().to_string(),
                            now.clone(),
                            Some(session.user_id.clone()),
                        ),
                    };

                    spaces.push(VenueSpace {
                        id: space_id,
                        tenant_id: session.tenant_id.clone(),
                        venue_id: existing.venue.id.clone(),
                        name_normalized: normalize_for_search(&sp.name),
                        name: sp.name,
                        capacity_people: sp.capacity_people,
                        area_sqm: sp.area_sqm,
                        ceiling_height_m: sp.ceiling_height_m,
                        power_available: sp.power_available,
                        loading_access: sp.loading_access,
                        rigging_points: sp.rigging_points,
                        acoustics_notes: sp.acoustics_notes,
                        setup_hours_notes: sp.setup_hours_notes,
                        noise_restrictions_notes: sp.noise_restrictions_notes,
                        load_dimensions_notes: sp.load_dimensions_notes,
                        notes: sp.notes,
                        active: sp.active,
                        sort_order: if sp.sort_order == 0 {
                            idx as i64
                        } else {
                            sp.sort_order
                        },
                        created_at,
                        updated_at: now.clone(),
                        deleted_at: None,
                        created_by,
                    });
                }

                let existing_contact_map: std::collections::HashMap<String, &VenueContact> =
                    existing.contacts.iter().map(|c| (c.id.clone(), c)).collect();
                let mut seen_contact_ids = std::collections::HashSet::new();

                let mut contacts = Vec::with_capacity(contact_inputs.len());
                for ct in contact_inputs {
                    let (contact_id, created_at, created_by) = match ct.id {
                        Some(ref raw_id) if !raw_id.trim().is_empty() => {
                            let cid = raw_id.trim().to_string();
                            if !seen_contact_ids.insert(cid.clone()) {
                                return Err(CoreError::Validation(
                                    "IDs de contato duplicados na atualização.".to_string(),
                                ));
                            }
                            let prev = existing_contact_map.get(&cid).ok_or_else(|| {
                                CoreError::Validation(format!(
                                    "O contato informado (id: {cid}) não pertence a este local."
                                ))
                            })?;
                            (cid, prev.created_at.clone(), prev.created_by.clone())
                        }
                        _ => (
                            Uuid::now_v7().to_string(),
                            now.clone(),
                            Some(session.user_id.clone()),
                        ),
                    };

                    contacts.push(VenueContact {
                        id: contact_id,
                        tenant_id: session.tenant_id.clone(),
                        venue_id: existing.venue.id.clone(),
                        name: ct.name,
                        email: ct.email,
                        phone: ct.phone,
                        role: ct.role,
                        is_primary: ct.is_primary,
                        created_at,
                        updated_at: now.clone(),
                        deleted_at: None,
                        created_by,
                    });
                }

                let updated_venue = Venue {
                    id: existing.venue.id.clone(),
                    tenant_id: session.tenant_id.clone(),
                    name,
                    street,
                    number,
                    complement,
                    neighborhood,
                    city,
                    state,
                    postal_code,
                    country,
                    parking_notes,
                    loading_dock_notes,
                    access_hours,
                    internal_transfer_minutes,
                    notes,
                    active: existing.venue.active,
                    search_text,
                    name_sort,
                    created_at: existing.venue.created_at.clone(),
                    updated_at: now,
                    deleted_at: None,
                    created_by: existing.venue.created_by.clone(),
                };

                stores
                    .venues()
                    .update(&updated_venue, &spaces, &contacts)
                    .await?;

                let detail = format!(
                    "Local de evento '{}' atualizado ({} espaço(s), {} contato(s), id: {})",
                    updated_venue.name,
                    spaces.len(),
                    contacts.len(),
                    updated_venue.id
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "venue.updated",
                        "venue",
                        &updated_venue.id,
                        &detail,
                    ))
                    .await?;

                Ok(VenueWithDetails {
                    venue: updated_venue,
                    spaces,
                    contacts,
                })
            })
        })
        .await
    }

    pub async fn get_venue(&self, session: &SessionInfo, id: &str) -> Result<VenueWithDetails> {
        self.check_session_and_permission(session, PERM_VENUE_READ)?;
        self.stores
            .venues
            .find_by_id(&session.tenant_id, id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Local de evento não encontrado.".to_string()))
    }

    pub async fn list_venues(
        &self,
        session: &SessionInfo,
        filter: &VenueFilter,
    ) -> Result<VenuePage> {
        self.check_session_and_permission(session, PERM_VENUE_READ)?;

        if let Some(ref cursor_str) = filter.cursor {
            VenueCursor::decode(cursor_str)?;
        }

        validate_search_query_limits(filter.query.as_deref())?;

        let mut normalized_filter = filter.clone();
        if let Some(ref q) = normalized_filter.query {
            let trimmed = q.trim();
            if trimmed.is_empty() {
                normalized_filter.query = None;
            } else {
                normalized_filter.query = Some(normalize_for_search(trimmed));
            }
        }
        if let Some(limit) = normalized_filter.limit {
            normalized_filter.limit = Some(limit.clamp(1, 100));
        } else {
            normalized_filter.limit = Some(50);
        }

        self.stores
            .venues
            .list(&session.tenant_id, &normalized_filter)
            .await
    }

    pub async fn check_venue_duplicate_name_city(
        &self,
        session: &SessionInfo,
        name: &str,
        city: Option<&str>,
        exclude_id: Option<&str>,
    ) -> Result<bool> {
        self.check_session_and_permission(session, PERM_VENUE_READ)?;
        let name_norm = normalize_for_search(name);
        if name_norm.is_empty() {
            return Ok(false);
        }
        let city_norm = city.map(normalize_for_search).filter(|s| !s.is_empty());
        self.stores
            .venues
            .exists_by_name_and_city(
                &session.tenant_id,
                &name_norm,
                city_norm.as_deref(),
                exclude_id,
            )
            .await
    }

    pub async fn set_venue_active(
        &self,
        session: &SessionInfo,
        id: &str,
        active: bool,
    ) -> Result<VenueWithDetails> {
        self.check_session_and_permission(session, PERM_VENUE_MANAGE)?;
        let session_clone = session.clone();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();

            Box::pin(async move {
                let mut existing = stores
                    .venues()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Local de evento não encontrado.".to_string())
                    })?;

                stores
                    .venues()
                    .set_active(&session.tenant_id, &id, active, &now)
                    .await?;
                existing.venue.active = active;
                existing.venue.updated_at = now;

                let action = if active {
                    "venue.activated"
                } else {
                    "venue.deactivated"
                };
                let status_label = if active { "ativado" } else { "desativado" };
                let detail = format!(
                    "Local de evento '{}' {} (id: {})",
                    existing.venue.name, status_label, existing.venue.id
                );

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        action,
                        "venue",
                        &id,
                        &detail,
                    ))
                    .await?;

                Ok(existing)
            })
        })
        .await
    }

    pub async fn delete_venue(&self, session: &SessionInfo, id: &str) -> Result<()> {
        self.check_session_and_permission(session, PERM_VENUE_MANAGE)?;
        let session_clone = session.clone();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let id = id_str.clone();
            let now = now.clone();

            Box::pin(async move {
                let existing = stores
                    .venues()
                    .find_by_id(&session.tenant_id, &id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::NotFound("Local de evento não encontrado.".to_string())
                    })?;

                stores
                    .venues()
                    .soft_delete(&session.tenant_id, &id, &now)
                    .await?;

                let detail = format!(
                    "Local de evento '{}' excluído logicamente (soft delete, id: {})",
                    existing.venue.name, existing.venue.id
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "venue.deleted",
                        "venue",
                        &id,
                        &detail,
                    ))
                    .await?;

                Ok(())
            })
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Configurações Operacionais / Alocação (Fase 1D / ADR-0012)
    // -----------------------------------------------------------------------

    pub async fn get_allocation_settings(
        &self,
        session: &SessionInfo,
    ) -> Result<AllocationSettings> {
        self.check_session_and_permission(session, PERM_SETTINGS_READ)?;
        self.get_allocation_settings_for_tenant(&session.tenant_id)
            .await
    }

    /// Leitura interna sem sessão (utilizada pelo motor de disponibilidade da Fase 3).
    /// Nunca grava no banco: quando a chave não existe ou tem valor inválido,
    /// devolve o padrão em memória (registrando aviso no log no caso inválido).
    pub async fn get_allocation_settings_for_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<AllocationSettings> {
        let rows = self
            .stores
            .settings
            .get_many(
                tenant_id,
                &[
                    SETTING_CHANGEOVER_MINUTES,
                    SETTING_INTERNAL_TRANSFER_MINUTES,
                    SETTING_EXTERNAL_TRANSFER_MINUTES,
                ],
            )
            .await?;

        let mut result = AllocationSettings::default();
        for row in rows {
            let max_allowed = match row.setting_key.as_str() {
                SETTING_CHANGEOVER_MINUTES => MAX_CHANGEOVER_MINUTES,
                SETTING_INTERNAL_TRANSFER_MINUTES => MAX_INTERNAL_TRANSFER_SETTING_MINUTES,
                SETTING_EXTERNAL_TRANSFER_MINUTES => MAX_EXTERNAL_TRANSFER_MINUTES,
                _ => continue,
            };

            let parsed_opt = row
                .setting_value
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|v| (0..=max_allowed).contains(v));

            let Some(parsed) = parsed_opt else {
                tracing::warn!(
                    tenant_id = %tenant_id,
                    setting_key = %row.setting_key,
                    raw_value = %row.setting_value,
                    "Valor inválido em tenant_setting; usando padrão em memória."
                );
                continue;
            };

            match row.setting_key.as_str() {
                SETTING_CHANGEOVER_MINUTES => result.changeover_minutes = parsed,
                SETTING_INTERNAL_TRANSFER_MINUTES => result.internal_transfer_minutes = parsed,
                SETTING_EXTERNAL_TRANSFER_MINUTES => result.external_transfer_minutes = parsed,
                _ => {}
            }
        }
        Ok(result)
    }

    pub async fn update_allocation_settings(
        &self,
        session: &SessionInfo,
        input: &UpdateAllocationSettingsInput,
    ) -> Result<AllocationSettings> {
        self.check_session_and_permission(session, PERM_SETTINGS_MANAGE)?;
        crate::domain::validate_allocation_settings(input)?;

        let session_clone = session.clone();
        let input_clone = input.clone();
        let now = Utc::now().to_rfc3339();

        self.run_transaction(|stores| {
            let session = session_clone.clone();
            let input = input_clone.clone();
            let now = now.clone();
            Box::pin(async move {
                let pairs = [
                    (
                        SETTING_CHANGEOVER_MINUTES,
                        input.changeover_minutes.to_string(),
                    ),
                    (
                        SETTING_INTERNAL_TRANSFER_MINUTES,
                        input.internal_transfer_minutes.to_string(),
                    ),
                    (
                        SETTING_EXTERNAL_TRANSFER_MINUTES,
                        input.external_transfer_minutes.to_string(),
                    ),
                ];

                for (key, val) in pairs {
                    stores
                        .settings()
                        .upsert(&TenantSetting {
                            id: Uuid::now_v7().to_string(),
                            tenant_id: session.tenant_id.clone(),
                            setting_key: key.to_string(),
                            setting_value: val,
                            updated_at: now.clone(),
                            updated_by: Some(session.user_id.clone()),
                        })
                        .await?;
                }

                let detail = format!(
                    "Configurações de alocação atualizadas: virada={}min, deslocamento_interno={}min, deslocamento_externo={}min",
                    input.changeover_minutes,
                    input.internal_transfer_minutes,
                    input.external_transfer_minutes
                );
                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "settings.allocation_updated",
                        "tenant_setting",
                        &session.tenant_id,
                        &detail,
                    ))
                    .await?;

                Ok(AllocationSettings {
                    changeover_minutes: input.changeover_minutes,
                    internal_transfer_minutes: input.internal_transfer_minutes,
                    external_transfer_minutes: input.external_transfer_minutes,
                })
            })
        })
        .await
    }
}

fn can_read_party(session: &SessionInfo, customer: &Customer) -> bool {
    let can_read_cust =
        session.has_permission(PERM_CUSTOMER_READ) || session.has_permission(PERM_CUSTOMER_MANAGE);
    let can_read_supp =
        session.has_permission(PERM_SUPPLIER_READ) || session.has_permission(PERM_SUPPLIER_MANAGE);
    (customer.is_customer && can_read_cust) || (customer.is_supplier && can_read_supp)
}

fn duplicate_document_conflict_error(session: &SessionInfo, existing: &Customer) -> CoreError {
    if can_read_party(session, existing) {
        CoreError::Conflict(format!(
            "Já existe um cadastro com este documento (id: {}).",
            existing.id
        ))
    } else {
        CoreError::Conflict(
            "Este documento já está cadastrado. Se precisar usá-lo neste papel, peça a quem gerencia o cadastro existente."
                .to_string(),
        )
    }
}

fn validate_search_query_limits(query: Option<&str>) -> Result<()> {
    if let Some(q) = query {
        let trimmed = q.trim();
        if trimmed.chars().count() > 100 {
            return Err(CoreError::Validation(
                "O termo de busca não pode exceder 100 caracteres.".to_string(),
            ));
        }
        if trimmed.split_whitespace().count() > 8 {
            return Err(CoreError::Validation(
                "O termo de busca não pode conter mais de 8 palavras.".to_string(),
            ));
        }
    }
    Ok(())
}
