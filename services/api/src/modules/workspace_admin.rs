use crate::db::DatabaseBackend;
use crate::http::{
    access::{require_human_workspace_role, WorkspaceRole},
    actor::{hash_secret, ActorContext},
    audit::{record_audit, AuditEvent},
    error::ApiError,
    request_id::RequestId,
    response::{ApiResponse, Created, ListData, ResponseMeta},
};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::workspace_core::{domain, repo};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspace_slug}", patch(update_workspace))
        .route(
            "/workspaces/{workspace_slug}/members",
            get(list_workspace_members),
        )
        .route(
            "/workspaces/{workspace_slug}/members/invites",
            get(list_workspace_invites).post(create_workspace_invite),
        )
        .route(
            "/workspaces/{workspace_slug}/members/{member_id}",
            patch(update_workspace_member),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}",
            patch(update_project),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/members",
            get(list_project_members),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/members/{member_id}",
            put(upsert_project_member).delete(delete_project_member),
        )
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WorkspaceMember {
    pub id: String,
    pub workspace_id: String,
    pub external_subject: String,
    pub display_name: String,
    pub github_login: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WorkspaceInvite {
    pub id: String,
    pub workspace_id: String,
    pub github_login: Option<String>,
    pub role: String,
    pub project_access_json: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub created_by_member_id: String,
    pub accepted_by_member_id: Option<String>,
    pub accepted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_url: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProjectMember {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub workspace_member_id: String,
    pub external_subject: String,
    pub display_name: String,
    pub github_login: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectAccessRequest {
    pub project_id: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceInviteRequest {
    pub github_login: Option<String>,
    pub role: String,
    #[serde(default)]
    pub project_access: Vec<ProjectAccessRequest>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceMemberRequest {
    pub role: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertProjectMemberRequest {
    pub role: String,
}

#[derive(sqlx::FromRow)]
struct OwnerCountRow {
    count: i64,
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

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "workspace.updated".to_string(),
            resource_kind: "workspace".to_string(),
            resource_id: updated.id.clone(),
            payload: None,
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

async fn list_workspace_members(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<ListData<WorkspaceMember>>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;

    let items = sqlx::query_as::<_, WorkspaceMember>(
        "SELECT CAST(wm.id AS TEXT) AS id,
                CAST(wm.workspace_id AS TEXT) AS workspace_id,
                wm.external_subject,
                wm.display_name,
                hi.display_name AS github_login,
                wm.role,
                wm.status,
                CAST(wm.created_at AS TEXT) AS created_at,
                CAST(wm.updated_at AS TEXT) AS updated_at
         FROM workspace_members wm
         LEFT JOIN human_identities hi
           ON hi.workspace_member_id = wm.id AND hi.provider = 'github'
         WHERE CAST(wm.workspace_id AS TEXT) = $1
         ORDER BY wm.created_at",
    )
    .bind(&workspace.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace.id, "list workspace members failed");
        ApiError::internal(&request_id, "failed to list workspace members")
    })?;

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

async fn list_workspace_invites(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<ListData<WorkspaceInvite>>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;

    let items = sqlx::query_as::<_, WorkspaceInvite>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                github_login,
                role,
                CAST(project_access_json AS TEXT) AS project_access_json,
                status,
                CAST(expires_at AS TEXT) AS expires_at,
                CAST(created_by_member_id AS TEXT) AS created_by_member_id,
                CAST(accepted_by_member_id AS TEXT) AS accepted_by_member_id,
                CAST(accepted_at AS TEXT) AS accepted_at,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at,
                NULL AS invite_url
         FROM workspace_invites
         WHERE CAST(workspace_id AS TEXT) = $1
         ORDER BY created_at DESC",
    )
    .bind(&workspace.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace.id, "list workspace invites failed");
        ApiError::internal(&request_id, "failed to list workspace invites")
    })?;

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

async fn create_workspace_invite(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
    Json(body): Json<CreateWorkspaceInviteRequest>,
) -> Result<Created<WorkspaceInvite>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;
    validate_member_role(&body.role, false, &request_id)?;
    validate_project_access(&state, &workspace.id, &body.project_access, &request_id).await?;

    let invite_id = Uuid::new_v4().to_string();
    let token = format!("awinv_{}_{}", Uuid::new_v4(), Uuid::new_v4());
    let project_access_json = serde_json::to_string(&body.project_access).map_err(|e| {
        ApiError::internal(&request_id, format!("failed to serialize project access: {e}"))
    })?;
    let github_login = body.github_login.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_lowercase);

    let insert_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO workspace_invites
             (id, workspace_id, github_login, token_hash, role, project_access_json, status, expires_at, created_by_member_id)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, $5, CAST($6 AS JSONB), 'pending', CAST($7 AS TIMESTAMPTZ), CAST($8 AS UUID))"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO workspace_invites
             (id, workspace_id, github_login, token_hash, role, project_access_json, status, expires_at, created_by_member_id)
             VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8)"
        }
    };

    sqlx::query(insert_sql)
        .bind(&invite_id)
        .bind(&workspace.id)
        .bind(github_login.as_deref())
        .bind(hash_secret(&token))
        .bind(&body.role)
        .bind(&project_access_json)
        .bind(body.expires_at.as_deref())
        .bind(&actor.actor_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace_id = %workspace.id, "create workspace invite failed");
            ApiError::internal(&request_id, "failed to create workspace invite")
        })?;

    let mut invite = fetch_invite(&state, &invite_id, &request_id).await?;
    invite.invite_url = Some(format!("/api/v1/auth/github/start?invite={token}"));

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "workspace_member.invited".to_string(),
            resource_kind: "workspace_invite".to_string(),
            resource_id: invite.id.clone(),
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: invite,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn update_workspace_member(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, member_id)): Path<(String, String)>,
    Json(body): Json<UpdateWorkspaceMemberRequest>,
) -> Result<ApiResponse<WorkspaceMember>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;

    if let Some(role) = body.role.as_deref() {
        validate_member_role(role, true, &request_id)?;
    }
    if let Some(status) = body.status.as_deref() {
        if !["active", "disabled"].contains(&status) {
            return Err(ApiError::validation_error(
                &request_id,
                "status must be one of: active, disabled",
            ));
        }
    }

    let current = fetch_workspace_member(&state, &workspace.id, &member_id, &request_id).await?;
    let removing_owner = current.role == "owner"
        && (body.role.as_deref().is_some_and(|role| role != "owner")
            || body.status.as_deref() == Some("disabled"));
    if removing_owner {
        ensure_not_last_owner(&state, &workspace.id, &request_id).await?;
    }

    let update_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE workspace_members
             SET role = COALESCE($3, role),
                 status = COALESCE($4, status),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = CAST($1 AS UUID) AND workspace_id = CAST($2 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE workspace_members
             SET role = COALESCE($3, role),
                 status = COALESCE($4, status),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $1 AND workspace_id = $2"
        }
    };

    sqlx::query(update_sql)
        .bind(&member_id)
        .bind(&workspace.id)
        .bind(body.role.as_deref())
        .bind(body.status.as_deref())
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, member_id = %member_id, "update workspace member failed");
            ApiError::internal(&request_id, "failed to update workspace member")
        })?;

    let updated = fetch_workspace_member(&state, &workspace.id, &member_id, &request_id).await?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "workspace_member.updated".to_string(),
            resource_kind: "workspace_member".to_string(),
            resource_id: updated.id.clone(),
            payload: None,
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

    require_owner(&state, &actor, &workspace.id, &request_id).await?;

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

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "project.updated".to_string(),
            resource_kind: "project".to_string(),
            resource_id: updated.id.clone(),
            payload: None,
        },
    )
    .await;

    Ok(ApiResponse {
        data: updated,
        meta: ResponseMeta {
            request_id: request_id.clone(),
            audit_event_id: None,
        },
    })
}

