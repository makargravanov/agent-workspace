use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    http::{
        access::{require_human_workspace_role, WorkspaceRole},
        actor::ActorContext,
        error::ApiError,
        pagination::PaginationParams,
        request_id::RequestId,
        response::{ApiResponse, ListData, ResponseMeta},
    },
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivityEventResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_github_login: Option<String>,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub event_type: String,
    pub payload_json: Option<String>,
    pub occurred_at: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/activity",
            get(list_workspace_activity),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/activity",
            get(list_project_activity),
        )
}

async fn resolve_workspace(
    pool: &sqlx::AnyPool,
    workspace_slug: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_as::<_, (String,)>("SELECT CAST(id AS TEXT) FROM workspaces WHERE slug = $1")
        .bind(workspace_slug)
        .fetch_optional(pool)
        .await
        .map(|row| row.map(|(id,)| id))
}

async fn resolve_project(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    project_slug: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_as::<_, (String,)>(
        "SELECT CAST(p.id AS TEXT)
         FROM projects p
         WHERE CAST(p.workspace_id AS TEXT) = $1 AND p.slug = $2 AND p.status != 'archived'",
    )
    .bind(workspace_id)
    .bind(project_slug)
    .fetch_optional(pool)
    .await
    .map(|row| row.map(|(id,)| id))
}

