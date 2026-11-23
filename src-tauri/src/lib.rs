pub mod commands;
pub mod contracts;
pub mod guards;

use chrono::{DateTime, Utc};
use eventrent_core::{
    ensure_default_tenant, ensure_system_roles, AppConfig, CoreServices, SessionInfo,
};
use eventrent_media_fs::FsMediaStore;
use eventrent_store_sqlite::{init_db, SqliteStore};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub struct AppState {
    pub services: CoreServices,
    pub default_tenant_id: String,
    pub config_path: PathBuf,
    pub temp_media_dir: PathBuf,
    config: Mutex<AppConfig>,
    session: Mutex<Option<SessionInfo>>,
}

/// Limpeza de arquivos temporários de mídia ao iniciar a aplicação (R3):
/// Remove subpastas e arquivos dentro de `temp_media_dir` com tempo de modificação superior a `max_age` (padrão: 1 hora).
pub fn cleanup_temp_media_dir_on_startup(temp_dir: &Path, max_age: std::time::Duration) {
    if !temp_dir.exists() {
        let _ = std::fs::create_dir_all(temp_dir);
        return;
    }

    let now = std::time::SystemTime::now();
    if let Ok(entries) = std::fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(elapsed) = now.duration_since(modified) {
                        if elapsed > max_age {
                            if meta.is_dir() {
                                let _ = std::fs::remove_dir_all(&path);
                            } else {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = std::fs::create_dir_all(temp_dir);
}

/// Limpeza de arquivos temporários de mídia ao encerrar a aplicação normalmente (R3):
/// Remove integralmente todo o conteúdo da pasta temporária.
pub fn cleanup_temp_media_dir_on_shutdown(temp_dir: &Path) {
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(temp_dir);
    }
    let _ = std::fs::create_dir_all(temp_dir);
}

/// Remove e recria a pasta de cópias temporárias de documentos.
pub fn cleanup_temp_media_dir(temp_dir: &Path) {
    cleanup_temp_media_dir_on_shutdown(temp_dir);
}

impl AppState {
    pub fn get_config(&self) -> Result<AppConfig, String> {
        let guard = self.config.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(guard.clone())
    }

    pub fn save_config(&self, new_cfg: &AppConfig) -> Result<(), String> {
        save_app_config_file(&self.config_path, new_cfg)?;
        let mut guard = self.config.lock().unwrap_or_else(PoisonError::into_inner);
        *guard = new_cfg.clone();
        Ok(())
    }

    pub fn set_session(&self, session: Option<SessionInfo>) {
        let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        *guard = session;
    }

    /// Retorna a sessão ativa se ainda válida, SEM renovar o timestamp de última atividade (touch).
    /// Garante que polling periódico do frontend (get_current_session) não mantenha a sessão viva indefinidamente.
    pub fn get_session_without_touch(&self) -> Option<SessionInfo> {
        self.get_session_at(Utc::now())
    }

    pub fn get_session_at(&self, now: DateTime<Utc>) -> Option<SessionInfo> {
        let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        let current = guard.as_mut()?;

        if CoreServices::verify_session_active_at(current, now).is_err() {
            *guard = None;
            return None;
        }

        Some(current.clone())
    }

    /// Exige sessão ativa e RENOVA o timestamp de última atividade (touch).
    /// Revalida dinamicamente no banco SQLite (auth_epoch, usuário ativo e permissões).
    /// Usado exclusivamente pela guarda de comandos protegidos.
    pub async fn require_session(&self) -> Result<SessionInfo, String> {
        self.require_session_at(Utc::now()).await
    }

    pub async fn require_session_at(&self, now: DateTime<Utc>) -> Result<SessionInfo, String> {
        let session_snapshot = {
            let guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            guard
                .clone()
                .ok_or_else(|| "Sessão expirada ou usuário não autenticado.".to_string())?
        };

        if CoreServices::verify_session_active_at(&session_snapshot, now).is_err() {
            let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            *guard = None;
            return Err("Sessão expirada por inatividade.".to_string());
        }

        // Validação dinâmica na base SQLite (S3 / ADR-0016):
        let revalidated = match self.services.revalidate_session(&session_snapshot).await {
            Ok(mut s) => {
                s.last_activity_at = now.to_rfc3339();
                s
            }
            Err(e) => {
                let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
                *guard = None;
                return Err(e.to_string());
            }
        };

        {
            let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            *guard = Some(revalidated.clone());
        }

        Ok(revalidated)
    }
}

pub fn map_core_err(err: eventrent_core::CoreError) -> String {
    match &err {
        eventrent_core::CoreError::TransientConflict(msg) => {
            format!("O sistema está temporariamente ocupado processando outra operação concorrente ({msg}). Por favor, tente novamente em instantes.")
        }
        _ => err.to_string(),
    }
}

pub fn load_or_create_app_config(config_path: &Path) -> Result<AppConfig, String> {
    if !config_path.exists() {
        let default_cfg = AppConfig::default();
        save_app_config_file(config_path, &default_cfg)?;
        return Ok(default_cfg);
    }

    let raw = std::fs::read_to_string(config_path).map_err(|e| {
        format!(
            "Falha ao ler arquivo de configuração ({}): {e}",
            config_path.display()
        )
    })?;

    serde_json::from_str::<AppConfig>(&raw).map_err(|e| {
        format!(
            "Arquivo de configuração inválido ({}): {e}",
            config_path.display()
        )
    })
}

/// Grava o arquivo config.json de forma atômica (arquivo temporário + sync_all + rename).
/// Previne corrupção de arquivo caso ocorra queda de energia ou terminação abrupta durante a escrita.
pub fn save_app_config_file(config_path: &Path, config: &AppConfig) -> Result<(), String> {
    save_app_config_file_internal(config_path, config, |src, dst| {
        std::fs::rename(src, dst).map_err(|e| e.to_string())
    })
}

pub(crate) fn save_app_config_file_internal<F>(
    config_path: &Path,
    config: &AppConfig,
    rename_op: F,
) -> Result<(), String>
where
    F: FnOnce(&Path, &Path) -> Result<(), String>,
{
    use std::io::Write;

    let serialized = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Falha ao serializar config.json: {e}"))?;

    let parent = config_path.parent().unwrap_or_else(|| Path::new("."));
    let temp_file = parent.join(format!(
        ".config.json.tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));

    {
        let mut file = std::fs::File::create(&temp_file).map_err(|e| {
            format!(
                "Falha ao criar arquivo temporário de configuração ({}): {e}",
                temp_file.display()
            )
        })?;

        file.write_all(serialized.as_bytes()).map_err(|e| {
            let _ = std::fs::remove_file(&temp_file);
            format!(
                "Falha ao gravar arquivo temporário de configuração ({}): {e}",
                temp_file.display()
            )
        })?;

        file.sync_all().map_err(|e| {
            let _ = std::fs::remove_file(&temp_file);
            format!(
                "Falha ao sincronizar arquivo temporário no disco ({}): {e}",
                temp_file.display()
            )
        })?;
    }

    rename_op(&temp_file, config_path).map_err(|err_msg| {
        let _ = std::fs::remove_file(&temp_file);
        format!(
            "Falha ao mover arquivo temporário para configuração final ({}): {err_msg}",
            config_path.display()
        )
    })?;

    Ok(())
}

fn init_logging(log_dir: &Path) {
    let file_appender = tracing_appender::rolling::daily(log_dir, "eventrent.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking);

    let stdout_layer = if cfg!(debug_assertions) {
        Some(tracing_subscriber::fmt::layer())
    } else {
        None
    };

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init();
}

/// Resolve o caminho do banco de dados SQLite.
/// O gancho de teste `EVENTRENT_FORCE_DB_PATH` existe exclusivamente em builds de
/// desenvolvimento (`#[cfg(debug_assertions)]`) e é completamente inexistente em release.
pub fn resolve_db_path(app_dir: &Path) -> PathBuf {
    #[cfg(debug_assertions)]
    if let Ok(forced_path) = std::env::var("EVENTRENT_FORCE_DB_PATH") {
        let trimmed = forced_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    app_dir.join("eventrent.db")
}

async fn initialize_app(app_handle: &AppHandle) -> Result<(AppState, PathBuf), (String, String)> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| {
        (
            format!("Falha ao obter diretório de dados do aplicativo (app_data_dir): {e}"),
            String::from("indisponível (app_data_dir não pôde ser resolvido)"),
        )
    })?;

    let log_dir = app_dir.join("logs");
    let log_dir_display = log_dir.display().to_string();

    std::fs::create_dir_all(&app_dir).map_err(|e| {
        (
            format!(
                "Falha ao criar diretório de dados do aplicativo ({}): {e}",
                app_dir.display()
            ),
            log_dir_display.clone(),
        )
    })?;

    std::fs::create_dir_all(&log_dir).map_err(|e| {
        (
            format!(
                "Falha ao criar diretório de logs do aplicativo ({}): {e}",
                log_dir.display()
            ),
            log_dir_display.clone(),
        )
    })?;

    init_logging(&log_dir);

    // Lê config.json ANTES de abrir o banco de dados (Requisito 2.1)
    let config_path = app_dir.join("config.json");
    let app_config = load_or_create_app_config(&config_path).map_err(|e| {
        tracing::error!("Erro crítico ao carregar config.json: {}", e);
        (e, log_dir_display.clone())
    })?;

    let db_path = resolve_db_path(&app_dir);

    tracing::info!(
        "Inicializando banco SQLite em: {}",
        db_path.to_string_lossy()
    );

    let pool = init_db(&db_path).await.map_err(|e| {
        tracing::error!("Erro crítico ao inicializar banco SQLite: {}", e);
        (
            format!("Falha ao inicializar ou migrar o banco de dados local: {e}"),
            log_dir_display.clone(),
        )
    })?;

    let store_bundle = SqliteStore::new(pool).into_bundle();

    let default_tenant_id = ensure_default_tenant(
        store_bundle.tenants.as_ref(),
        store_bundle.branding.as_ref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Erro crítico ao garantir tenant padrão: {}", e);
        (
            format!("Falha ao configurar dados iniciais do tenant padrão: {e}"),
            log_dir_display.clone(),
        )
    })?;

    ensure_system_roles(
        &default_tenant_id,
        store_bundle.roles.as_ref(),
        store_bundle.audit.as_ref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Erro crítico ao garantir perfis de sistema: {}", e);
        (
            format!("Falha ao inicializar perfis de permissão do sistema: {e}"),
            log_dir_display.clone(),
        )
    })?;

    tracing::info!(
        "Banco SQLite, tenant padrão ({}) e perfis de sistema verificados com sucesso",
        default_tenant_id
    );

    let temp_media_dir = app_dir.join("temp-media");
    cleanup_temp_media_dir_on_startup(&temp_media_dir, std::time::Duration::from_secs(3600));

    let media_store = FsMediaStore::new(&app_dir).map_err(|e| {
        tracing::error!("Erro crítico ao inicializar pasta de mídia: {}", e);
        (
            format!("Falha ao inicializar armazenamento de mídia local: {e}"),
            log_dir_display.clone(),
        )
    })?;

    // R7.2: Varredura ao iniciar removendo arquivos .tmp.* com mais de 24 horas
    if let Err(e) = media_store
        .cleanup_stale_temp_writes(std::time::Duration::from_secs(24 * 3600))
        .await
    {
        tracing::warn!("Aviso: falha na limpeza de temporários de mídia ao iniciar: {e}");
    }

    let services = CoreServices::with_media_store(store_bundle, Arc::new(media_store));

    Ok((
        AppState {
            services,
            default_tenant_id,
            config_path,
            temp_media_dir,
            config: Mutex::new(app_config),
            session: Mutex::new(None),
        },
        log_dir,
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                match initialize_app(&app_handle).await {
                    Ok((state, _log_dir)) => {
                        app_handle.manage(state);
                        Ok(())
                    }
                    Err((err_msg, log_path_display)) => {
                        tracing::error!("Inicialização abortada: {}", err_msg);

                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }

                        let exit_handle = app_handle.clone();
                        app_handle
                            .dialog()
                            .message(format!(
                                "Não foi possível iniciar o EventRent.\n\nMotivo: {err_msg}\n\nArquivo de log: {log_path_display}"
                            ))
                            .title("EventRent — Erro de Inicialização")
                            .kind(MessageDialogKind::Error)
                            .buttons(MessageDialogButtons::Ok)
                            .show(move |_| {
                                exit_handle.exit(1);
                            });

                        Ok(())
                    }
                }
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::setup::get_app_status,
            commands::setup::complete_initial_setup,
            commands::branding::get_public_branding,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_current_session,
            commands::auth::recover_password,
            commands::auth::regenerate_recovery_key,
            commands::users::list_users,
            commands::users::create_user,
            commands::users::update_user,
            commands::users::set_user_active,
            commands::users::reset_user_password,
            commands::users::list_roles,
            commands::branding::get_branding,
            commands::branding::save_branding,
            commands::customers::list_customers,
            commands::customers::get_customer,
            commands::customers::create_customer,
            commands::customers::update_customer,
            commands::customers::set_customer_active,
            commands::customers::delete_customer,
            commands::customers::add_customer_role,
            commands::venues::list_venues,
            commands::venues::get_venue,
            commands::venues::create_venue,
            commands::venues::update_venue,
            commands::venues::set_venue_active,
            commands::venues::delete_venue,
            commands::venues::check_venue_duplicate_name_city,
            commands::settings::get_allocation_settings,
            commands::settings::update_allocation_settings,
            commands::catalog::list_brands,
            commands::catalog::create_brand,
            commands::catalog::update_brand,
            commands::catalog::delete_brand,
            commands::catalog::list_categories_tree,
            commands::catalog::get_category_effective_attributes,
            commands::catalog::create_category,
            commands::catalog::update_category,
            commands::catalog::delete_category,
            commands::catalog::create_attribute_definition,
            commands::catalog::update_attribute_definition,
            commands::catalog::delete_attribute_definition,
            commands::catalog::list_equipment_models,
            commands::catalog::get_equipment_model,
            commands::catalog::create_equipment_model,
            commands::catalog::update_equipment_model,
            commands::catalog::set_equipment_model_active,
            commands::catalog::delete_equipment_model,
            commands::catalog::load_suggested_catalog_categories,
            commands::media::list_model_media,
            commands::media::get_media_blob_bytes,
            commands::media::attach_model_photo,
            commands::media::attach_model_document,
            commands::media::update_model_media_metadata,
            commands::media::reorder_model_media,
            commands::media::set_primary_model_photo,
            commands::media::remove_model_media,
            commands::media::collect_unreferenced_media,
            commands::media::open_model_document_external,
            commands::media::save_model_document_as,
        ])
        .build(tauri::generate_context!());

    match app_result {
        Ok(app) => {
            app.run(|app_handle, event| {
                if matches!(event, RunEvent::Exit) {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        cleanup_temp_media_dir_on_shutdown(&state.temp_media_dir);
                    }
                }
            });
        }
        Err(e) => {
            tracing::error!("Aplicação Tauri encerrada com erro: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_app_config_file_atomic() {
        let temp_dir = std::env::temp_dir().join(format!(
            "eventrent_cfg_test_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let cfg_path = temp_dir.join("config.json");

        let cfg = AppConfig {
            mode: Some(eventrent_core::OperationMode::Local),
            setup_completed: true,
            inactivity_timeout_minutes: 45,
        };

        save_app_config_file(&cfg_path, &cfg).unwrap();
        assert!(cfg_path.exists());

        let loaded = load_or_create_app_config(&cfg_path).unwrap();
        assert_eq!(loaded.inactivity_timeout_minutes, 45);
        assert!(loaded.setup_completed);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_app_config_file_never_deletes_original_on_rename_failure() {
        let temp_dir = std::env::temp_dir().join(format!(
            "eventrent_cfg_fail_test_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let cfg_path = temp_dir.join("config.json");

        // 1. Grava configuração inicial com sucesso (versão 1: 30 minutos)
        let initial_cfg = AppConfig {
            mode: Some(eventrent_core::OperationMode::Local),
            setup_completed: true,
            inactivity_timeout_minutes: 30,
        };
        save_app_config_file(&cfg_path, &initial_cfg).unwrap();
        assert!(cfg_path.exists());

        // 2. Tenta gravar nova configuração (versão 2: 90 minutos) simulando falha no rename
        let new_cfg = AppConfig {
            mode: Some(eventrent_core::OperationMode::Local),
            setup_completed: true,
            inactivity_timeout_minutes: 90,
        };

        let fail_res = save_app_config_file_internal(&cfg_path, &new_cfg, |_src, _dst| {
            Err("Simulação de falha catastrófica de I/O no rename".to_string())
        });
        assert!(
            fail_res.is_err(),
            "Deve retornar erro quando o rename falha"
        );

        // 3. O arquivo original NUNCA deve sumir e deve manter o conteúdo íntegro da versão 1
        assert!(
            cfg_path.exists(),
            "config.json original deve permanecer intacto"
        );
        let loaded = load_or_create_app_config(&cfg_path).unwrap();
        assert_eq!(
            loaded.inactivity_timeout_minutes, 30,
            "Conteúdo anterior deve ser preservado sem corrupção"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_get_current_session_does_not_renew_last_activity_at() {
        use chrono::Duration;
        use eventrent_store_sqlite::init_memory_db;

        let pool = init_memory_db().await.unwrap();

        let stores = eventrent_store_sqlite::SqliteStore::new(pool).into_bundle();
        let default_tenant_id =
            ensure_default_tenant(stores.tenants.as_ref(), stores.branding.as_ref())
                .await
                .unwrap();

        let services = CoreServices::new(stores);

        let setup_res = services
            .complete_initial_setup(
                &default_tenant_id,
                &eventrent_core::InitialSetupInput {
                    company_name: "Test Ltda".to_string(),
                    owner_name: "Operador".to_string(),
                    owner_email: "op@luzesom.com.br".to_string(),
                    owner_password: "Password123!".to_string(),
                    mode: eventrent_core::OperationMode::Local,
                },
                30,
            )
            .await
            .unwrap();

        let temp_dir = std::env::temp_dir().join(format!(
            "eventrent_session_test_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let config_path = temp_dir.join("config.json");

        let app_state = AppState {
            services,
            default_tenant_id: default_tenant_id.clone(),
            config_path,
            temp_media_dir: temp_dir.join("temp-media"),
            config: Mutex::new(AppConfig::default()),
            session: Mutex::new(None),
        };

        let base_time = Utc::now();
        let mut session = setup_res.session.clone();
        session.authenticated_at = base_time.to_rfc3339();
        session.last_activity_at = base_time.to_rfc3339();
        app_state.set_session(Some(session));

        // 1. Chamada de polling (get_session_without_touch) 15 minutos depois:
        // A sessão é retornada, MAS last_activity_at NÃO é renovado
        let t15 = base_time + Duration::minutes(15);
        let s15 = app_state
            .get_session_at(t15)
            .expect("Sessão deve estar ativa aos 15 min");
        assert_eq!(
            s15.last_activity_at,
            base_time.to_rfc3339(),
            "Polling não deve alterar last_activity_at"
        );

        // 2. Chamada de polling aos 25 minutos: ainda não renovou
        let t25 = base_time + Duration::minutes(25);
        let s25 = app_state
            .get_session_at(t25)
            .expect("Sessão deve estar ativa aos 25 min");
        assert_eq!(
            s25.last_activity_at,
            base_time.to_rfc3339(),
            "Polling contínuo não pode renovar a sessão"
        );

        // 3. Chamada de polling aos 31 minutos (sem nenhum comando protegido invocado):
        // Expirou!
        let t31 = base_time + Duration::minutes(31);
        let s31 = app_state.get_session_at(t31);
        assert!(
            s31.is_none(),
            "Sessão deve expirar aos 31 min se apenas polling ocorreu"
        );

        // 4. Cenário com comando protegido: reinicia sessão e executa require_session_at aos 20 min
        let mut new_session = setup_res.session.clone();
        new_session.authenticated_at = base_time.to_rfc3339();
        new_session.last_activity_at = base_time.to_rfc3339();
        app_state.set_session(Some(new_session));

        let t20 = base_time + Duration::minutes(20);
        let s_touched = app_state
            .require_session_at(t20)
            .await
            .expect("Comando protegido deve ter sucesso aos 20 min");
        assert_eq!(
            s_touched.last_activity_at,
            t20.to_rfc3339(),
            "Comando protegido DEVE renovar last_activity_at"
        );

        // Agora aos 40 min do início (20 min após o comando protegido), a sessão continua viva:
        let t40 = base_time + Duration::minutes(40);
        let s40 = app_state.get_session_at(t40);
        assert!(
            s40.is_some(),
            "Sessão deve continuar viva porque comando protegido aos 20 min renovou a atividade"
        );
    }

    #[test]
    fn test_media_ipc_frame_decoders_and_temp_cleanup() {
        use crate::commands::media::{
            decode_document_ipc_frame, decode_photo_ipc_frame, write_sanitized_temp_document,
        };
        use eventrent_core::{MAX_DISPLAY_IMAGE_BYTES, MAX_THUMB_IMAGE_BYTES};

        // 1. Frame binário de foto válido
        let display_bytes = b"RIFF____WEBPVP8X..........".to_vec();
        let thumb_bytes = b"RIFF____WEBPVP8X....".to_vec();
        let header_json = serde_json::json!({
            "modelId": "m-1",
            "title": "Painel Frontal",
            "origin": "manufacturer",
            "creditNote": "Pioneer DJ",
            "originalFilename": "cdj-3000.webp",
            "displayMimeType": "image/webp",
            "displayByteLen": display_bytes.len(),
            "thumbMimeType": "image/webp",
            "thumbByteLen": thumb_bytes.len()
        })
        .to_string();
        let header_bytes = header_json.as_bytes();
        let mut frame = Vec::new();
        frame.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        frame.extend_from_slice(header_bytes);
        frame.extend_from_slice(&display_bytes);
        frame.extend_from_slice(&thumb_bytes);

        let decoded_photo = decode_photo_ipc_frame(&frame).unwrap();
        assert_eq!(decoded_photo.model_id, "m-1");
        assert_eq!(decoded_photo.title, "Painel Frontal");
        assert_eq!(decoded_photo.display_bytes, display_bytes);
        assert_eq!(decoded_photo.thumb_bytes, thumb_bytes);

        // 2. Rejeição antes do processamento quando displayByteLen ou frame excede o limite
        let oversized_frame = vec![0u8; MAX_DISPLAY_IMAGE_BYTES + MAX_THUMB_IMAGE_BYTES + 20_000];
        assert!(decode_photo_ipc_frame(&oversized_frame).is_err());

        // 3. Frame binário de documento PDF válido
        let pdf_bytes = b"%PDF-1.7\n%%EOF".to_vec();
        let doc_header_json = serde_json::json!({
            "modelId": "m-1",
            "title": "Manual Técnico",
            "docType": "manual",
            "origin": "manufacturer",
            "creditNote": null,
            "language": "pt-BR",
            "versionNote": "v2.0",
            "originalFilename": "manual.pdf",
            "mimeType": "application/pdf",
            "byteLen": pdf_bytes.len()
        })
        .to_string();
        let doc_header_bytes = doc_header_json.as_bytes();
        let mut doc_frame = Vec::new();
        doc_frame.extend_from_slice(&(doc_header_bytes.len() as u32).to_le_bytes());
        doc_frame.extend_from_slice(doc_header_bytes);
        doc_frame.extend_from_slice(&pdf_bytes);

        let decoded_doc = decode_document_ipc_frame(&doc_frame).unwrap();
        assert_eq!(decoded_doc.title, "Manual Técnico");
        assert_eq!(decoded_doc.bytes, pdf_bytes);

        // 4. Gravação temporária com nome saneado e limpeza ao iniciar (R3) e ao encerrar
        let temp_root = std::env::temp_dir().join(format!(
            "eventrent_temp_media_test_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        cleanup_temp_media_dir_on_shutdown(&temp_root);

        let temp_doc =
            write_sanitized_temp_document(&temp_root, "C:\\temp\\manual:v1?.pdf", &pdf_bytes)
                .unwrap();
        assert!(temp_doc.exists());
        assert_eq!(temp_doc.file_name().unwrap(), "manual_v1_.pdf");

        // Arquivo recente não é apagado quando a idade máxima é 1 hora
        cleanup_temp_media_dir_on_startup(&temp_root, std::time::Duration::from_secs(3600));
        assert!(temp_doc.exists());

        // Arquivo é apagado na inicialização se exceder o limite (simulado com idade máxima de 0 segundos)
        cleanup_temp_media_dir_on_startup(&temp_root, std::time::Duration::from_millis(0));
        assert!(!temp_doc.exists());

        // Nova gravação e limpeza total ao encerrar (shutdown)
        let temp_doc2 = write_sanitized_temp_document(&temp_root, "Manual 2", &pdf_bytes).unwrap();
        assert!(temp_doc2.exists());
        cleanup_temp_media_dir_on_shutdown(&temp_root);
        assert!(!temp_doc2.exists());

        let _ = std::fs::remove_dir_all(&temp_root);
    }
}

#[cfg(test)]
mod session_lifecycle_tests;
