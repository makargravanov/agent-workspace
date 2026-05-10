use crate::http::{
    access::{require_human_workspace_role, WorkspaceRole},
    actor::ActorContext,
    audit::{emit_audit, AuditEvent},
    error::ApiError,
    request_id::RequestId,
    response::{ApiResponse, ResponseMeta},
};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    routing::patch,
    Json, Router,
};

use super::workspace_core::{domain, repo};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspace_slug}", patch(update_workspace))
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}",
            patch(update_project),
        )
}

async fn update_workspace(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
    Json(body): Json<domain::UpdateWorkspaceRequest>,
) -> Result<ApiResponse<domain::Workspace>, ApiError> {
    let workspace = repo::get_workspace_by_slug(&state.pool, &workspace_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, "update_workspace resolve failed");
            ApiError::internal(&request_id, "failed to resolve workspace")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, format!("workspace '{workspace_slug}' not found")))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Owner,
        &request_id,
    )
    .await?;

    if let Some(ref slug) = body.slug {
        validate_slug(slug, &request_id)?;
    }

    let updated = repo::update_workspace(
        &state.pool,
        state.db_backend,
        &workspace.id,
        body.slug.as_deref(),
        body.name.as_deref(),
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if is_unique_violation(db_err.as_ref()) => {
            ApiError::validation_error(
                &request_id,
                format!("workspace slug '{}' is already taken", body.slug.as_deref().unwrap_or(&workspace.slug)),
            )
        }
        other => {
            tracing::error!(error = %other, workspace_id = %workspace.id, "update_workspace db error");
            ApiError::internal(&request_id, "failed to update workspace")
        }
    })?;

    emit_audit(AuditEvent {
        request_id: request_id.clone(),
        actor,
        action: "workspace.updated".to_string(),
        resource_kind: "workspace".to_string(),
        resource_id: updated.id.clone(),
        payload: None,
    });

    Ok(ApiResponse {
        data: updated,
        meta: ResponseMeta {
            request_id: request_id,
            audit_event_id: None,
        },
    })
}

async fn update_project(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    Json(body): Json<domain::UpdateProjectRequest>,
) -> Result<ApiResponse<domain::Project>, ApiError> {
    let workspace = repo::get_workspace_by_slug(&state.pool, &workspace_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, "update_project resolve workspace failed");
            ApiError::internal(&request_id, "failed to resolve workspace")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, format!("workspace '{workspace_slug}' not found")))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace.id,
        WorkspaceRole::Owner,
        &request_id,
    )
    .await?;

    let project = repo::get_project_by_slug(&state.pool, &workspace.id, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project = %project_slug, "update_project resolve project failed");
            ApiError::internal(&request_id, "failed to resolve project")
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                &request_id,
                format!("project '{project_slug}' not found in workspace '{workspace_slug}'"),
            )
        })?;

    if let Some(ref slug) = body.slug {
        validate_slug(slug, &request_id)?;
    }

    if let Some(ref status) = body.status {
        if !["active", "on_hold", "archived"].contains(&status.as_str()) {
            return Err(ApiError::validation_error(
                &request_id,
                "status must be one of: active, on_hold, archived",
            ));
        }
    }

    let updated = repo::update_project(
        &state.pool,
        state.db_backend,
        &project.id,
        body.slug.as_deref(),
        body.name.as_deref(),
        body.status.as_deref(),
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if is_unique_violation(db_err.as_ref()) => {
            ApiError::validation_error(
                &request_id,
                format!(
                    "project slug '{}' already exists in workspace '{}'",
                    body.slug.as_deref().unwrap_or(&project.slug),
                    workspace_slug
                ),
            )
        }
        other => {
            tracing::error!(error = %other, project_id = %project.id, "update_project db error");
            ApiError::internal(&request_id, "failed to update project")
        }
    })?;

    emit_audit(AuditEvent {
        request_id: request_id.clone(),
        actor,
        action: "project.updated".to_string(),
        resource_kind: "project".to_string(),
        resource_id: updated.id.clone(),
        payload: None,
    });

    Ok(ApiResponse {
        data: updated,
        meta: ResponseMeta {
            request_id: request_id.clone(),
            audit_event_id: None,
        },
    })
}

fn validate_slug(slug: &str, request_id: &str) -> Result<(), ApiError> {
    if slug.is_empty() {
        return Err(ApiError::validation_error(
            request_id,
            "slug must not be empty",
        ));
    }

    let valid = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid || slug.starts_with('-') || slug.ends_with('-') {
        return Err(ApiError::validation_error(
            request_id,
            "slug must be lowercase kebab-case (a-z, 0-9, hyphens; no leading/trailing hyphen)",
        ));
    }

    Ok(())
}

fn is_unique_violation(err: &dyn sqlx::error::DatabaseError) -> bool {
    err.code().map_or(false, |c| c == "2067" || c == "23505")
}

#[cfg(test)]
mod tests {
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
    use uuid::Uuid;

    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";

    async fn setup() -> (axum::Router, String, String, String) {
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
            workspace_id,
            project_id,
        )
    }

    #[tokio::test]
    async fn update_workspace_changes_slug_and_name() {
        let (router, member_id, _workspace_id, _project_id) = setup().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/workspaces/dev-workspace")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({ "slug": "renamed-workspace", "name": "Renamed" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_project_changes_status() {
        let (router, member_id, _workspace_id, _project_id) = setup().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(json!({ "status": "on_hold" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
