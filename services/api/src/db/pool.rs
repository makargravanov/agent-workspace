use super::config::DatabaseConfig;
use sqlx::any::AnyPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;

pub async fn build_pool(cfg: &DatabaseConfig) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .connect(&cfg.url)
        .await
}

pub async fn build_sqlite_pool(cfg: &DatabaseConfig) -> Result<sqlx::SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .connect(&cfg.url)
        .await
}

/// Build a database-agnostic pool backed by either Postgres or SQLite depending
/// on the `DATABASE_URL` scheme.  Callers must invoke
/// [`sqlx::any::install_default_drivers`] before calling this function.
pub async fn build_any_pool(cfg: &DatabaseConfig) -> Result<sqlx::AnyPool, sqlx::Error> {
    AnyPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .connect(&cfg.url)
        .await
}
