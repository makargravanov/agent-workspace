use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::DatabaseBackend;
use crate::http::{
    access::{require_project_access, WorkspaceRole},
    actor::ActorContext,
    audit::{record_audit, AuditEvent},
    error::ApiError,
    request_id::RequestId,
    response::{ApiResponse, Created, ListData, ResponseMeta},
};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskGroupResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub kind: String,
    pub title: String,
    pub description_md: Option<String>,
    pub status: String,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskGroupRequest {
    pub kind: String,
    pub title: String,
    pub description_md: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskGroupRequest {
    pub kind: Option<String>,
    pub title: Option<String>,
    pub description_md: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
}

fn default_status() -> String {
    "active".to_string()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/task-groups",
            get(list_task_groups).post(create_task_group),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/task-groups/{group_id}",
            get(get_task_group)
                .patch(update_task_group)
                .delete(delete_task_group),
        )
}

async fn resolve_project(
    pool: &sqlx::AnyPool,
    workspace_slug: &str,
    project_slug: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT CAST(w.id AS TEXT) AS workspace_id, CAST(p.id AS TEXT) AS project_id
         FROM workspaces w
         JOIN projects p ON p.workspace_id = w.id
         WHERE w.slug = $1 AND p.slug = $2 AND p.status != 'archived'",
    )
    .bind(workspace_slug)
    .bind(project_slug)
    .fetch_optional(pool)
    .await
}

fn validate_kind(kind: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(kind, "initiative" | "epic") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "kind must be one of: initiative, epic",
        ))
    }
}

fn validate_status(status: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(status, "draft" | "active" | "done" | "archived") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "status must be one of: draft, active, done, archived",
        ))
    }
}

