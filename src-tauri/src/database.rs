//! Database connection and schema initialization.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Executor, SqlitePool,
};

/// Cooperative database file used during development.
const DATABASE_FILE: &str = "koperasi.db";

pub(crate) async fn initialize(app_data_dir: &Path) -> Result<SqlitePool, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
    let options = SqliteConnectOptions::new()
        .filename(database_path(app_data_dir))
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|error| error.to_string())?;
    apply_schema(&pool).await?;
    Ok(pool)
}

/// The live database file inside the app data folder.
pub(crate) fn database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(DATABASE_FILE)
}

/// Local time as `YYYYMMDD-HHMMSS`, for backup file names.
pub(crate) async fn local_timestamp(pool: &SqlitePool) -> Result<String, String> {
    sqlx::query_scalar("SELECT strftime('%Y%m%d-%H%M%S', 'now', 'localtime')")
        .fetch_one(pool)
        .await
        .map_err(|error| error.to_string())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.to_string()),
        _ => Ok(()),
    }
}

/// Writes a consistent copy of the whole database to `destination`, replacing
/// the file there.
pub(crate) async fn backup(pool: &SqlitePool, destination: &Path) -> Result<(), String> {
    // VACUUM INTO refuses an existing file, so write beside it, then move it in.
    let staging = with_suffix(destination, ".tmp");
    remove_if_exists(&staging)?;
    let written = sqlx::query("VACUUM INTO ?")
        .bind(staging.to_string_lossy().into_owned())
        .execute(pool)
        .await
        .map_err(|error| error.to_string())
        .and_then(|_| std::fs::rename(&staging, destination).map_err(|error| error.to_string()));
    if written.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    written
}

const NOT_A_DATABASE: &str = "File ini bukan database koperasi.";

/// Copies `source` into the data folder, checks that it is a cooperative
/// database this version can open, and upgrades it to the current schema.
/// Returns the prepared file for [`replace`]; the live database is untouched.
pub(crate) async fn prepare_import(source: &Path, app_data_dir: &Path) -> Result<PathBuf, String> {
    let staging = app_data_dir.join("koperasi-impor.db.tmp");
    remove_if_exists(&staging)?;
    remove_if_exists(&with_suffix(&staging, "-journal"))?;
    std::fs::copy(source, &staging).map_err(|error| error.to_string())?;
    let checked = check_and_upgrade(&staging).await;
    if checked.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    checked.map(|_| staging)
}

async fn check_and_upgrade(path: &Path) -> Result<(), String> {
    // Rollback journal: the prepared file must stand alone, without -wal/-shm.
    let options = SqliteConnectOptions::new()
        .filename(path)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Delete);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|_| NOT_A_DATABASE.to_string())?;
    let checked = async {
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .map_err(|_| NOT_A_DATABASE.to_string())?;
        if integrity != "ok" {
            return Err(format!("File database rusak: {integrity}"));
        }
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('companies', 'members', 'schema_migrations')",
        )
        .fetch_all(&pool)
        .await
        .map_err(|error| error.to_string())?;
        if !["companies", "members"].iter().all(|table| tables.iter().any(|name| name == table)) {
            return Err(NOT_A_DATABASE.to_string());
        }
        if tables.iter().any(|name| name == "schema_migrations") {
            let applied: Vec<String> = sqlx::query_scalar("SELECT name FROM schema_migrations")
                .fetch_all(&pool)
                .await
                .map_err(|error| error.to_string())?;
            if applied.iter().any(|name| !MIGRATIONS.iter().any(|(known, _)| known == name)) {
                return Err("File ini dibuat oleh versi aplikasi yang lebih baru. Perbarui aplikasi dulu, lalu impor lagi.".to_string());
            }
        }
        apply_schema(&pool).await
    }
    .await;
    pool.close().await;
    checked
}

/// Puts a file from [`prepare_import`] in place of the live database. Close
/// the live pool first.
pub(crate) fn replace(app_data_dir: &Path, prepared: &Path) -> Result<(), String> {
    let live = database_path(app_data_dir);
    // A leftover WAL of the old file must not be replayed onto the new one.
    for suffix in ["-wal", "-shm", "-journal"] {
        remove_if_exists(&with_suffix(&live, suffix))?;
    }
    std::fs::rename(prepared, &live).map_err(|error| error.to_string())
}

/// Schema changes (DDL), from `migrations/*.sql`.
const MIGRATIONS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/migrations.rs"));
/// Seeds and fixes to existing data, from `data/*.sql`.
const DATA_SCRIPTS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/data.rs"));

