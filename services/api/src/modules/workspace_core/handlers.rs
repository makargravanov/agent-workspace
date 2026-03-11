use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::app::AppState;
use crate::http::audit::{emit_audit, AuditEvent};
use crate::http::error::ApiError;
use crate::http::request_id::RequestId;
use crate::http::response::{ApiResponse, Created, ListData, ResponseMeta};

use super::{domain, repo};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route("/workspaces/{workspace_slug}", get(get_workspace))
        .route(
            "/workspaces/{workspace_slug}/projects",
            get(list_projects).post(create_project),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}",
            get(get_project),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers — Workspaces
// ---------------------------------------------------------------------------

async fn list_workspaces(
    State(state): State<AppState>,
    request_id: RequestId,
) -> Result<ApiResponse<ListData<domain::Workspace>>, ApiError> {
    let items = repo::list_workspaces(&state.pool).await.map_err(|e| {
        tracing::error!(error = %e, "list_workspaces db error");
        ApiError::internal(&request_id.0, "failed to list workspaces")
    })?;

    Ok(ApiResponse {
        data: ListData { items, next_cursor: None },
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

async fn create_workspace(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: crate::http::actor::ActorContext,
    Json(body): Json<domain::CreateWorkspaceRequest>,
) -> Result<Created<domain::Workspace>, ApiError> {
    validate_slug(&body.slug, &request_id)?;

    let id = Uuid::new_v4();
    let workspace =
        repo::insert_workspace(&state.pool, &id.to_string(), &body.slug, &body.name)
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(ref db_err) if is_unique_violation(db_err.as_ref()) => {
                    ApiError::validation_error(
                        &request_id.0,
                        format!("workspace slug '{}' is already taken", body.slug),
                    )
                }
                other => {
                    tracing::error!(error = %other, "create_workspace db error");
                    ApiError::internal(&request_id.0, "failed to create workspace")
                }
            })?;

    emit_audit(AuditEvent {
        request_id: request_id.0.clone(),
        actor,
        action: "workspace.created".to_string(),
        resource_kind: "workspace".to_string(),
        resource_id: workspace.id.clone(),
        payload: None,
    });

    Ok(Created(ApiResponse {
        data: workspace,
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    }))
}

async fn get_workspace(
    State(state): State<AppState>,
    request_id: RequestId,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<domain::Workspace>, ApiError> {
    let workspace = repo::get_workspace_by_slug(&state.pool, &workspace_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "get_workspace db error");
            ApiError::internal(&request_id.0, "failed to fetch workspace")
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                &request_id.0,
                format!("workspace '{workspace_slug}' not found"),
            )
        })?;

    Ok(ApiResponse {
        data: workspace,
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

// ---------------------------------------------------------------------------
// Handlers — Projects
// ---------------------------------------------------------------------------

async fn list_projects(
    State(state): State<AppState>,
    request_id: RequestId,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<ListData<domain::Project>>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;

    let items = repo::list_projects(&state.pool, &workspace.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "list_projects db error");
            ApiError::internal(&request_id.0, "failed to list projects")
        })?;

    Ok(ApiResponse {
        data: ListData { items, next_cursor: None },
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

async fn create_project(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: crate::http::actor::ActorContext,
    Path(workspace_slug): Path<String>,
    Json(body): Json<domain::CreateProjectRequest>,
) -> Result<Created<domain::Project>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    validate_slug(&body.slug, &request_id)?;

    let id = Uuid::new_v4();
    let project = repo::insert_project(
        &state.pool,
        &id.to_string(),
        &workspace.id,
        &body.slug,
        &body.name,
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if is_unique_violation(db_err.as_ref()) => {
            ApiError::validation_error(
                &request_id.0,
                format!(
                    "project slug '{}' already exists in workspace '{}'",
                    body.slug, workspace_slug
                ),
            )
        }
        other => {
            tracing::error!(error = %other, "create_project db error");
            ApiError::internal(&request_id.0, "failed to create project")
        }
    })?;

    emit_audit(AuditEvent {
        request_id: request_id.0.clone(),
        actor,
        action: "project.created".to_string(),
        resource_kind: "project".to_string(),
        resource_id: project.id.clone(),
        payload: None,
    });

    Ok(Created(ApiResponse {
        data: project,
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    }))
}

async fn get_project(
    State(state): State<AppState>,
    request_id: RequestId,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<domain::Project>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;

    let project = repo::get_project_by_slug(&state.pool, &workspace.id, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "get_project db error");
            ApiError::internal(&request_id.0, "failed to fetch project")
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                &request_id.0,
                format!("project '{project_slug}' not found in workspace '{workspace_slug}'"),
            )
        })?;

    Ok(ApiResponse {
        data: project,
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve workspace by slug or return 404.
async fn resolve_workspace(
    state: &AppState,
    request_id: &RequestId,
    slug: &str,
) -> Result<domain::Workspace, ApiError> {
    repo::get_workspace_by_slug(&state.pool, slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "resolve_workspace db error");
            ApiError::internal(&request_id.0, "failed to resolve workspace")
        })?
        .ok_or_else(|| {
            ApiError::not_found(&request_id.0, format!("workspace '{slug}' not found"))
        })
}

