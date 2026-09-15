//! Database connection and schema initialization.

use std::path::Path;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

pub(crate) async fn initialize(app_data_dir: &Path) -> Result<SqlitePool, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
    let options = SqliteConnectOptions::new()
        .filename(app_data_dir.join("koperasi-v2.db"))
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::raw_sql(include_str!("../migrations/001_backend.sql"))
        .execute(&pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(pool)
}