async fn fetch_task_group(
    pool: &sqlx::AnyPool,
    project_id: &str,
    group_id: &str,
) -> Result<Option<TaskGroupResponse>, sqlx::Error> {
    sqlx::query_as::<_, TaskGroupResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                kind,
                title,
                description_md,
                status,
                priority,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM task_groups
         WHERE CAST(project_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(project_id)
    .bind(group_id)
    .fetch_optional(pool)
    .await
}

async fn list_task_groups(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<ListData<TaskGroupResponse>>, ApiError> {
    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let (workspace_id, project_id) =
        ids.ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Viewer,
        Some("task_groups:read"),
        &request_id,
    )
    .await?;

    let items = sqlx::query_as::<_, TaskGroupResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                kind,
                title,
                description_md,
                status,
                priority,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM task_groups
         WHERE CAST(project_id AS TEXT) = $1
         ORDER BY priority DESC, created_at DESC",
    )
    .bind(&project_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    Ok(ApiResponse {
        data: ListData {
            items,
            next_cursor: None,
        },
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn create_task_group(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    Json(body): Json<CreateTaskGroupRequest>,
) -> Result<Created<TaskGroupResponse>, ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::validation_error(
            &request_id,
            "title must not be empty",
        ));
    }
    validate_kind(&body.kind, &request_id)?;
    validate_status(&body.status, &request_id)?;

    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let (workspace_id, project_id) =
        ids.ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Editor,
        None,
        &request_id,
    )
    .await?;

    let group_id = Uuid::new_v4().to_string();
    let insert_sql = if state.db_backend == DatabaseBackend::Postgres {
        "INSERT INTO task_groups
         (id, workspace_id, project_id, kind, title, description_md, status, priority)
         VALUES (CAST($1 AS UUID), CAST($2 AS UUID), CAST($3 AS UUID), $4, $5, $6, $7, $8)"
    } else {
        "INSERT INTO task_groups
         (id, workspace_id, project_id, kind, title, description_md, status, priority)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    };

    sqlx::query(insert_sql)
        .bind(&group_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind(&body.kind)
        .bind(body.title.trim())
        .bind(&body.description_md)
        .bind(&body.status)
        .bind(body.priority)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let group = fetch_task_group(&state.pool, &project_id, &group_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "task group not found after insert"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task_group.created".to_string(),
            resource_kind: "task_group".to_string(),
            resource_id: group_id,
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: group,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn get_task_group(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, group_id)): Path<(String, String, String)>,
) -> Result<ApiResponse<TaskGroupResponse>, ApiError> {
    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let (workspace_id, project_id) =
        ids.ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Viewer,
        Some("task_groups:read"),
        &request_id,
    )
    .await?;

    let group = fetch_task_group(&state.pool, &project_id, &group_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "task group not found"))?;

    Ok(ApiResponse {
        data: group,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn update_task_group(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, group_id)): Path<(String, String, String)>,
    Json(body): Json<UpdateTaskGroupRequest>,
) -> Result<ApiResponse<TaskGroupResponse>, ApiError> {
    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let (workspace_id, project_id) =
        ids.ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Editor,
        None,
        &request_id,
    )
    .await?;

    if let Some(ref kind) = body.kind {
        validate_kind(kind, &request_id)?;
    }
    if let Some(ref status) = body.status {
        validate_status(status, &request_id)?;
    }
    if let Some(ref title) = body.title {
        if title.trim().is_empty() {
            return Err(ApiError::validation_error(
                &request_id,
                "title must not be empty",
            ));
        }
    }

    let task_group = fetch_task_group(&state.pool, &project_id, &group_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "task group not found"))?;

    sqlx::query(
        "UPDATE task_groups
         SET kind = COALESCE($1, kind),
             title = COALESCE($2, title),
             description_md = COALESCE($3, description_md),
             status = COALESCE($4, status),
             priority = COALESCE($5, priority),
             updated_at = CURRENT_TIMESTAMP
         WHERE CAST(id AS TEXT) = $6 AND CAST(project_id AS TEXT) = $7",
    )
    .bind(body.kind.as_deref())
    .bind(body.title.as_deref().map(str::trim))
    .bind(body.description_md.as_deref())
    .bind(body.status.as_deref())
    .bind(body.priority)
    .bind(&group_id)
    .bind(&project_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let updated = fetch_task_group(&state.pool, &project_id, &group_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "task group not found after update"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task_group.updated".to_string(),
            resource_kind: "task_group".to_string(),
            resource_id: group_id,
            payload: Some(serde_json::json!({ "previous_title": task_group.title })),
        },
    )
    .await;

    Ok(ApiResponse {
        data: updated,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn delete_task_group(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, group_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let (workspace_id, project_id) =
        ids.ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Editor,
        None,
        &request_id,
    )
    .await?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    sqlx::query(
        "UPDATE tasks SET group_id = NULL
         WHERE CAST(group_id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&group_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let affected = sqlx::query(
        "DELETE FROM task_groups
         WHERE CAST(id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&group_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "task group not found"));
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task_group.deleted".to_string(),
            resource_kind: "task_group".to_string(),
            resource_id: group_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    use crate::{
        app::build_router,
        db::DatabaseBackend,
        state::AppState,
        testing::{any_test_pool, fixtures},
    };

    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";

    async fn setup() -> (axum::Router, String) {
        let pool = any_test_pool().await;
        let workspace_id = Uuid::new_v4().to_string();
        let member_id = Uuid::new_v4().to_string();
        let project_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)")
            .bind(&workspace_id)
            .bind(fixtures::WORKSPACE_SLUG)
            .bind(fixtures::WORKSPACE_NAME)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO workspace_members (id, workspace_id, external_subject, display_name, role, status)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&member_id)
        .bind(&workspace_id)
        .bind("test:member-1")
        .bind("Test Member")
        .bind("owner")
        .bind("active")
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, $5)")
            .bind(&project_id)
            .bind(&workspace_id)
            .bind(fixtures::PROJECT_SLUG)
            .bind(fixtures::PROJECT_NAME)
            .bind("active")
            .execute(&pool)
            .await
            .unwrap();

        (
            build_router(AppState::new(pool, DatabaseBackend::Sqlite)),
            member_id,
        )
    }

    #[tokio::test]
    async fn create_task_group_returns_201() {
        let (router, member_id) = setup().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/task-groups")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "kind": "epic",
                            "title": "Foundation",
                            "description_md": "Group work",
                            "priority": 5
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn list_get_update_and_delete_task_group() {
        let (router, member_id) = setup().await;
        let created = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/task-groups")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "kind": "epic",
                            "title": "Foundation",
                            "description_md": "Group work",
                            "priority": 5
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(created.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let group_id = created_body["data"]["id"].as_str().unwrap().to_string();

        let listed = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/task-groups")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);

        let fetched = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/task-groups/{group_id}"
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(fetched.status(), StatusCode::OK);

        let updated = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/task-groups/{group_id}"
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "title": "Foundation v2",
                            "status": "done",
                            "priority": 8
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let updated_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(updated.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(updated_body["data"]["title"], "Foundation v2");
        assert_eq!(updated_body["data"]["status"], "done");

        let deleted = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/task-groups/{group_id}"
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn create_task_group_rejects_invalid_kind() {
        let (router, member_id) = setup().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/task-groups")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "kind": "story",
                            "title": "Foundation"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_task_group_rejects_empty_title() {
        let (router, member_id) = setup().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/task-groups")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "kind": "epic",
                            "title": "   "
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