async fn list_project_members(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<ListData<ProjectMember>>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;
    let project = resolve_project(&state, &request_id, &workspace, &project_slug).await?;

    let items = project_members_query()
        .bind(&project.id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project_id = %project.id, "list project members failed");
            ApiError::internal(&request_id, "failed to list project members")
        })?;

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

async fn upsert_project_member(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, member_id)): Path<(String, String, String)>,
    Json(body): Json<UpsertProjectMemberRequest>,
) -> Result<ApiResponse<ProjectMember>, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;
    let project = resolve_project(&state, &request_id, &workspace, &project_slug).await?;
    validate_member_role(&body.role, false, &request_id)?;
    let member = fetch_workspace_member(&state, &workspace.id, &member_id, &request_id).await?;
    if member.status != "active" {
        return Err(ApiError::validation_error(
            &request_id,
            "project access can only be granted to active workspace members",
        ));
    }
    if member.role == "owner" {
        return Err(ApiError::validation_error(
            &request_id,
            "workspace owners inherit all project access",
        ));
    }

    let access_id = Uuid::new_v4().to_string();
    let sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO project_members
             (id, workspace_id, project_id, workspace_member_id, role, status)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), CAST($3 AS UUID), CAST($4 AS UUID), $5, 'active')
             ON CONFLICT (project_id, workspace_member_id)
             DO UPDATE SET role = EXCLUDED.role, status = 'active', updated_at = CURRENT_TIMESTAMP"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO project_members
             (id, workspace_id, project_id, workspace_member_id, role, status)
             VALUES ($1, $2, $3, $4, $5, 'active')
             ON CONFLICT(project_id, workspace_member_id)
             DO UPDATE SET role = excluded.role, status = 'active', updated_at = CURRENT_TIMESTAMP"
        }
    };

    sqlx::query(sql)
        .bind(access_id)
        .bind(&workspace.id)
        .bind(&project.id)
        .bind(&member_id)
        .bind(&body.role)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, member_id = %member_id, project_id = %project.id, "upsert project member failed");
            ApiError::internal(&request_id, "failed to update project member")
        })?;

    let updated = fetch_project_member(&state, &project.id, &member_id, &request_id).await?;
    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "project_member.updated".to_string(),
            resource_kind: "project_member".to_string(),
            resource_id: updated.id.clone(),
            payload: None,
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

