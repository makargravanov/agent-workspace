use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::http::{
    access::{require_authenticated_human, require_human_workspace_role, WorkspaceRole},
    actor::ActorContext,
    audit::{record_audit, AuditEvent},
    error::ApiError,
    request_id::RequestId,
    response::{ApiResponse, Created, ListData, ResponseMeta},
};
use crate::state::AppState;

use super::{domain, repo};

#[derive(sqlx::FromRow)]
struct CurrentMemberIdentityRow {
    external_subject: String,
    display_name: String,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/workspaces/{workspace_slug}",
            get(get_workspace).delete(delete_workspace),
        )
        .route(
            "/workspaces/{workspace_slug}/projects",
            get(list_projects).post(create_project),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}",
            get(get_project).delete(delete_project),
        )
}

// ---------------------------------------------------------------------------
// Handlers — Workspaces
// ---------------------------------------------------------------------------

async fn list_workspaces(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: ActorContext,
) -> Result<ApiResponse<ListData<domain::Workspace>>, ApiError> {
    require_authenticated_human(&actor, &request_id.0)?;

    let query = if state.db_backend == crate::db::DatabaseBackend::Postgres {
        "SELECT DISTINCT
             CAST(w.id AS TEXT) AS id,
             w.slug,
             w.name,
             CAST(w.created_at AS TEXT) AS created_at,
             CAST(w.updated_at AS TEXT) AS updated_at
         FROM workspace_members current
         JOIN workspace_members target
           ON target.external_subject = current.external_subject
         JOIN workspaces w ON w.id = target.workspace_id
         WHERE current.id = CAST($1 AS UUID)
           AND current.status = 'active'
           AND target.status = 'active'
         ORDER BY w.name, w.slug"
    } else {
        "SELECT DISTINCT
             CAST(w.id AS TEXT) AS id,
             w.slug,
             w.name,
             CAST(w.created_at AS TEXT) AS created_at,
             CAST(w.updated_at AS TEXT) AS updated_at
         FROM workspace_members current
         JOIN workspace_members target
           ON target.external_subject = current.external_subject
         JOIN workspaces w ON w.id = target.workspace_id
         WHERE current.id = $1
           AND current.status = 'active'
           AND target.status = 'active'
         ORDER BY w.name, w.slug"
    };

    let items = sqlx::query_as::<_, domain::Workspace>(query)
        .bind(&actor.actor_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, actor_id = %actor.actor_id, "list_workspaces db error");
            ApiError::internal(&request_id.0, "failed to list workspaces")
        })?;

    Ok(ApiResponse {
        data: ListData {
            items,
            next_cursor: None,
        },
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

async fn create_workspace(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: ActorContext,
    Json(body): Json<domain::CreateWorkspaceRequest>,
) -> Result<Created<domain::Workspace>, ApiError> {
    require_authenticated_human(&actor, &request_id.0)?;
    validate_slug(&body.slug, &request_id)?;

    let id = Uuid::new_v4();
    let workspace = repo::insert_workspace(
        &state.pool,
        state.db_backend,
        &id.to_string(),
        &body.slug,
        &body.name,
    )
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

    let current_member_lookup_sql = match state.db_backend {
        crate::db::DatabaseBackend::Postgres => {
            "SELECT external_subject, display_name
             FROM workspace_members
             WHERE id = CAST($1 AS UUID) AND status = 'active'"
        }
        crate::db::DatabaseBackend::Sqlite => {
            "SELECT external_subject, display_name
             FROM workspace_members
             WHERE id = $1 AND status = 'active'"
        }
    };

    let current_member = sqlx::query_as::<_, CurrentMemberIdentityRow>(current_member_lookup_sql)
        .bind(&actor.actor_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, actor_id = %actor.actor_id, "current member lookup failed");
            ApiError::internal(&request_id.0, "failed to resolve current workspace member")
        })?
        .ok_or_else(|| {
            ApiError::forbidden(
                &request_id.0,
                "human session is not linked to an active workspace member",
            )
        })?;

    let membership_insert_sql = match state.db_backend {
        crate::db::DatabaseBackend::Postgres => {
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, 'owner', 'active')"
        }
        crate::db::DatabaseBackend::Sqlite => {
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES ($1, $2, $3, $4, 'owner', 'active')"
        }
    };

    sqlx::query(membership_insert_sql)
        .bind(Uuid::new_v4().to_string())
        .bind(&workspace.id)
        .bind(current_member.external_subject)
        .bind(current_member.display_name)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace_id = %workspace.id, "creator membership insert failed");
            ApiError::internal(&request_id.0, "failed to create creator workspace membership")
        })?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.0.clone(),
            actor,
            action: "workspace.created".to_string(),
            resource_kind: "workspace".to_string(),
            resource_id: workspace.id.clone(),
            payload: None,
        },
    )
    .await;

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
    actor: ActorContext,
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

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Viewer,
        &request_id.0,
    )
    .await?;

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
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<ListData<domain::Project>>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Viewer,
        &request_id.0,
    )
    .await?;

    let items = repo::list_projects(&state.pool, &workspace.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "list_projects db error");
            ApiError::internal(&request_id.0, "failed to list projects")
        })?;

    Ok(ApiResponse {
        data: ListData {
            items,
            next_cursor: None,
        },
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

async fn create_project(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
    Json(body): Json<domain::CreateProjectRequest>,
) -> Result<Created<domain::Project>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Owner,
        &request_id.0,
    )
    .await?;
    validate_slug(&body.slug, &request_id)?;

    let id = Uuid::new_v4();
    let project = repo::insert_project(
        &state.pool,
        state.db_backend,
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

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.0.clone(),
            actor,
            action: "project.created".to_string(),
            resource_kind: "project".to_string(),
            resource_id: project.id.clone(),
            payload: None,
        },
    )
    .await;

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
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<domain::Project>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Viewer,
        &request_id.0,
    )
    .await?;

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

