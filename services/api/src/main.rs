use agent_workspace_api::{
    app::{build_router, AppState},
    db::{build_sqlite_pool, DatabaseConfig},
    telemetry::init_tracing,
};
use std::env;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();

    let db_cfg = DatabaseConfig::from_env()
        .expect("failed to load database config: DATABASE_URL must be set");

    // BL-11: SQLite-first for local/dev; Postgres path to be wired after BL-10.
    let pool = build_sqlite_pool(&db_cfg)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("./migrations_sqlite")
        .run(&pool)
        .await
        .expect("failed to run database migrations");

    info!("database migrations applied");

    let state = AppState { pool };

    let bind_address = env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .expect("failed to bind API listener");

    info!(address = %bind_address, "agent-workspace-api listening");

    axum::serve(listener, build_router(state))
        .await
        .expect("failed to serve API");
}
