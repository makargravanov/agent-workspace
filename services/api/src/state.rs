/// Shared application state injected into every handler via `axum::extract::State`.
///
/// The runtime uses `sqlx::AnyPool` so the same handler code can run against
/// PostgreSQL and SQLite.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::AnyPool,
}

impl AppState {
    pub fn new(pool: sqlx::AnyPool) -> Self {
        Self { pool }
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

        Self { pool }
    }
}
