/// Shared application state injected into every handler via `axum::extract::State`.
///
/// All handlers that need database access receive this through `State<AppState>`.
/// Stateless handlers (health, root) simply ignore it.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
}

impl AppState {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(test)]
impl AppState {
    /// Build a state with a lazy (non-connecting) pool for tests that exercise
    /// handlers which validate input before any database access.
    ///
    /// The pool will never make an actual connection during the test because
    /// validation failures return early before any query is executed.
    pub fn new_lazy_for_test() -> Self {
        use sqlx::postgres::PgPoolOptions;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/nonexistent_test_db_bl20")
            .expect("lazy PgPool creation should not fail at URL-parse time");
        Self { pool }
    }
}
