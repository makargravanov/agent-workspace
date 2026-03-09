use std::env;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();

    let mode = env::args().nth(1).unwrap_or_else(|| "stdio".to_string());
    let api_base_url = env::var("AGENT_WORKSPACE_API_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    info!(mode = %mode, api_base_url = %api_base_url, "agent-workspace-mcp bootstrap started");
    info!(
        tools = "search_tasks,get_document,update_plan,create_note,sync_github",
        "planned MCP surface"
    );

    tokio::signal::ctrl_c()
        .await
        .expect("failed to wait for shutdown signal");
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_workspace_mcp=info".into()),
        )
        .compact()
        .init();
}