async fn list_workspace_activity(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
    pagination: PaginationParams,
) -> Result<ApiResponse<ListData<ActivityEventResponse>>, ApiError> {
    let workspace_id = resolve_workspace(&state.pool, &workspace_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace not found"))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace_id,
        WorkspaceRole::Viewer,
        &request_id,
    )
    .await?;

    let total_sql = "SELECT COUNT(*) FROM audit_events WHERE CAST(workspace_id AS TEXT) = $1";
    let (total,): (i64,) = sqlx::query_as(total_sql)
        .bind(&workspace_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let offset = ((pagination.page - 1) * pagination.per_page) as i64;
    let limit = pagination.per_page as i64;

    let rows: Vec<ActivityEventResponse> = sqlx::query_as(
        "SELECT CAST(ae.id AS TEXT) AS id,
                CAST(ae.workspace_id AS TEXT) AS workspace_id,
                CAST(ae.project_id AS TEXT) AS project_id,
                ae.actor_type,
                CAST(ae.actor_id AS TEXT) AS actor_id,
                (
                    SELECT hi.display_name
                    FROM human_identities hi
                    WHERE hi.workspace_member_id = ae.actor_id
                      AND hi.provider = 'github'
                    ORDER BY hi.updated_at DESC
                    LIMIT 1
                ) AS actor_github_login,
                ae.entity_type,
                CAST(ae.entity_id AS TEXT) AS entity_id,
                ae.event_type,
                CAST(ae.payload_json AS TEXT) AS payload_json,
                CAST(ae.occurred_at AS TEXT) AS occurred_at
         FROM audit_events ae
         WHERE CAST(ae.workspace_id AS TEXT) = $1
         ORDER BY ae.occurred_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(&workspace_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let total_pages = if pagination.per_page == 0 {
        0u32
    } else {
        ((total as f64) / (pagination.per_page as f64)).ceil() as u32
    };

    Ok(ApiResponse {
        data: ListData {
            items: rows,
            next_cursor: if pagination.page < total_pages {
                Some((pagination.page + 1).to_string())
            } else {
                None
            },
        },
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn list_project_activity(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    pagination: PaginationParams,
) -> Result<ApiResponse<ListData<ActivityEventResponse>>, ApiError> {
    let workspace_id = resolve_workspace(&state.pool, &workspace_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace not found"))?;
    let project_id = resolve_project(&state.pool, &workspace_id, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace_id,
        WorkspaceRole::Viewer,
        &request_id,
    )
    .await?;

    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_events
         WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&workspace_id)
    .bind(&project_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let offset = ((pagination.page - 1) * pagination.per_page) as i64;
    let limit = pagination.per_page as i64;

    let rows: Vec<ActivityEventResponse> = sqlx::query_as(
        "SELECT CAST(ae.id AS TEXT) AS id,
                CAST(ae.workspace_id AS TEXT) AS workspace_id,
                CAST(ae.project_id AS TEXT) AS project_id,
                ae.actor_type,
                CAST(ae.actor_id AS TEXT) AS actor_id,
                (
                    SELECT hi.display_name
                    FROM human_identities hi
                    WHERE hi.workspace_member_id = ae.actor_id
                      AND hi.provider = 'github'
                    ORDER BY hi.updated_at DESC
                    LIMIT 1
                ) AS actor_github_login,
                ae.entity_type,
                CAST(ae.entity_id AS TEXT) AS entity_id,
                ae.event_type,
                CAST(ae.payload_json AS TEXT) AS payload_json,
                CAST(ae.occurred_at AS TEXT) AS occurred_at
         FROM audit_events ae
         WHERE CAST(ae.workspace_id AS TEXT) = $1 AND CAST(ae.project_id AS TEXT) = $2
         ORDER BY ae.occurred_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(&workspace_id)
    .bind(&project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let total_pages = if pagination.per_page == 0 {
        0u32
    } else {
        ((total as f64) / (pagination.per_page as f64)).ceil() as u32
    };

    Ok(ApiResponse {
        data: ListData {
            items: rows,
            next_cursor: if pagination.page < total_pages {
                Some((pagination.page + 1).to_string())
            } else {
                None
            },
        },
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{
        app::build_router,
        db::DatabaseBackend,
        state::AppState,
        testing::{
            any_test_pool, assert_api_error_code, assert_status_with_body, json_request,
            seed_workspace_member_project,
        },
    };

    const WS_SLUG: &str = "ops-workspace";
    const PROJECT_SLUG: &str = "ops-project";

    async fn setup(role: &str) -> (axum::Router, sqlx::AnyPool, String, String) {
        let pool = any_test_pool().await;
        let scope = seed_workspace_member_project(
            &pool,
            WS_SLUG,
            "Ops Workspace",
            PROJECT_SLUG,
            "Ops Project",
            "test:activity-user",
            "Activity User",
            role,
        )
        .await;
        let router = build_router(AppState::new(pool.clone(), DatabaseBackend::Sqlite));
        (router, pool, scope.member_id, scope.workspace_id)
    }

    #[tokio::test]
    async fn workspace_activity_contains_agent_event() {
        let (app, _, member_id, _) = setup("owner").await;
        let create = app
            .clone()
            .oneshot(json_request(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/workspaces/{WS_SLUG}/agents"))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id),
                &json!({ "key": "ops-bot", "display_name": "Ops Bot" }),
            ))
            .await
            .unwrap();
        let _ = assert_status_with_body(create, StatusCode::CREATED).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/workspaces/{WS_SLUG}/activity"))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::OK).await;
        assert!(body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["entity_type"].as_str() == Some("agent")));
    }

    #[tokio::test]
    async fn project_activity_is_scoped_to_project() {
        let (app, pool, member_id, workspace_id) = setup("owner").await;
        sqlx::query(
            "INSERT INTO human_identities
             (id, workspace_member_id, provider, provider_subject, display_name)
             VALUES ($1, $2, 'github', 'github:user:6206084', 'makargravanov')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&member_id)
        .execute(&pool)
        .await
        .unwrap();
        let create = app
            .clone()
            .oneshot(json_request(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/projects/{PROJECT_SLUG}/documents"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .header("x-workspace-id", &workspace_id),
                &json!({
                    "slug": "runbook",
                    "title": "Runbook",
                    "body_md": "# Runbook"
                }),
            ))
            .await
            .unwrap();
        let _ = assert_status_with_body(create, StatusCode::CREATED).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/projects/{PROJECT_SLUG}/activity"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::OK).await;
        assert!(body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["project_id"].is_string()));
        assert!(body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["actor_github_login"].as_str() == Some("makargravanov")));
    }

    #[tokio::test]
    async fn workspace_activity_paginates() {
        let (app, _, member_id, _) = setup("owner").await;
        for key in ["ops-bot-a", "ops-bot-b"] {
            let create = app
                .clone()
                .oneshot(json_request(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/workspaces/{WS_SLUG}/agents"))
                        .header("x-actor-kind", "human")
                        .header("x-actor-id", &member_id),
                    &json!({ "key": key, "display_name": key }),
                ))
                .await
                .unwrap();
            let _ = assert_status_with_body(create, StatusCode::CREATED).await;
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/activity?per_page=1&page=1"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::OK).await;
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"]["next_cursor"].as_str(), Some("2"));
    }

    #[tokio::test]
    async fn non_member_cannot_read_activity() {
        let (app, _, _, _) = setup("owner").await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/workspaces/{WS_SLUG}/activity"))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", Uuid::new_v4().to_string())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::FORBIDDEN).await;
        assert_api_error_code(&body, "forbidden");
    }
}
