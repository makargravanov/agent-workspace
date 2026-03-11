use super::config::DatabaseConfig;
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
