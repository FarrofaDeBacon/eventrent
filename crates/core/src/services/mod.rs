use crate::domain::{
    AuditLogEntry, CreateUserInput, InitialSetupInput, InitialSetupResult, LoginInput,
    OperationMode, RecoverPasswordInput, RecoverPasswordResult, ResetPasswordInput, Role,
    RoleWithPermissions, SessionInfo, Tenant, TenantBranding, UpdateBrandingInput, UpdateUserInput,
    UserRecord, UserWithRole, PERM_BRANDING_MANAGE, PERM_ROLE_READ, PERM_USER_MANAGE,
    PERM_USER_READ, ROLE_SLUG_OWNER, SYSTEM_ROLES,
};
use crate::error::{CoreError, Result};
use crate::store::{BrandingStore, RoleStore, StoreBundle, TenantStore};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use uuid::Uuid;

pub const DEFAULT_COMPANY_NAME: &str = "Minha empresa";
pub const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;
pub const MIN_PASSWORD_LEN: usize = 8;

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
        };
        branding.save(&tenant_id, &default_branding).await?;
    }

    Ok(tenant_id)
}

/// Garante de forma idempotente a criação dos 5 perfis de sistema para um tenant.
pub async fn ensure_system_roles(
    tenant_id: &str,
    roles: &dyn RoleStore,
) -> Result<Vec<RoleWithPermissions>> {
    let now = Utc::now().to_rfc3339();

    for spec in SYSTEM_ROLES {
        let perms: Vec<String> = spec.permissions.iter().map(|s| (*s).to_string()).collect();

        if let Some(existing) = roles.find_by_slug(tenant_id, spec.slug).await? {
            if existing.permissions.len() != perms.len() {
                roles
                    .set_permissions(tenant_id, &existing.id, &perms, &now)
                    .await?;
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

#[derive(Clone)]
pub struct CoreServices {
    pub stores: StoreBundle,
}

impl CoreServices {
    pub fn new(stores: StoreBundle) -> Self {
        Self { stores }
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
        let last_activity = DateTime::parse_from_rfc3339(&session.last_activity_at)
            .map_err(|_| CoreError::Unauthorized("Timestamp de sessão inválido.".to_string()))?
            .with_timezone(&Utc);

        let timeout = Duration::minutes(i64::from(session.inactivity_timeout_minutes.max(1)));
        if Utc::now() - last_activity > timeout {
            return Err(CoreError::Unauthorized(
                "Sessão expirada por inatividade.".to_string(),
            ));
        }
        Ok(())
    }

    async fn record_audit(
        &self,
        tenant_id: &str,
        actor_user_id: Option<&str>,
        action: &str,
        entity_type: &str,
        entity_id: &str,
        summary: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let entry = AuditLogEntry {
            id: Uuid::now_v7().to_string(),
            tenant_id: tenant_id.to_string(),
            actor_user_id: actor_user_id.map(str::to_string),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            summary: summary.to_string(),
            created_at: now,
            created_by: actor_user_id.map(str::to_string),
        };
        self.stores.audit.append(&entry).await
    }

    /// Executa o assistente de primeira execução no modo Local:
    /// configura nome da empresa, perfis de sistema, conta do Proprietário e retorna a chave de recuperação única.
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

        let existing_users = self.stores.users.count_by_tenant(tenant_id).await?;
        if existing_users > 0 {
            return Err(CoreError::Conflict(
                "A configuração inicial já foi realizada para este ambiente.".to_string(),
            ));
        }

        let company_name = input.company_name.trim();
        let owner_name = input.owner_name.trim();
        let owner_email = input.owner_email.trim().to_lowercase();

        if company_name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome da empresa.".to_string(),
            ));
        }
        if owner_name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do proprietário.".to_string(),
            ));
        }
        if !owner_email.contains('@') || owner_email.len() < 5 {
            return Err(CoreError::Validation(
                "Informe um e-mail válido.".to_string(),
            ));
        }
        if input.owner_password.len() < MIN_PASSWORD_LEN {
            return Err(CoreError::Validation(format!(
                "A senha deve ter pelo menos {MIN_PASSWORD_LEN} caracteres."
            )));
        }

        let now = Utc::now().to_rfc3339();
        self.stores
            .tenants
            .update_name(tenant_id, company_name, &now)
            .await?;

        let current_branding = self.stores.branding.get_by_tenant_id(tenant_id).await?;
        let branding_input = UpdateBrandingInput {
            company_name: company_name.to_string(),
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
            logo_url: current_branding.and_then(|b| b.logo_url),
        };
        let branding = self
            .stores
            .branding
            .save(tenant_id, &branding_input)
            .await?;

        ensure_system_roles(tenant_id, self.stores.roles.as_ref()).await?;

        let owner_role = self
            .stores
            .roles
            .find_by_slug(tenant_id, ROLE_SLUG_OWNER)
            .await?
            .ok_or_else(|| {
                CoreError::Internal("Perfil Proprietário não encontrado.".to_string())
            })?;

        let recovery_key = generate_recovery_key();
        let password_hash = hash_secret(&input.owner_password).await?;
        let recovery_key_hash = hash_secret(&normalize_recovery_key(&recovery_key)).await?;

        let owner_id = Uuid::now_v7().to_string();
        let owner_record = UserRecord {
            id: owner_id.clone(),
            tenant_id: tenant_id.to_string(),
            name: owner_name.to_string(),
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

        let created_owner = self.stores.users.create(&owner_record).await?;

        self.record_audit(
            tenant_id,
            Some(&created_owner.id),
            "tenant.setup_completed",
            "tenant",
            tenant_id,
            &format!("Configuração inicial concluída para '{company_name}' em modo local"),
        )
        .await?;

        self.record_audit(
            tenant_id,
            Some(&created_owner.id),
            "user.owner_created",
            "user",
            &created_owner.id,
            &format!("Conta de proprietário criada para {owner_email}"),
        )
        .await?;

        let session = SessionInfo {
            user_id: created_owner.id,
            tenant_id: tenant_id.to_string(),
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
            tenant_id: tenant_id.to_string(),
            recovery_key,
            session,
            branding,
        })
    }

    /// Realiza autenticação por e-mail e senha com proteção Argon2id e bloqueio progressivo.
    pub async fn login(
        &self,
        tenant_id: &str,
        input: &LoginInput,
        inactivity_timeout_minutes: u32,
    ) -> Result<SessionInfo> {
        let email = input.email.trim().to_lowercase();
        if email.is_empty() || input.password.is_empty() {
            return Err(CoreError::InvalidCredentials);
        }

        let Some(user) = self.stores.users.find_by_email(tenant_id, &email).await? else {
            return Err(CoreError::InvalidCredentials);
        };

        if !user.active {
            return Err(CoreError::InvalidCredentials);
        }

        let now = Utc::now();
        let now_str = now.to_rfc3339();

        if let Some(ref locked_str) = user.locked_until {
            if let Ok(locked_dt) = DateTime::parse_from_rfc3339(locked_str) {
                if locked_dt.with_timezone(&Utc) > now {
                    return Err(CoreError::AccountLocked {
                        locked_until: locked_str.clone(),
                    });
                }
            }
        }

        if !verify_secret(&input.password, &user.password_hash).await? {
            let new_failures = user.failed_attempts + 1;
            let lockout_secs = calculate_lockout_seconds(new_failures);
            let locked_until = if lockout_secs > 0 {
                Some((now + Duration::seconds(lockout_secs)).to_rfc3339())
            } else {
                None
            };

            self.stores
                .users
                .record_login_failure(
                    tenant_id,
                    &user.id,
                    new_failures,
                    locked_until.as_deref(),
                    &now_str,
                )
                .await?;

            if let Some(ref until) = locked_until {
                self.record_audit(
                    tenant_id,
                    Some(&user.id),
                    "auth.account_locked",
                    "user",
                    &user.id,
                    &format!(
                        "Conta bloqueada temporariamente após {new_failures} tentativas falhas"
                    ),
                )
                .await?;
                return Err(CoreError::AccountLocked {
                    locked_until: until.clone(),
                });
            }

            self.record_audit(
                tenant_id,
                Some(&user.id),
                "auth.login_failed",
                "user",
                &user.id,
                &format!("Tentativa de login falha ({new_failures}) para {email}"),
            )
            .await?;

            return Err(CoreError::InvalidCredentials);
        }

        self.stores
            .users
            .record_login_success(tenant_id, &user.id, &now_str)
            .await?;

        let user_with_role = self
            .stores
            .users
            .find_with_role(tenant_id, &user.id)
            .await?
            .ok_or_else(|| {
                CoreError::NotFound("Usuário autenticado não encontrado.".to_string())
            })?;

        self.record_audit(
            tenant_id,
            Some(&user.id),
            "auth.login_success",
            "session",
            &user.id,
            &format!("Login realizado com sucesso por {}", user.email),
        )
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
    }

    /// Redefine a senha do proprietário usando a Chave de Recuperação offline.
    ///
    /// Políticas de segurança reforçadas (Fase 1A.1):
    /// - Limite de tentativas com backoff progressivo persistido em banco (`recovery_failed_attempts` e `recovery_locked_until`).
    /// - Gravação obrigatória em `audit_log` para falhas e sucessos.
    /// - Rotação obrigatória: a chave usada é invalidada e uma NOVA chave (80 bits) é gerada e retornada para exibição única.
    /// - Verificação criptográfica sempre via Argon2id em `tokio::task::spawn_blocking`.
    pub async fn recover_password(
        &self,
        tenant_id: &str,
        input: &RecoverPasswordInput,
    ) -> Result<RecoverPasswordResult> {
        let email = input.email.trim().to_lowercase();
        let normalized_key = normalize_recovery_key(&input.recovery_key);

        if input.new_password.len() < MIN_PASSWORD_LEN {
            return Err(CoreError::Validation(format!(
                "A nova senha deve ter pelo menos {MIN_PASSWORD_LEN} caracteres."
            )));
        }

        let Some(user) = self.stores.users.find_by_email(tenant_id, &email).await? else {
            return Err(CoreError::InvalidCredentials);
        };

        let Some(ref key_hash) = user.recovery_key_hash else {
            return Err(CoreError::InvalidCredentials);
        };

        let now = Utc::now();
        let now_str = now.to_rfc3339();

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
            let new_failures = user.recovery_failed_attempts + 1;
            let lockout_secs = calculate_lockout_seconds(new_failures);
            let locked_until = if lockout_secs > 0 {
                Some((now + Duration::seconds(lockout_secs)).to_rfc3339())
            } else {
                None
            };

            self.stores
                .users
                .record_recovery_failure(
                    tenant_id,
                    &user.id,
                    new_failures,
                    locked_until.as_deref(),
                    &now_str,
                )
                .await?;

            if let Some(ref until) = locked_until {
                self.record_audit(
                    tenant_id,
                    Some(&user.id),
                    "auth.recovery_locked",
                    "user",
                    &user.id,
                    &format!(
                        "Chave de recuperação bloqueada temporariamente após {new_failures} falhas consecutivas"
                    ),
                )
                .await?;
                return Err(CoreError::AccountLocked {
                    locked_until: until.clone(),
                });
            }

            self.record_audit(
                tenant_id,
                Some(&user.id),
                "auth.recovery_failed",
                "user",
                &user.id,
                &format!(
                    "Tentativa inválida de chave de recuperação ({new_failures}) para {email}"
                ),
            )
            .await?;

            return Err(CoreError::InvalidCredentials);
        }

        let new_recovery_key = generate_recovery_key();
        let new_recovery_key_hash = hash_secret(&normalize_recovery_key(&new_recovery_key)).await?;
        let new_password_hash = hash_secret(&input.new_password).await?;

        self.stores
            .users
            .update_password_and_recovery_key(
                tenant_id,
                &user.id,
                &new_password_hash,
                &new_recovery_key_hash,
                &now_str,
            )
            .await?;

        self.record_audit(
            tenant_id,
            Some(&user.id),
            "user.password_recovered",
            "user",
            &user.id,
            &format!("Senha redefinida com sucesso via chave de recuperação para {email}"),
        )
        .await?;

        self.record_audit(
            tenant_id,
            Some(&user.id),
            "auth.recovery_key_rotated",
            "user",
            &user.id,
            &format!("Chave de recuperação rotacionada com 80 bits de entropia para {email}"),
        )
        .await?;

        Ok(RecoverPasswordResult { new_recovery_key })
    }

    pub async fn list_users(&self, session: &SessionInfo) -> Result<Vec<UserWithRole>> {
        self.verify_session_active(session)?;
        if !session.has_permission(PERM_USER_READ) && !session.has_permission(PERM_USER_MANAGE) {
            return Err(CoreError::Forbidden(PERM_USER_READ.to_string()));
        }
        self.stores.users.list_by_tenant(&session.tenant_id).await
    }

    pub async fn create_user(
        &self,
        session: &SessionInfo,
        input: &CreateUserInput,
    ) -> Result<UserWithRole> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;

        let name = input.name.trim();
        let email = input.email.trim().to_lowercase();

        if name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do usuário.".to_string(),
            ));
        }
        if !email.contains('@') || email.len() < 5 {
            return Err(CoreError::Validation(
                "Informe um e-mail válido.".to_string(),
            ));
        }
        if input.password.len() < MIN_PASSWORD_LEN {
            return Err(CoreError::Validation(format!(
                "A senha deve ter pelo menos {MIN_PASSWORD_LEN} caracteres."
            )));
        }

        if self
            .stores
            .users
            .find_by_email(&session.tenant_id, &email)
            .await?
            .is_some()
        {
            return Err(CoreError::Conflict(
                "Já existe um usuário ativo com este e-mail.".to_string(),
            ));
        }

        let role = self
            .stores
            .roles
            .find_by_id(&session.tenant_id, &input.role_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Perfil de acesso não encontrado.".to_string()))?;

        let now = Utc::now().to_rfc3339();
        let password_hash = hash_secret(&input.password).await?;

        let record = UserRecord {
            id: Uuid::now_v7().to_string(),
            tenant_id: session.tenant_id.clone(),
            name: name.to_string(),
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

        let created = self.stores.users.create(&record).await?;

        self.record_audit(
            &session.tenant_id,
            Some(&session.user_id),
            "user.created",
            "user",
            &created.id,
            &format!(
                "Usuário '{}' ({}) criado com perfil '{}'",
                created.name, created.email, role.name
            ),
        )
        .await?;

        Ok(created)
    }

    pub async fn update_user(
        &self,
        session: &SessionInfo,
        user_id: &str,
        input: &UpdateUserInput,
    ) -> Result<UserWithRole> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;

        let name = input.name.trim();
        let email = input.email.trim().to_lowercase();

        if name.is_empty() {
            return Err(CoreError::Validation(
                "Informe o nome do usuário.".to_string(),
            ));
        }
        if !email.contains('@') || email.len() < 5 {
            return Err(CoreError::Validation(
                "Informe um e-mail válido.".to_string(),
            ));
        }

        let target = self
            .stores
            .users
            .find_with_role(&session.tenant_id, user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

        if let Some(existing_email_user) = self
            .stores
            .users
            .find_by_email(&session.tenant_id, &email)
            .await?
        {
            if existing_email_user.id != user_id {
                return Err(CoreError::Conflict(
                    "Já existe outro usuário com este e-mail.".to_string(),
                ));
            }
        }

        let new_role = self
            .stores
            .roles
            .find_by_id(&session.tenant_id, &input.role_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Perfil de acesso não encontrado.".to_string()))?;

        if target.role_slug == ROLE_SLUG_OWNER && target.active && new_role.slug != ROLE_SLUG_OWNER
        {
            let active_owners = self
                .stores
                .users
                .count_active_by_role_slug(&session.tenant_id, ROLE_SLUG_OWNER)
                .await?;
            if active_owners <= 1 {
                return Err(CoreError::Validation(
                    "Não é possível alterar o perfil do último Proprietário ativo.".to_string(),
                ));
            }
        }

        let now = Utc::now().to_rfc3339();
        let updated = self
            .stores
            .users
            .update(
                &session.tenant_id,
                user_id,
                name,
                &email,
                &new_role.id,
                &now,
            )
            .await?;

        self.record_audit(
            &session.tenant_id,
            Some(&session.user_id),
            "user.updated",
            "user",
            user_id,
            &format!(
                "Usuário '{}' atualizado (e-mail: {}, perfil: {})",
                updated.name, updated.email, updated.role_name
            ),
        )
        .await?;

        Ok(updated)
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

        let target = self
            .stores
            .users
            .find_with_role(&session.tenant_id, user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

        if !active && target.active && target.role_slug == ROLE_SLUG_OWNER {
            let active_owners = self
                .stores
                .users
                .count_active_by_role_slug(&session.tenant_id, ROLE_SLUG_OWNER)
                .await?;
            if active_owners <= 1 {
                return Err(CoreError::Validation(
                    "Não é possível desativar o último Proprietário ativo.".to_string(),
                ));
            }
        }

        let now = Utc::now().to_rfc3339();
        let updated = self
            .stores
            .users
            .set_active(&session.tenant_id, user_id, active, &now)
            .await?;

        let (action, verb) = if active {
            ("user.activated", "reativado")
        } else {
            ("user.deactivated", "desativado")
        };

        self.record_audit(
            &session.tenant_id,
            Some(&session.user_id),
            action,
            "user",
            user_id,
            &format!("Usuário '{}' ({}) {}", updated.name, updated.email, verb),
        )
        .await?;

        Ok(updated)
    }

    pub async fn reset_user_password(
        &self,
        session: &SessionInfo,
        user_id: &str,
        input: &ResetPasswordInput,
    ) -> Result<()> {
        self.check_session_and_permission(session, PERM_USER_MANAGE)?;

        if input.new_password.len() < MIN_PASSWORD_LEN {
            return Err(CoreError::Validation(format!(
                "A nova senha deve ter pelo menos {MIN_PASSWORD_LEN} caracteres."
            )));
        }

        let target = self
            .stores
            .users
            .find_by_id(&session.tenant_id, user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Usuário não encontrado.".to_string()))?;

        let now = Utc::now().to_rfc3339();
        let password_hash = hash_secret(&input.new_password).await?;
        self.stores
            .users
            .update_password(&session.tenant_id, user_id, &password_hash, &now)
            .await?;

        self.record_audit(
            &session.tenant_id,
            Some(&session.user_id),
            "user.password_reset",
            "user",
            user_id,
            &format!("Senha redefinida administrativamente para {}", target.email),
        )
        .await?;

        Ok(())
    }

    pub async fn list_roles(&self, session: &SessionInfo) -> Result<Vec<RoleWithPermissions>> {
        self.verify_session_active(session)?;
        if !session.has_permission(PERM_ROLE_READ)
            && !session.has_permission(PERM_USER_READ)
            && !session.has_permission(PERM_USER_MANAGE)
        {
            return Err(CoreError::Forbidden(PERM_ROLE_READ.to_string()));
        }
        self.stores.roles.list_by_tenant(&session.tenant_id).await
    }

    pub async fn save_branding(
        &self,
        session: &SessionInfo,
        input: &UpdateBrandingInput,
    ) -> Result<TenantBranding> {
        self.check_session_and_permission(session, PERM_BRANDING_MANAGE)?;
        let updated = self.stores.branding.save(&session.tenant_id, input).await?;

        self.record_audit(
            &session.tenant_id,
            Some(&session.user_id),
            "branding.updated",
            "tenant_branding",
            &updated.id,
            &format!(
                "Identidade visual atualizada para '{}'",
                updated.company_name
            ),
        )
        .await?;

        Ok(updated)
    }
}
