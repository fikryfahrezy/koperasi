//! Tauri application composition and plugin registration.

use tauri::Manager;

use crate::{database, state::AppState};

pub(crate) fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(std::io::Error::other)?;
            let pool = tauri::async_runtime::block_on(database::initialize(&app_data_dir))
                .map_err(std::io::Error::other)?;
            app.manage(AppState { db: pool });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_upload::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            crate::commands::get_app_snapshot,
            crate::commands::import_workbook,
            crate::commands::add_member,
            crate::commands::preview_loan,
            crate::commands::create_loan,
            crate::commands::disburse_loan,
            crate::commands::post_savings_transaction,
            crate::commands::post_payment,
            crate::commands::reverse_transaction,
            crate::commands::get_admin_state,
            crate::commands::save_financial_parameters
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
