use agent_workspace_api::{app::build_router, telemetry::init_tracing};
use std::env;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();

    let bind_address = env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .expect("failed to bind API listener");

    info!(address = %bind_address, "agent-workspace-api listening");

    axum::serve(listener, build_router())
        .await
        .expect("failed to serve API");
}
