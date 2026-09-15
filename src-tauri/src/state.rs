use sqlx::SqlitePool;

pub(crate) struct AppState {
    pub(crate) db: SqlitePool,
}
