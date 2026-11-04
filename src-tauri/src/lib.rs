pub mod commands;

use eventrent_core::DbPool;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub struct AppState {
    pub pool: DbPool,
    pub default_tenant_id: String,
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

    let db_path = app_dir.join("eventrent.db");
    tracing::info!(
        "Inicializando banco SQLite em: {}",
        db_path.to_string_lossy()
    );

    let pool = eventrent_core::init_db(&db_path).await.map_err(|e| {
        tracing::error!("Erro crítico ao inicializar banco SQLite: {}", e);
        (
            format!("Falha ao inicializar ou migrar o banco de dados local: {e}"),
            log_dir_display.clone(),
        )
    })?;

    let default_tenant_id = eventrent_core::ensure_default_tenant(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Erro crítico ao garantir tenant padrão: {}", e);
            (
                format!("Falha ao configurar dados iniciais do tenant padrão: {e}"),
                log_dir_display.clone(),
            )
        })?;

    tracing::info!(
        "Banco SQLite inicializado e tenant padrão ({}) verificado com sucesso",
        default_tenant_id
    );

    Ok((
        AppState {
            pool,
            default_tenant_id,
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

                        app_handle
                            .dialog()
                            .message(format!(
                                "Não foi possível iniciar o EventRent.\n\nMotivo: {err_msg}\n\nArquivo de log: {log_path_display}"
                            ))
                            .title("EventRent — Erro de Inicialização")
                            .kind(MessageDialogKind::Error)
                            .buttons(MessageDialogButtons::Ok)
                            .blocking_show();

                        Err(Box::<dyn std::error::Error>::from(err_msg))
                    }
                }
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::branding::get_branding,
            commands::branding::save_branding,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = app_result {
        tracing::error!("Aplicação Tauri encerrada com erro: {}", e);
    }
}
