use agent_workspace_api::{
    db::{build_any_pool, DatabaseBackend, DatabaseConfig},
    app::build_router,
    state::AppState,
    telemetry::init_tracing,
};
use std::env;
use sqlx::migrate::MigrateError;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();

    // Register Postgres and SQLite drivers for AnyPool before the first connection
    // is attempted.  The active driver is selected by the DATABASE_URL scheme.
    sqlx::any::install_default_drivers();

    let db_cfg = DatabaseConfig::from_env()
        .expect("failed to load database config: DATABASE_URL must be set");

    let pool = build_any_pool(&db_cfg)
        .await
        .expect("failed to connect to database");

    // Run the correct migration set depending on which backend is active.
    // Postgres uses ./migrations, SQLite uses ./migrations_sqlite.
    let migration_result = match db_cfg.backend {
        DatabaseBackend::Sqlite => sqlx::migrate!("./migrations_sqlite").run(&pool).await,
        DatabaseBackend::Postgres => sqlx::migrate!("./migrations").run(&pool).await,
    };
    if let Err(error) = migration_result {
        match error {
            MigrateError::VersionMismatch(version) => {
                panic!(
                    "failed to run database migrations: VersionMismatch({version}). Existing database migration checksums do not match repository files"
                );
            }
            other => panic!("failed to run database migrations: {other}"),
        }
    }

    info!("database migrations applied");

    let bind_address = env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .expect("failed to bind API listener");

    info!(address = %bind_address, "agent-workspace-api listening");

    axum::serve(listener, build_router(AppState::new(pool, db_cfg.backend)))
        .await
        .expect("failed to serve API");
}
