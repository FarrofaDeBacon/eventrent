pub mod commands;

use eventrent_core::DbPool;
use tauri::Manager;

pub struct AppState {
    pub pool: DbPool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let app_dir = app_handle
                    .path()
                    .app_data_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));

                if let Err(e) = std::fs::create_dir_all(&app_dir) {
                    tracing::warn!("Falha ao criar diretório do aplicativo: {}", e);
                }

                let db_path = app_dir.join("eventrent.db");
                let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

                tracing::info!("Inicializando banco SQLite em: {}", db_url);

                match eventrent_core::init_db(&db_url).await {
                    Ok(pool) => {
                        app_handle.manage(AppState { pool });
                        tracing::info!("Banco SQLite inicializado e migrado com sucesso");
                    }
                    Err(e) => {
                        tracing::error!("Erro crítico ao inicializar banco: {}", e);
                        panic!("Não foi possível inicializar o banco de dados: {e}");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::branding::get_branding,
            commands::branding::save_branding,
        ])
        .run(tauri::generate_context!())
        .expect("Erro ao executar a aplicação Tauri");
}