/// Minimal slug validation: non-empty, lowercase ASCII + digits + hyphens.
fn validate_slug(slug: &str, request_id: &RequestId) -> Result<(), ApiError> {
    if slug.is_empty() {
        return Err(ApiError::validation_error(
            &request_id.0,
            "slug must not be empty",
        ));
    }
    let valid = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid || slug.starts_with('-') || slug.ends_with('-') {
        return Err(ApiError::validation_error(
            &request_id.0,
            "slug must be lowercase kebab-case (a-z, 0-9, hyphens; no leading/trailing hyphen)",
        ));
    }
    Ok(())
}

fn is_unique_violation(err: &dyn sqlx::error::DatabaseError) -> bool {
    // SQLite reports UNIQUE constraint violations with code "2067".
    // Postgres reports them with code "23505".
    err.code().map_or(false, |c| c == "2067" || c == "23505")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::sqlite_test_pool;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    const RID: &str = "x-request-id";
    const TEST_RID: &str = "test-request-id";

    async fn test_router() -> Router {
        let state = AppState { pool: sqlite_test_pool().await };
        routes(state)
    }

    // ---- Workspace endpoints ----

    #[tokio::test]
    async fn list_workspaces_returns_empty_initially() {
        let router = test_router().await;
        let resp = router
            .oneshot(
                Request::get("/workspaces")
                    .header(RID, TEST_RID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = parse_body(resp).await;
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_and_get_workspace() {
        let state = AppState { pool: sqlite_test_pool().await };
        let router = routes(state);

        // Create
        let create_resp = router
            .clone()
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"slug":"my-ws","name":"My Workspace"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let created: serde_json::Value = parse_body(create_resp).await;
        assert_eq!(created["data"]["slug"], "my-ws");
        assert_eq!(created["data"]["name"], "My Workspace");

        // Get by slug
        let get_resp = router
            .clone()
            .oneshot(
                Request::get("/workspaces/my-ws")
                    .header(RID, TEST_RID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let fetched: serde_json::Value = parse_body(get_resp).await;
        assert_eq!(fetched["data"]["slug"], "my-ws");
    }

    #[tokio::test]
    async fn get_nonexistent_workspace_returns_404() {
        let router = test_router().await;
        let resp = router
            .oneshot(
                Request::get("/workspaces/no-such")
                    .header(RID, TEST_RID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn duplicate_workspace_slug_returns_422() {
        let state = AppState { pool: sqlite_test_pool().await };
        let router = routes(state);
        let body = r#"{"slug":"dup","name":"First"}"#;

        let r1 = router
            .clone()
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::CREATED);

        let r2 = router
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn invalid_slug_returns_422() {
        let router = test_router().await;
        let resp = router
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"slug":"Bad Slug!","name":"X"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ---- Project endpoints ----

    #[tokio::test]
    async fn create_and_get_project() {
        let state = AppState { pool: sqlite_test_pool().await };
        let router = routes(state.clone());

        // Seed workspace via repo
        repo::insert_workspace(&state.pool, &Uuid::new_v4().to_string(), "ws", "WS")
            .await
            .unwrap();

        // Create project
        let resp = router
            .clone()
            .oneshot(
                Request::post("/workspaces/ws/projects")
                    .header(RID, TEST_RID)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"slug":"proj-a","name":"Project A"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let created: serde_json::Value = parse_body(resp).await;
        assert_eq!(created["data"]["slug"], "proj-a");
        assert_eq!(created["data"]["status"], "active");

        // Get project
        let get_resp = router
            .oneshot(
                Request::get("/workspaces/ws/projects/proj-a")
                    .header(RID, TEST_RID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_projects_on_nonexistent_workspace_returns_404() {
        let router = test_router().await;
        let resp = router
            .oneshot(
                Request::get("/workspaces/ghost/projects")
                    .header(RID, TEST_RID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_nonexistent_project_returns_404() {
        let state = AppState { pool: sqlite_test_pool().await };
        let router = routes(state.clone());
        repo::insert_workspace(&state.pool, &Uuid::new_v4().to_string(), "ws2", "WS2")
            .await
            .unwrap();

        let resp = router
            .oneshot(
                Request::get("/workspaces/ws2/projects/nope")
                    .header(RID, TEST_RID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ---- Helper ----

    async fn parse_body(resp: axum::http::Response<Body>) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}
