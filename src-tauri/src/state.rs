use std::path::PathBuf;

use sqlx::SqlitePool;

pub(crate) struct AppState {
    pub(crate) db: SqlitePool,
    /// Folder holding the database file and the copies kept before an import.
    pub(crate) data_dir: PathBuf,
}
