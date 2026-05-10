use crate::db::DatabaseBackend;
use std::path::PathBuf;

/// Shared application state injected into every handler via `axum::extract::State`.
///
/// The runtime uses `sqlx::AnyPool` so the same handler code can run against
/// PostgreSQL and SQLite.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::AnyPool,
    pub db_backend: DatabaseBackend,
    pub asset_storage_dir: PathBuf,
}

impl AppState {
    pub fn new(pool: sqlx::AnyPool, db_backend: DatabaseBackend) -> Self {
        let asset_storage_dir = std::env::var("ASSET_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("agent-workspace-assets"));

        Self::new_with_asset_storage(pool, db_backend, asset_storage_dir)
    }

    pub fn new_with_asset_storage(
        pool: sqlx::AnyPool,
        db_backend: DatabaseBackend,
        asset_storage_dir: PathBuf,
    ) -> Self {
        Self {
            pool,
            db_backend,
            asset_storage_dir,
        }
    }
}

#[cfg(test)]
impl AppState {
    /// Build a lazy SQLite-backed AnyPool for tests that validate inputs before
    /// any real query is executed.
    pub fn new_lazy_for_test() -> Self {
        sqlx::any::install_default_drivers();

        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect_lazy("sqlite::memory:")
            .expect("lazy AnyPool creation should not fail at URL-parse time");

        Self {
            pool,
            db_backend: DatabaseBackend::Sqlite,
            asset_storage_dir: std::env::temp_dir().join("agent-workspace-assets"),
        }
    }
}