async fn delete_project_member(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, member_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    let workspace = resolve_workspace(&state, &request_id, &workspace_slug).await?;
    require_owner(&state, &actor, &workspace.id, &request_id).await?;
    let project = resolve_project(&state, &request_id, &workspace, &project_slug).await?;

    sqlx::query(
        "DELETE FROM project_members
         WHERE CAST(project_id AS TEXT) = $1
           AND CAST(workspace_member_id AS TEXT) = $2",
    )
    .bind(&project.id)
    .bind(&member_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, member_id = %member_id, project_id = %project.id, "delete project member failed");
        ApiError::internal(&request_id, "failed to delete project member")
    })?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id,
            actor,
            action: "project_member.deleted".to_string(),
            resource_kind: "project_member".to_string(),
            resource_id: member_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn resolve_workspace(
    state: &AppState,
    request_id: &str,
    slug: &str,
) -> Result<domain::Workspace, ApiError> {
    repo::get_workspace_by_slug(&state.pool, slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "resolve_workspace db error");
            ApiError::internal(request_id, "failed to resolve workspace")
        })?
        .ok_or_else(|| ApiError::not_found(request_id, format!("workspace '{slug}' not found")))
}

async fn resolve_project(
    state: &AppState,
    request_id: &str,
    workspace: &domain::Workspace,
    slug: &str,
) -> Result<domain::Project, ApiError> {
    repo::get_project_by_slug(&state.pool, &workspace.id, slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "resolve_project db error");
            ApiError::internal(request_id, "failed to resolve project")
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                request_id,
                format!("project '{slug}' not found in workspace '{}'", workspace.slug),
            )
        })
}

async fn require_owner(
    state: &AppState,
    actor: &ActorContext,
    workspace_id: &str,
    request_id: &str,
) -> Result<(), ApiError> {
    require_human_workspace_role(
        &state.pool,
        actor,
        workspace_id,
        WorkspaceRole::Owner,
        request_id,
    )
    .await
}

async fn fetch_invite(
    state: &AppState,
    invite_id: &str,
    request_id: &str,
) -> Result<WorkspaceInvite, ApiError> {
    sqlx::query_as::<_, WorkspaceInvite>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                github_login,
                role,
                CAST(project_access_json AS TEXT) AS project_access_json,
                status,
                CAST(expires_at AS TEXT) AS expires_at,
                CAST(created_by_member_id AS TEXT) AS created_by_member_id,
                CAST(accepted_by_member_id AS TEXT) AS accepted_by_member_id,
                CAST(accepted_at AS TEXT) AS accepted_at,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at,
                NULL AS invite_url
         FROM workspace_invites
         WHERE CAST(id AS TEXT) = $1",
    )
    .bind(invite_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, invite_id = %invite_id, "fetch invite failed");
        ApiError::internal(request_id, "failed to fetch workspace invite")
    })
}