async fn delete_project(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Owner,
        &request_id.0,
    )
    .await?;

    let project = repo::get_project_by_slug(&state.pool, &workspace.id, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "delete_project db error");
            ApiError::internal(&request_id.0, "failed to resolve project")
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                &request_id.0,
                format!("project '{project_slug}' not found in workspace '{workspace_slug}'"),
            )
        })?;

    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!(error = %e, project_id = %project.id, "delete_project tx begin failed");
        ApiError::internal(&request_id.0, "failed to delete project")
    })?;

    let project_id = project.id.clone();

    sqlx::query(
        "UPDATE tasks SET parent_task_id = NULL
         WHERE CAST(project_id AS TEXT) = $1",
    )
    .bind(&project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, project_id = %project_id, "delete_project task parent cleanup failed");
        ApiError::internal(&request_id.0, "failed to delete project")
    })?;

    sqlx::query(
        "UPDATE documents SET parent_document_id = NULL
         WHERE CAST(project_id AS TEXT) = $1",
    )
    .bind(&project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, project_id = %project_id, "delete_project document parent cleanup failed");
        ApiError::internal(&request_id.0, "failed to delete project")
    })?;

    for sql in [
        "DELETE FROM agent_session_tasks WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM task_dependencies WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM notes WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM assets WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM documents WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM tasks WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM task_groups WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM agent_credentials WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM agent_sessions WHERE CAST(project_id AS TEXT) = $1",
        "DELETE FROM projects WHERE CAST(id AS TEXT) = $1",
    ] {
        sqlx::query(sql)
            .bind(&project_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, project_id = %project_id, "delete_project cascade failed");
                ApiError::internal(&request_id.0, "failed to delete project")
            })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, project_id = %project_id, "delete_project commit failed");
        ApiError::internal(&request_id.0, "failed to delete project")
    })?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.0.clone(),
            actor,
            action: "project.deleted".to_string(),
            resource_kind: "project".to_string(),
            resource_id: project_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_workspace(
    State(state): State<AppState>,
    request_id: RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
) -> Result<StatusCode, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Owner,
        &request_id.0,
    )
    .await?;

    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace.id, "delete_workspace tx begin failed");
        ApiError::internal(&request_id.0, "failed to delete workspace")
    })?;

    let workspace_id = workspace.id.clone();

    sqlx::query(
        "UPDATE tasks SET parent_task_id = NULL
         WHERE CAST(workspace_id AS TEXT) = $1",
    )
    .bind(&workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, "delete_workspace task parent cleanup failed");
        ApiError::internal(&request_id.0, "failed to delete workspace")
    })?;

    sqlx::query(
        "UPDATE documents SET parent_document_id = NULL
         WHERE CAST(workspace_id AS TEXT) = $1",
    )
    .bind(&workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, "delete_workspace document parent cleanup failed");
        ApiError::internal(&request_id.0, "failed to delete workspace")
    })?;

    for sql in [
        "DELETE FROM agent_session_tasks WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM task_dependencies WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM notes WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM assets WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM documents WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM tasks WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM task_groups WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM agent_credentials WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM agent_sessions WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM agents WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM projects WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM workspace_members WHERE CAST(workspace_id AS TEXT) = $1",
        "DELETE FROM workspaces WHERE CAST(id AS TEXT) = $1",
    ] {
        sqlx::query(sql)
            .bind(&workspace_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, workspace_id = %workspace_id, "delete_workspace cascade failed");
                ApiError::internal(&request_id.0, "failed to delete workspace")
            })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, "delete_workspace commit failed");
        ApiError::internal(&request_id.0, "failed to delete workspace")
    })?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.0.clone(),
            actor,
            action: "workspace.deleted".to_string(),
            resource_kind: "workspace".to_string(),
            resource_id: workspace_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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
        .ok_or_else(|| ApiError::not_found(&request_id.0, format!("workspace '{slug}' not found")))
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
    use crate::testing::any_test_pool;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    const RID: &str = "x-request-id";
    const TEST_RID: &str = "test-request-id";
    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";

    async fn test_router() -> Router {
        routes().with_state(AppState::new(
            any_test_pool().await,
            crate::db::DatabaseBackend::Sqlite,
        ))
    }

    async fn seed_member(
        state: &AppState,
        workspace_slug: &str,
        workspace_name: &str,
        role: &str,
    ) -> String {
        let workspace_id = Uuid::new_v4().to_string();
        let member_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)")
            .bind(&workspace_id)
            .bind(workspace_slug)
            .bind(workspace_name)
            .execute(&state.pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES ($1, $2, $3, $4, $5, 'active')",
        )
        .bind(&member_id)
        .bind(&workspace_id)
        .bind("test:owner-1")
        .bind("Test Owner")
        .bind(role)
        .execute(&state.pool)
        .await
        .unwrap();

        member_id
    }

    // ---- Workspace endpoints ----

    #[tokio::test]
    async fn list_workspaces_returns_empty_initially() {
        let router = test_router().await;
        let resp = router
            .oneshot(
                Request::get("/workspaces")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, "ghost-member")
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
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let member_id = seed_member(&state, "seed-ws", "Seed WS", "owner").await;
        let router = routes().with_state(state);

        // Create
        let create_resp = router
            .clone()
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
    async fn delete_workspace_removes_workspace_and_children() {
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let member_id = seed_member(&state, "seed-delete-ws", "Seed Delete WS", "owner").await;
        let (workspace_id,): (String,) = sqlx::query_as("SELECT id FROM workspaces WHERE slug = ?")
            .bind("seed-delete-ws")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let project_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();
        let note_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&project_id)
        .bind(&workspace_id)
        .bind("delete-project")
        .bind("Delete Project")
        .bind("active")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO tasks
             (id, workspace_id, project_id, rank_key, title, description_md, status, priority)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&task_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind("rank-a")
        .bind("Task to delete")
        .bind("body")
        .bind("todo")
        .bind("normal")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO notes
             (id, workspace_id, project_id, kind, author_type, author_id, title, body_md)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&note_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind("context")
        .bind("workspace_member")
        .bind(&member_id)
        .bind(Some("Note to delete"))
        .bind("body")
        .execute(&state.pool)
        .await
        .unwrap();

        let router = routes().with_state(state.clone());
        let resp = router
            .oneshot(
                Request::delete("/workspaces/seed-delete-ws")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let (workspace_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let (project_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let (task_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let (note_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notes")
            .fetch_one(&state.pool)
            .await
            .unwrap();

        assert_eq!(workspace_count, 0);
        assert_eq!(project_count, 0);
        assert_eq!(task_count, 0);
        assert_eq!(note_count, 0);
    }

    #[tokio::test]
    async fn delete_project_removes_project_children_but_keeps_workspace() {
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let member_id = seed_member(
            &state,
            "seed-delete-proj-ws",
            "Seed Delete Project WS",
            "owner",
        )
        .await;
        let (workspace_id,): (String,) = sqlx::query_as("SELECT id FROM workspaces WHERE slug = ?")
            .bind("seed-delete-proj-ws")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let project_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();
        let note_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&project_id)
        .bind(&workspace_id)
        .bind("delete-project")
        .bind("Delete Project")
        .bind("active")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO tasks
             (id, workspace_id, project_id, rank_key, title, description_md, status, priority)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&task_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind("rank-a")
        .bind("Task to delete")
        .bind("body")
        .bind("todo")
        .bind("normal")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO notes
             (id, workspace_id, project_id, kind, author_type, author_id, title, body_md)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&note_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind("context")
        .bind("workspace_member")
        .bind(&member_id)
        .bind(Some("Note to delete"))
        .bind("body")
        .execute(&state.pool)
        .await
        .unwrap();

        let router = routes().with_state(state.clone());
        let resp = router
            .oneshot(
                Request::delete("/workspaces/seed-delete-proj-ws/projects/delete-project")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let (workspace_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let (project_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let (task_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        let (note_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notes")
            .fetch_one(&state.pool)
            .await
            .unwrap();

        assert_eq!(workspace_count, 1);
        assert_eq!(project_count, 0);
        assert_eq!(task_count, 0);
        assert_eq!(note_count, 0);
    }

    #[tokio::test]
    async fn duplicate_workspace_slug_returns_422() {
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let member_id = seed_member(&state, "seed-ws", "Seed WS", "owner").await;
        let router = routes().with_state(state);
        let body = r#"{"slug":"dup","name":"First"}"#;

        let r1 = router
            .clone()
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let member_id = seed_member(&state, "seed-ws", "Seed WS", "owner").await;
        let router = routes().with_state(state);
        let resp = router
            .oneshot(
                Request::post("/workspaces")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let router = routes().with_state(state.clone());

        // Seed workspace via repo
        let member_id = seed_member(&state, "ws", "WS", "owner").await;

        // Create project
        let resp = router
            .clone()
            .oneshot(
                Request::post("/workspaces/ws/projects")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
        let state = AppState::new(any_test_pool().await, crate::db::DatabaseBackend::Sqlite);
        let router = routes().with_state(state.clone());
        let member_id = seed_member(&state, "ws2", "WS2", "owner").await;

        let resp = router
            .oneshot(
                Request::get("/workspaces/ws2/projects/nope")
                    .header(RID, TEST_RID)
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
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
