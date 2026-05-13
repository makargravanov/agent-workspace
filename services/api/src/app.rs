use axum::{extract::DefaultBodyLimit, Router};
use tower_http::trace::TraceLayer;

use crate::http::request_id::request_id_layer;
use crate::modules;
use crate::state::AppState;

const API_BODY_LIMIT_BYTES: usize = 25 * 1024 * 1024;

pub fn build_router(state: AppState) -> Router {
    Router::<AppState>::new()
        .merge(modules::system::public_routes())
        .nest("/api/v1", api_v1_router())
        .layer(TraceLayer::new_for_http())
        .layer(request_id_layer())
        .with_state(state)
}

fn api_v1_router() -> Router<AppState> {
    Router::new()
        .merge(modules::system::api_routes())
        .merge(modules::auth::routes())
        .merge(modules::workspace_core::routes())
        .merge(modules::workspace_admin::routes())
        .merge(modules::task_management::routes())
        .merge(modules::task_structure::routes())
        .merge(modules::assets::routes())
        .merge(modules::agents::routes())
        .merge(modules::agent_credentials::routes())
        .merge(modules::documents::routes())
        .merge(modules::activity::routes())
        .merge(modules::knowledge_base::routes())
        .merge(modules::integration_connections::routes())
        .merge(modules::search_indexing::routes())
        .merge(modules::agent_access::routes())
        .merge(modules::github_integration::routes())
        .merge(modules::mcp_access::routes())
        .merge(modules::operator::routes())
        .layer(DefaultBodyLimit::max(API_BODY_LIMIT_BYTES))
}

#[cfg(test)]
mod tests {
    use super::build_router;
    use crate::{state::AppState, testing::any_test_pool};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        build_router(AppState::new(
            any_test_pool().await,
            crate::db::DatabaseBackend::Sqlite,
        ))
    }

    #[tokio::test]
    async fn root_overview_is_available() {
        let response = test_app()
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
        let response = test_app()
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
