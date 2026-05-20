use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create_loans_table",
            sql: "CREATE TABLE IF NOT EXISTS loans (
                id TEXT PRIMARY KEY,
                member_name TEXT NOT NULL,
                amount INTEGER NOT NULL,
                date TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Berjalan'
            );",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "seed_loans",
            sql: "INSERT OR IGNORE INTO loans (id, member_name, amount, date, status) VALUES
                ('PINJ-260301', 'Bapak Sugeng Riadi', 5000000, '01 Mar 2026', 'Berjalan'),
                ('PINJ-260215', 'Ibu Ratna', 12000000, '15 Feb 2026', 'Berjalan'),
                ('PINJ-251210', 'Sukirman', 3000000, '10 Des 2025', 'Lunas');",
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_store::Builder::new().build())
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_upload::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(
            tauri_plugin_sql::Builder::new()
                .add_migrations("sqlite:koperasi.db", migrations)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