/// Applies the migrations, then the data scripts. Both folders are embedded at
/// build time (see `build.rs`); each file runs once, in file-name order, and is
/// recorded so a later start skips it.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<(), String> {
    run_once(pool, "schema_migrations", MIGRATIONS).await?;
    run_once(pool, "applied_data_scripts", DATA_SCRIPTS).await
}

async fn run_once(pool: &SqlitePool, table: &str, scripts: &[(&str, &str)]) -> Result<(), String> {
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {table} (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)"
    ))
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    for (name, sql) in scripts {
        let mut db = pool.begin().await.map_err(|error| error.to_string())?;
        let applied: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS (SELECT 1 FROM {table} WHERE name = ?)"
        ))
        .bind(name)
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
        if applied {
            continue;
        }
        // `Executor::execute` returns a boxed `Send` future; `RawSql::execute`
        // is an `async fn` whose future rustc cannot prove `Send` here.
        (&mut *db)
            .execute(sqlx::raw_sql(sql))
            .await
            .map_err(|error| format!("{name} gagal dijalankan: {error}"))?;
        sqlx::query(&format!("INSERT INTO {table} (name) VALUES (?)"))
            .bind(name)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        db.commit().await.map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) async fn memory_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    apply_schema(&pool).await.unwrap();
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_a_database_created_before_tracked_migrations() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            // Installs up to 0.15.9 ran 001 on every start without recording it.
            sqlx::raw_sql(MIGRATIONS[0].1).execute(&pool).await.unwrap();
            apply_schema(&pool).await.unwrap();
            apply_schema(&pool).await.unwrap();
            let recorded: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schema_migrations")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(recorded as usize, MIGRATIONS.len());
        });
    }

    #[test]
    fn runs_each_data_script_once() {
        tauri::async_runtime::block_on(async {
            let pool = memory_pool().await;
            // Scripts recorded by an older install whose files are gone are ignored.
            sqlx::query("INSERT INTO applied_data_scripts (name) VALUES ('001_removed')")
                .execute(&pool)
                .await
                .unwrap();
            let scripts = [(
                "999_test",
                "INSERT INTO members (id, company_id, name, joined_at) VALUES ('default-M999', 'default', 'Test', '2026-01-01')",
            )];
            run_once(&pool, "applied_data_scripts", &scripts)
                .await
                .unwrap();
            sqlx::query("DELETE FROM members WHERE id = 'default-M999'")
                .execute(&pool)
                .await
                .unwrap();
            // Already applied: running again does not insert the member again.
            run_once(&pool, "applied_data_scripts", &scripts)
                .await
                .unwrap();
            let members: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE id = 'default-M999'")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(members, 0);
        });
    }

    #[test]
    fn imports_a_backup_of_the_database() {
        tauri::async_runtime::block_on(async {
            let dir = tempfile::tempdir().unwrap();
            // File-backed: VACUUM INTO from an in-memory database writes in memory too.
            let pool = initialize(&dir.path().join("asal")).await.unwrap();
            sqlx::query("INSERT INTO members (id, company_id, name, joined_at) VALUES ('default-M999', 'default', 'Test', '2026-01-01')")
                .execute(&pool)
                .await
                .unwrap();
            let file = dir.path().join("cadangan.db");
            std::fs::write(&file, "isi lama").unwrap();
            backup(&pool, &file).await.unwrap();

            let prepared = prepare_import(&file, dir.path()).await.unwrap();
            replace(dir.path(), &prepared).unwrap();
            let restored = initialize(dir.path()).await.unwrap();
            let name: String =
                sqlx::query_scalar("SELECT name FROM members WHERE id = 'default-M999'")
                    .fetch_one(&restored)
                    .await
                    .unwrap();
            assert_eq!(name, "Test");
        });
    }

    #[test]
    fn refuses_files_that_are_not_a_usable_database() {
        tauri::async_runtime::block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let text = dir.path().join("catatan.db");
            std::fs::write(&text, "bukan database").unwrap();
            assert_eq!(
                prepare_import(&text, dir.path()).await.unwrap_err(),
                NOT_A_DATABASE
            );

            // A backup from a newer version has migrations this one does not know.
            let pool = initialize(&dir.path().join("asal")).await.unwrap();
            sqlx::query("INSERT INTO schema_migrations (name) VALUES ('999_future')")
                .execute(&pool)
                .await
                .unwrap();
            let newer = dir.path().join("baru.db");
            backup(&pool, &newer).await.unwrap();
            assert!(prepare_import(&newer, dir.path())
                .await
                .unwrap_err()
                .contains("lebih baru"));
            assert!(!dir.path().join("koperasi-impor.db.tmp").exists());
        });
    }
}
