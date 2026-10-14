//! Database connection and schema initialization.

use std::path::Path;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

/// Cooperative database file used during development.
const DATABASE_FILE: &str = "koperasi.db";

pub(crate) async fn initialize(app_data_dir: &Path) -> Result<SqlitePool, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
    let options = SqliteConnectOptions::new()
        .filename(app_data_dir.join(DATABASE_FILE))
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
        sqlx::raw_sql(sql)
            .execute(&mut *db)
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
            run_once(&pool, "applied_data_scripts", &scripts).await.unwrap();
            sqlx::query("DELETE FROM members WHERE id = 'default-M999'")
                .execute(&pool)
                .await
                .unwrap();
            // Already applied: running again does not insert the member again.
            run_once(&pool, "applied_data_scripts", &scripts).await.unwrap();
            let members: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE id = 'default-M999'")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(members, 0);
        });
    }
}
