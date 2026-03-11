pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_workspace_api=info,tower_http=info".into()),
        )
        .compact()
        .init();
}
