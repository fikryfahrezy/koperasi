//! Database connection and schema initialization.

use std::path::Path;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

/// Berkas database buku besar. Versi sebelumnya (`koperasi-v2.db`) menyimpan
/// saldo yang diubah di tempat; berkas itu dibiarkan apa adanya sebagai arsip.
const DATABASE_FILE: &str = "koperasi-v3.db";

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

/// Membuat skema buku besar. Aman dijalankan berulang.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::raw_sql(include_str!("../migrations/001_ledger.sql"))
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
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
