pub mod commands;

use chrono::Utc;
use eventrent_core::{
    ensure_default_tenant, ensure_system_roles, AppConfig, CoreServices, SessionInfo,
};
use eventrent_store_sqlite::{init_db, SqliteStore};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub struct AppState {
    pub services: CoreServices,
    pub default_tenant_id: String,
    pub config_path: PathBuf,
    config: Mutex<AppConfig>,
    session: Mutex<Option<SessionInfo>>,
}

impl AppState {
    pub fn get_config(&self) -> Result<AppConfig, String> {
        self.config
            .lock()
            .map(|c| c.clone())
            .map_err(|_| "Falha ao acessar estado de configuração.".to_string())
    }

    pub fn save_config(&self, new_cfg: &AppConfig) -> Result<(), String> {
        save_app_config_file(&self.config_path, new_cfg)?;
        if let Ok(mut guard) = self.config.lock() {
            *guard = new_cfg.clone();
        }
        Ok(())
    }

    pub fn set_session(&self, session: Option<SessionInfo>) {
        if let Ok(mut guard) = self.session.lock() {
            *guard = session;
        }
    }

    pub fn touch_and_get_session(&self) -> Option<SessionInfo> {
        let mut guard = self.session.lock().ok()?;
        let current = guard.as_mut()?;

        if self.services.verify_session_active(current).is_err() {
            *guard = None;
            return None;
        }

        current.last_activity_at = Utc::now().to_rfc3339();
        Some(current.clone())
    }

    pub fn require_session(&self) -> Result<SessionInfo, String> {
        self.touch_and_get_session()
            .ok_or_else(|| "Sessão expirada ou usuário não autenticado.".to_string())
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

pub fn save_app_config_file(config_path: &Path, config: &AppConfig) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Falha ao serializar config.json: {e}"))?;
    std::fs::write(config_path, serialized).map_err(|e| {
        format!(
            "Falha ao gravar arquivo de configuração ({}): {e}",
            config_path.display()
        )
    })
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

    ensure_system_roles(&default_tenant_id, store_bundle.roles.as_ref())
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

    let services = CoreServices::new(store_bundle);

    Ok((
        AppState {
            services,
            default_tenant_id,
            config_path,
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

                        // Usa `.show(...)` não bloqueante em vez de `.blocking_show()` para não
                        // causar deadlock na thread principal antes do início do event loop.
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
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_current_session,
            commands::auth::recover_password,
            commands::users::list_users,
            commands::users::create_user,
            commands::users::update_user,
            commands::users::set_user_active,
            commands::users::reset_user_password,
            commands::users::list_roles,
            commands::branding::get_branding,
            commands::branding::save_branding,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = app_result {
        tracing::error!("Aplicação Tauri encerrada com erro: {}", e);
    }
}
