use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::env;
use tracing::info;

#[derive(Serialize)]
struct ServiceOverview {
    name: &'static str,
    status: &'static str,
    architecture: &'static str,
    primary_database: &'static str,
    search_strategy: &'static str,
}

#[derive(Serialize)]
struct HealthStatus {
    status: &'static str,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let bind_address = env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .expect("failed to bind API listener");

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    info!(address = %bind_address, "agent-workspace-api listening");

    axum::serve(listener, app)
        .await
        .expect("failed to serve API");
}

async fn root() -> Json<ServiceOverview> {
    Json(ServiceOverview {
        name: "agent-workspace-api",
        status: "bootstrap",
        architecture: "modular-monolith",
        primary_database: "postgresql",
        search_strategy: "hybrid-postgres-full-text-plus-pgvector",
    })
}

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_workspace_api=info,tower_http=info".into()),
        )
        .compact()
        .init();
}
