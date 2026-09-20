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

    let company_column_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('members') WHERE name = 'company_id'",
    )
    .fetch_one(&pool)
    .await
    .map_err(|error| error.to_string())?;
    if company_column_exists == 0 {
        sqlx::raw_sql(include_str!("../migrations/002_multi_company.sql"))
            .execute(&pool)
            .await
            .map_err(|error| error.to_string())?;
    }

    let enum_checks_exist: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'members' AND sql LIKE '%status IN%'",
    )
    .fetch_one(&pool)
    .await
    .map_err(|error| error.to_string())?;
    if enum_checks_exist > 0 {
        sqlx::raw_sql(include_str!("../migrations/003_remove_enum_checks.sql"))
            .execute(&pool)
            .await
            .map_err(|error| error.to_string())?;
    }

    Ok(pool)
}