async fn fetch_workspace_member(
    state: &AppState,
    workspace_id: &str,
    member_id: &str,
    request_id: &str,
) -> Result<WorkspaceMember, ApiError> {
    sqlx::query_as::<_, WorkspaceMember>(
        "SELECT CAST(wm.id AS TEXT) AS id,
                CAST(wm.workspace_id AS TEXT) AS workspace_id,
                wm.external_subject,
                wm.display_name,
                hi.display_name AS github_login,
                wm.role,
                wm.status,
                CAST(wm.created_at AS TEXT) AS created_at,
                CAST(wm.updated_at AS TEXT) AS updated_at
         FROM workspace_members wm
         LEFT JOIN human_identities hi
           ON hi.workspace_member_id = wm.id AND hi.provider = 'github'
         WHERE CAST(wm.workspace_id AS TEXT) = $1
           AND CAST(wm.id AS TEXT) = $2",
    )
    .bind(workspace_id)
    .bind(member_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, member_id = %member_id, "fetch workspace member failed");
        ApiError::internal(request_id, "failed to fetch workspace member")
    })?
    .ok_or_else(|| ApiError::not_found(request_id, "workspace member not found"))
}

fn project_members_query<'q>(
) -> sqlx::query::QueryAs<'q, sqlx::Any, ProjectMember, sqlx::any::AnyArguments<'q>> {
    sqlx::query_as::<_, ProjectMember>(
        "SELECT CAST(pm.id AS TEXT) AS id,
                CAST(pm.workspace_id AS TEXT) AS workspace_id,
                CAST(pm.project_id AS TEXT) AS project_id,
                CAST(pm.workspace_member_id AS TEXT) AS workspace_member_id,
                wm.external_subject,
                wm.display_name,
                hi.display_name AS github_login,
                pm.role,
                pm.status,
                CAST(pm.created_at AS TEXT) AS created_at,
                CAST(pm.updated_at AS TEXT) AS updated_at
         FROM project_members pm
         JOIN workspace_members wm ON wm.id = pm.workspace_member_id
         LEFT JOIN human_identities hi
           ON hi.workspace_member_id = wm.id AND hi.provider = 'github'
         WHERE CAST(pm.project_id AS TEXT) = $1
         ORDER BY wm.display_name",
    )
}

async fn fetch_project_member(
    state: &AppState,
    project_id: &str,
    member_id: &str,
    request_id: &str,
) -> Result<ProjectMember, ApiError> {
    sqlx::query_as::<_, ProjectMember>(
        "SELECT CAST(pm.id AS TEXT) AS id,
                CAST(pm.workspace_id AS TEXT) AS workspace_id,
                CAST(pm.project_id AS TEXT) AS project_id,
                CAST(pm.workspace_member_id AS TEXT) AS workspace_member_id,
                wm.external_subject,
                wm.display_name,
                hi.display_name AS github_login,
                pm.role,
                pm.status,
                CAST(pm.created_at AS TEXT) AS created_at,
                CAST(pm.updated_at AS TEXT) AS updated_at
         FROM project_members pm
         JOIN workspace_members wm ON wm.id = pm.workspace_member_id
         LEFT JOIN human_identities hi
           ON hi.workspace_member_id = wm.id AND hi.provider = 'github'
         WHERE CAST(pm.project_id AS TEXT) = $1
           AND CAST(pm.workspace_member_id AS TEXT) = $2",
    )
    .bind(project_id)
    .bind(member_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, project_id = %project_id, member_id = %member_id, "fetch project member failed");
        ApiError::internal(request_id, "failed to fetch project member")
    })
}

async fn ensure_not_last_owner(
    state: &AppState,
    workspace_id: &str,
    request_id: &str,
) -> Result<(), ApiError> {
    let row = sqlx::query_as::<_, OwnerCountRow>(
        "SELECT COUNT(*) AS count
         FROM workspace_members
         WHERE CAST(workspace_id AS TEXT) = $1
           AND role = 'owner'
           AND status = 'active'",
    )
    .bind(workspace_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, "owner count failed");
        ApiError::internal(request_id, "failed to validate workspace owners")
    })?;

    if row.count <= 1 {
        return Err(ApiError::validation_error(
            request_id,
            "workspace must keep at least one active owner",
        ));
    }

    Ok(())
}

