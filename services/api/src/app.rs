use axum::Router;
use tower_http::trace::TraceLayer;

use crate::http::request_id::request_id_layer;
use crate::modules;

/// Shared application state injected into every handler via Axum's `State`
/// extractor.  The `AnyPool` abstracts over Postgres (production) and SQLite
/// (local dev / tests) so that the same handler code works in both
/// environments.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::AnyPool,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(modules::system::public_routes())
        .nest("/api/v1", api_v1_router(state))
        .layer(TraceLayer::new_for_http())
        .layer(request_id_layer())
}

fn api_v1_router(state: AppState) -> Router {
    Router::new()
        .merge(modules::system::api_routes())
        .merge(modules::auth::routes())
        .merge(modules::workspace_core::routes())
        .merge(modules::workspace_admin::routes())
        .merge(modules::task_management::routes())
        .merge(modules::task_structure::routes())
        .merge(modules::knowledge_base::routes(state.clone()))
        .merge(modules::search_indexing::routes())
        .merge(modules::agent_access::routes())
        .merge(modules::github_integration::routes())
        .merge(modules::mcp_access::routes())
        .merge(modules::operator::routes())
}

#[cfg(test)]
mod tests {
    use super::{build_router, AppState};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn test_router() -> axum::Router {
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test AnyPool (SQLite) should open");
        sqlx::migrate!("./migrations_sqlite")
            .run(&pool)
            .await
            .expect("SQLite migrations should apply");
        build_router(AppState { pool })
    }

    #[tokio::test]
    async fn root_overview_is_available() {
        let response = test_router()
            .await
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_health_is_available_under_versioned_namespace() {
        let response = test_router()
            .await
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
