use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

const DOMAIN_MODULES: &[&str] = &[
    "auth",
    "workspace-core",
    "workspace-admin",
    "task-management",
    "task-structure",
    "knowledge-base",
    "search-indexing",
    "agent-access",
    "github-integration",
    "mcp-access",
    "operator",
];

#[derive(Serialize)]
struct ServiceOverview {
    name: &'static str,
    status: &'static str,
    architecture: &'static str,
    api_base_path: &'static str,
    primary_database: &'static str,
    search_strategy: &'static str,
    modules: &'static [&'static str],
}

#[derive(Serialize)]
struct HealthStatus {
    status: &'static str,
}

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/", get(root))
}

pub fn api_routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn root() -> Json<ServiceOverview> {
    Json(ServiceOverview {
        name: "agent-workspace-api",
        status: "bootstrap",
        architecture: "modular-monolith",
        api_base_path: "/api/v1",
        primary_database: "postgresql",
        search_strategy: "full-text-first-semantic-deferred",
        modules: DOMAIN_MODULES,
    })
}

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}