async fn validate_project_access(
    state: &AppState,
    workspace_id: &str,
    project_access: &[ProjectAccessRequest],
    request_id: &str,
) -> Result<(), ApiError> {
    for access in project_access {
        validate_member_role(&access.role, false, request_id)?;
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT CAST(id AS TEXT)
             FROM projects
             WHERE CAST(workspace_id AS TEXT) = $1
               AND CAST(id AS TEXT) = $2",
        )
        .bind(workspace_id)
        .bind(&access.project_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project_id = %access.project_id, "project access validation failed");
            ApiError::internal(request_id, "failed to validate project access")
        })?;

        if row.is_none() {
            return Err(ApiError::validation_error(
                request_id,
                format!("project_id '{}' is not in this workspace", access.project_id),
            ));
        }
    }

    Ok(())
}

fn validate_member_role(role: &str, allow_owner: bool, request_id: &str) -> Result<(), ApiError> {
    let valid = if allow_owner {
        ["owner", "editor", "viewer"].contains(&role)
    } else {
        ["editor", "viewer"].contains(&role)
    };

    if !valid {
        let message = if allow_owner {
            "role must be one of: owner, editor, viewer"
        } else {
            "role must be one of: editor, viewer"
        };
        return Err(ApiError::validation_error(request_id, message));
    }

    Ok(())
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
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{app::build_router, db::DatabaseBackend, state::AppState, testing::any_test_pool};

    async fn setup() -> (axum::Router, String, String, String, String) {
        let pool = any_test_pool().await;
        let workspace_id = Uuid::new_v4().to_string();
        let owner_id = Uuid::new_v4().to_string();
        let editor_id = Uuid::new_v4().to_string();
        let project_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)")
            .bind(&workspace_id)
            .bind("team")
            .bind("Team")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO workspace_members (id, workspace_id, external_subject, display_name, role, status)
             VALUES ($1, $2, $3, $4, $5, 'active')",
        )
        .bind(&owner_id)
        .bind(&workspace_id)
        .bind("github:user:owner")
        .bind("Owner")
        .bind("owner")
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO workspace_members (id, workspace_id, external_subject, display_name, role, status)
             VALUES ($1, $2, $3, $4, $5, 'active')",
        )
        .bind(&editor_id)
        .bind(&workspace_id)
        .bind("github:user:editor")
        .bind("Editor")
        .bind("editor")
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, 'active')")
            .bind(&project_id)
            .bind(&workspace_id)
            .bind("app")
            .bind("App")
            .execute(&pool)
            .await
            .unwrap();

        (
            build_router(AppState::new(pool, DatabaseBackend::Sqlite)),
            owner_id,
            editor_id,
            workspace_id,
            project_id,
        )
    }

    #[tokio::test]
    async fn owner_creates_and_lists_invite() {
        let (app, owner_id, _editor_id, _workspace_id, project_id) = setup().await;

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/team/members/invites")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &owner_id)
                    .body(Body::from(
                        json!({
                            "github_login": "new-dev",
                            "role": "editor",
                            "project_access": [{ "project_id": project_id, "role": "editor" }]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::CREATED);
        let create_body = body_json(create.into_body()).await;
        assert_eq!(create_body["data"]["github_login"], "new-dev");
        assert!(create_body["data"]["invite_url"].as_str().unwrap().contains("invite="));

        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/team/members/invites")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &owner_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        let list_body = body_json(list.into_body()).await;
        assert_eq!(list_body["data"]["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn last_owner_cannot_be_disabled() {
        let (app, owner_id, _editor_id, _workspace_id, _project_id) = setup().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/workspaces/team/members/{owner_id}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &owner_id)
                    .body(Body::from(json!({ "status": "disabled" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn project_member_grant_controls_project_visibility() {
        let (app, owner_id, editor_id, _workspace_id, _project_id) = setup().await;

        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/team/projects/app")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &editor_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let grant = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/workspaces/team/projects/app/members/{editor_id}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &owner_id)
                    .body(Body::from(json!({ "role": "viewer" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(grant.status(), StatusCode::OK);

        let allowed = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/team/projects/app")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &editor_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status(), StatusCode::OK);
    }

    async fn body_json(body: Body) -> Value {
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}
