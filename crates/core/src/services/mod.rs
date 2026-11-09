use crate::domain::{
    AuditLogEntry, BrandingValidator, CreateUserInput, InitialSetupInput, InitialSetupResult,
    LoginInput, LoginPolicy, OperationMode, PublicBranding, RecoverPasswordInput,
    RecoverPasswordResult, ResetPasswordInput, Role, RoleWithPermissions, SessionInfo, Tenant,
    TenantBranding, UpdateBrandingInput, UpdateUserInput, UserRecord, UserWithRole,
    PERM_BRANDING_MANAGE, PERM_BRANDING_READ, PERM_ROLE_READ, PERM_USER_MANAGE, PERM_USER_READ,
    ROLE_SLUG_OWNER, SYSTEM_ROLES,
};
use crate::error::{CoreError, Result};
use crate::store::{
    run_in_tx, BrandingStore, RoleStore, StoreBundle, TenantStore, TransactionPolicy, TxBoxFuture,
    TxStoreBundle, UserStore,
};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use async_trait::async_trait;
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

/// Valida formato de e-mail sem dependências externas nem unwrap().
///
/// Regras aplicadas (RFC 5321 e RFC 5890 para IDN/Punycode):
/// 1. Sem espaços em branco ou caracteres de controle.
/// 2. Comprimento total entre 5 e 254 caracteres.
/// 3. Presença de exatamente um caractere '@', separando parte local e domínio.
/// 4. Parte local: 1 a 64 caracteres, sem pontos no início/fim e sem pontos consecutivos.
/// 5. Domínio: não vazio, sem pontos ou hífens no início/fim, com pelo menos uma extensão (TLD).
/// 6. TLD (Top-Level Domain): no mínimo 2 caracteres, alfanumérico ou hífen (ex: punycode xn--p1ai), sem iniciar/terminar com hífen.
pub fn validate_email(email: &str) -> Result<String> {
    if email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(CoreError::Validation(
            "O e-mail não pode conter espaços ou caracteres de controle.".to_string(),
        ));
    }

    let lowered = email.to_lowercase();
    if lowered.len() < 5 || lowered.len() > 254 {
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
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
    {
        return Err(CoreError::Validation(
            "Parte local do e-mail inválida (máximo 64 caracteres, sem pontos nas extremidades ou consecutivos).".to_string(),
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

pub struct PassthroughTxStoreBundle<'a> {
    pub stores: &'a StoreBundle,
}

#[async_trait]
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
    fn users(&self) -> &dyn UserStore {
        self.stores.users.as_ref()
    }
    fn audit(&self) -> &dyn crate::store::AuditStore {
        self.stores.audit.as_ref()
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

impl CoreServices {
    pub fn new(stores: StoreBundle) -> Self {
        Self { stores }
    }

    pub async fn run_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'a> Fn(&'a dyn TxStoreBundle) -> TxBoxFuture<'a, T>,
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
        let owner_name = input.owner_name.trim();
        let owner_email = validate_email(&input.owner_email)?;
        validate_password(&input.owner_password)?;

        if owner_name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do proprietário.".to_string(),
            ));
        }

        let recovery_key = generate_recovery_key();
        let password_hash = hash_secret(&input.owner_password).await?;
        let recovery_key_hash = hash_secret(&normalize_recovery_key(&recovery_key)).await?;

        let tenant_id_str = tenant_id.to_string();
        let company_name_str = company_name.to_string();
        let owner_name_str = owner_name.to_string();
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
                    logo_dark_url: current_branding.as_ref().and_then(|b| b.logo_dark_url.clone()),
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

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&created_owner.id),
                        "user.owner_created",
                        "user",
                        &created_owner.id,
                        &format!("Conta de proprietário criada para {owner_email}"),
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
                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.login_failed",
                                    "user",
                                    &user_id,
                                    &format!("Tentativa de login falha ({new_failures}) para {email}"),
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

                    stores
                        .audit()
                        .append(&build_audit_entry(
                            &tenant_id,
                            Some(&user_id),
                            "auth.login_success",
                            "session",
                            &user_id,
                            &format!("Login realizado com sucesso por {user_email}"),
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
    pub async fn recover_password(
        &self,
        tenant_id: &str,
        input: &RecoverPasswordInput,
    ) -> Result<RecoverPasswordResult> {
        let email = input.email.trim().to_lowercase();
        let normalized_key = normalize_recovery_key(&input.recovery_key);

        validate_password(&input.new_password)?;

        let Some(user) = self.stores.users.find_by_email(tenant_id, &email).await? else {
            return Err(CoreError::InvalidCredentials);
        };

        let Some(ref key_hash) = user.recovery_key_hash else {
            return Err(CoreError::InvalidCredentials);
        };

        let now = Utc::now();
        if let Some(ref locked_str) = user.recovery_locked_until {
            if let Ok(locked_dt) = DateTime::parse_from_rfc3339(locked_str) {
                if locked_dt.with_timezone(&Utc) > now {
                    return Err(CoreError::AccountLocked {
                        locked_until: locked_str.clone(),
                    });
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
                            stores
                                .audit()
                                .append(&build_audit_entry(
                                    &tenant_id,
                                    Some(&user_id),
                                    "auth.recovery_failed",
                                    "user",
                                    &user_id,
                                    &format!(
                                        "Tentativa inválida de chave de recuperação ({new_failures}) para {email}"
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
                    return Err(CoreError::AccountLocked { locked_until });
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

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &tenant_id,
                        Some(&user_id),
                        "user.password_recovered",
                        "user",
                        &user_id,
                        &format!(
                            "Senha redefinida com sucesso via chave de recuperação para {email}"
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
                            "Chave de recuperação rotacionada com 80 bits de entropia para {email}"
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

        let name = input.name.trim();
        let email = validate_email(&input.email)?;
        validate_password(&input.password)?;

        if name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do usuário.".to_string(),
            ));
        }

        let password_hash = hash_secret(&input.password).await?;
        let session = session.clone();
        let name_str = name.to_string();
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
                            created.name, created.email, role.name
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

        let name = input.name.trim();
        let email = validate_email(&input.email)?;

        if name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do usuário.".to_string(),
            ));
        }

        let session = session.clone();
        let user_id_str = user_id.to_string();
        let name_str = name.to_string();
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
                            updated.name, updated.email, updated.role_name
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

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        action,
                        "user",
                        &user_id,
                        &format!("Usuário '{}' ({}) {}", updated.name, updated.email, verb),
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

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "user.password_reset",
                        "user",
                        &user_id,
                        &format!("Senha redefinida administrativamente para {}", target.email),
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

                stores
                    .audit()
                    .append(&build_audit_entry(
                        &session.tenant_id,
                        Some(&session.user_id),
                        "branding.updated",
                        "tenant_branding",
                        &updated.id,
                        &format!(
                            "Identidade visual atualizada para '{}'",
                            updated.company_name
                        ),
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
}
