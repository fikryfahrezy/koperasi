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
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_companies,
            crate::commands::get_app_snapshot,
            crate::commands::get_monthly_ledger,
            crate::commands::get_cash_book,
            crate::commands::post_cash_entry,
            crate::commands::add_member,
            crate::commands::preview_loan,
            crate::commands::create_loan,
            crate::commands::post_savings_transaction,
            crate::commands::post_payment,
            crate::commands::reverse_transaction,
            crate::commands::get_financial_parameters
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
