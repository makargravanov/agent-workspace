//! BL-20: Tasks foundation
//!
//! Implements the core task CRUD surface:
//!   GET  /workspaces/{workspaceSlug}/projects/{projectSlug}/tasks
//!   POST /workspaces/{workspaceSlug}/projects/{projectSlug}/tasks
//!   GET  /workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}
//!   PATCH /workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}/status
//!
//! Depends on: BL-01 (persistence), BL-02 (HTTP runtime), BL-11 (project context).
//! The project lookup by workspaceSlug + projectSlug is handled inline here until BL-11
//! lands a shared project resolution layer.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::DatabaseBackend,
    http::{
        access::{require_project_access, WorkspaceRole},
        actor::ActorContext,
        audit::{record_audit, AuditEvent},
        error::ApiError,
        request_id::RequestId,
        response::{ApiResponse, Created, ListData, ResponseMeta},
    },
    state::AppState,
};

// ── DB row types ──────────────────────────────────────────────────────────────

/// Full task row including the computed `blocked` field.
#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    project_id: String,
    group_id: Option<String>,
    parent_task_id: Option<String>,
    title: String,
    description_md: Option<String>,
    status: String,
    priority: String,
    rank_key: String,
    starts_at: Option<String>,
    due_at: Option<String>,
    assignee_type: Option<String>,
    assignee_id: Option<String>,
    blocked: i64,
    created_at: String,
    updated_at: String,
}

/// Minimal row returned from the project resolution query.
#[derive(sqlx::FromRow)]
struct ProjectLookupRow {
    id: String,
    workspace_id: String,
}

// ── Public response type (used in tests) ─────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TaskDetail {
    pub id: String,
    pub project_id: String,
    pub group_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub title: String,
    pub description_md: Option<String>,
    pub status: String,
    pub priority: String,
    pub rank_key: String,
    pub starts_at: Option<String>,
    pub due_at: Option<String>,
    pub assignee_type: Option<String>,
    pub assignee_id: Option<String>,
    pub blocked: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TaskRow> for TaskDetail {
    fn from(r: TaskRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            group_id: r.group_id,
            parent_task_id: r.parent_task_id,
            title: r.title,
            description_md: r.description_md,
            status: r.status,
            priority: r.priority,
            rank_key: r.rank_key,
            starts_at: r.starts_at,
            due_at: r.due_at,
            assignee_type: r.assignee_type,
            assignee_id: r.assignee_id,
            blocked: r.blocked != 0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub group_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub description_md: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_rank_key")]
    pub rank_key: String,
    pub assignee_type: Option<String>,
    pub assignee_id: Option<String>,
}

fn default_priority() -> String {
    "normal".to_string()
}

fn default_rank_key() -> String {
    "a0".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub group_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub description_md: Option<String>,
    pub priority: Option<String>,
    pub rank_key: Option<String>,
    pub assignee_type: Option<String>,
    pub assignee_id: Option<String>,
    pub status: Option<String>,
}

/// Optional query filters for the task list endpoint.
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    /// Filter by task status.
    pub status: Option<String>,
    /// Filter by task group (epic / initiative).
    pub group_id: Option<String>,
    /// Filter by assignee UUID.
    pub assignee_id: Option<String>,
    /// Maximum number of tasks to return (1–200, default 50).
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

// ── Route registration ────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/tasks",
            get(list_tasks).post(create_task),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/tasks/{task_id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/tasks/{task_id}/status",
            patch(update_task_status),
        )
}

// ── Repository helpers ────────────────────────────────────────────────────────

/// Look up a project by workspace slug and project slug.
/// Returns `None` when the workspace/project doesn't exist or the project is archived.
async fn resolve_project(
    pool: &sqlx::AnyPool,
    workspace_slug: &str,
    project_slug: &str,
) -> Result<Option<ProjectLookupRow>, sqlx::Error> {
    sqlx::query_as::<_, ProjectLookupRow>(
        "SELECT CAST(p.id AS TEXT) AS id, CAST(p.workspace_id AS TEXT) AS workspace_id
         FROM projects p
         JOIN workspaces w ON w.id = p.workspace_id
         WHERE w.slug = $1 AND p.slug = $2 AND p.status != 'archived'",
    )
    .bind(workspace_slug)
    .bind(project_slug)
    .fetch_optional(pool)
    .await
}

/// SELECT a single task by ID within a project, including the computed `blocked` field.
///
/// `blocked` is `true` when there is at least one `task_dependency` row where:
///   - `successor_task_id = task.id`
///   - `is_hard_block = true`
///   - the predecessor task is not yet `done` or `cancelled`
async fn fetch_task_detail(
    pool: &sqlx::AnyPool,
    project_id: &str,
    task_id: &str,
) -> Result<Option<TaskRow>, sqlx::Error> {
    sqlx::query_as::<_, TaskRow>(
        "SELECT
             CAST(t.id AS TEXT) AS id,
             CAST(t.project_id AS TEXT) AS project_id,
             CAST(t.group_id AS TEXT) AS group_id,
             CAST(t.parent_task_id AS TEXT) AS parent_task_id,
             t.title, t.description_md, t.status, t.priority, t.rank_key,
             CAST(t.starts_at AS TEXT) AS starts_at,
             CAST(t.due_at AS TEXT) AS due_at,
             t.assignee_type,
             CAST(t.assignee_id AS TEXT) AS assignee_id,
             CAST(t.created_at AS TEXT) AS created_at,
             CAST(t.updated_at AS TEXT) AS updated_at,
             CASE WHEN EXISTS (
                 SELECT 1
                 FROM task_dependencies td
                 JOIN tasks blocker ON blocker.id = td.predecessor_task_id
                 WHERE td.successor_task_id = t.id
                   AND td.is_hard_block = true
                   AND blocker.status NOT IN ('done', 'cancelled')
             ) THEN 1 ELSE 0 END AS blocked
         FROM tasks t
         WHERE CAST(t.id AS TEXT) = $1 AND CAST(t.project_id AS TEXT) = $2",
    )
    .bind(task_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /workspaces/:workspace_slug/projects/:project_slug/tasks`
///
/// Returns a page of tasks for the given project, ordered by `rank_key`.
/// Supports optional filtering by `status`, `group_id`, and `assignee_id`.
/// Pagination uses a `limit` parameter (default 50, max 200); cursor-based
/// continuation is deferred to a later iteration (BL-22).
async fn list_tasks(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    Query(query): Query<ListTasksQuery>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
) -> Result<ApiResponse<ListData<TaskDetail>>, ApiError> {
    let project = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, project = %project_slug, "db error resolving project");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &project.workspace_id,
        &project.id,
        WorkspaceRole::Viewer,
        Some("tasks:read"),
        &request_id,
    )
    .await?;

    let limit = query.limit.clamp(1, 200);

    let rows: Vec<TaskRow> = sqlx::query_as(
        "SELECT
             CAST(t.id AS TEXT) AS id,
             CAST(t.project_id AS TEXT) AS project_id,
             CAST(t.group_id AS TEXT) AS group_id,
             CAST(t.parent_task_id AS TEXT) AS parent_task_id,
             t.title, t.description_md, t.status, t.priority, t.rank_key,
             CAST(t.starts_at AS TEXT) AS starts_at,
             CAST(t.due_at AS TEXT) AS due_at,
             t.assignee_type,
             CAST(t.assignee_id AS TEXT) AS assignee_id,
             CAST(t.created_at AS TEXT) AS created_at,
             CAST(t.updated_at AS TEXT) AS updated_at,
             CASE WHEN EXISTS (
                 SELECT 1
                 FROM task_dependencies td
                 JOIN tasks blocker ON blocker.id = td.predecessor_task_id
                 WHERE td.successor_task_id = t.id
                   AND td.is_hard_block = TRUE
                   AND blocker.status NOT IN ('done', 'cancelled')
             ) THEN 1 ELSE 0 END AS blocked
         FROM tasks t
         WHERE CAST(t.project_id AS TEXT) = $1
           AND ($2 IS NULL OR t.status = $2)
           AND ($3 IS NULL OR CAST(t.group_id AS TEXT) = $3)
           AND ($4 IS NULL OR CAST(t.assignee_id AS TEXT) = $4)
         ORDER BY t.rank_key
         LIMIT $5",
    )
    .bind(project.id.clone())
    .bind(query.status.clone())
    .bind(query.group_id.clone())
    .bind(query.assignee_id.clone())
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, project_id = %project.id, "db error listing tasks");
        ApiError::internal(&request_id, "database error")
    })?;

    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)
         FROM tasks t
         WHERE CAST(t.project_id AS TEXT) = $1
           AND ($2 IS NULL OR t.status = $2)
           AND ($3 IS NULL OR CAST(t.group_id AS TEXT) = $3)
           AND ($4 IS NULL OR CAST(t.assignee_id AS TEXT) = $4)",
    )
    .bind(project.id.clone())
    .bind(query.status.clone())
    .bind(query.group_id.clone())
    .bind(query.assignee_id.clone())
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, project_id = %project.id, "db error counting tasks");
        ApiError::internal(&request_id, "database error")
    })?;

    let items: Vec<TaskDetail> = rows.into_iter().map(TaskDetail::from).collect();
    let next_cursor = if total > limit {
        Some(limit.to_string())
    } else {
        None
    };

    Ok(ApiResponse {
        data: ListData { items, next_cursor },
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

/// `GET /workspaces/:workspace_slug/projects/:project_slug/tasks/:task_id`
///
/// Returns the full task detail, including the computed `blocked` field.
async fn get_task(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug, task_id)): Path<(String, String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
) -> Result<ApiResponse<TaskDetail>, ApiError> {
    let project = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, project = %project_slug, "db error resolving project");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &project.workspace_id,
        &project.id,
        WorkspaceRole::Viewer,
        Some("tasks:read"),
        &request_id,
    )
    .await?;

    let task = fetch_task_detail(&state.pool, &project.id, &task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "db error fetching task");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "task not found"))?;

    Ok(ApiResponse {
        data: TaskDetail::from(task),
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

/// `PATCH /workspaces/:workspace_slug/projects/:project_slug/tasks/:task_id`
///
/// Updates task metadata without changing dependency relations. Status may be
/// updated here as well for clients that do not use the dedicated status route.
async fn update_task(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug, task_id)): Path<(String, String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Json(body): Json<UpdateTaskRequest>,
) -> Result<ApiResponse<TaskDetail>, ApiError> {
    if let Some(ref title) = body.title {
        if title.trim().is_empty() {
            return Err(ApiError::validation_error(
                &request_id,
                "title must not be empty",
            ));
        }
    }
    if let Some(ref priority) = body.priority {
        if !["low", "normal", "high", "critical"].contains(&priority.as_str()) {
            return Err(ApiError::validation_error(
                &request_id,
                "priority must be one of: low, normal, high, critical",
            ));
        }
    }
    if let Some(ref status) = body.status {
        if !["todo", "in_progress", "done", "cancelled"].contains(&status.as_str()) {
            return Err(ApiError::validation_error(
                &request_id,
                "status must be one of: todo, in_progress, done, cancelled",
            ));
        }
    }
    if let Some(ref assignee_type) = body.assignee_type {
        if !["workspace_member", "agent"].contains(&assignee_type.as_str()) {
            return Err(ApiError::validation_error(
                &request_id,
                "assignee_type must be one of: workspace_member, agent",
            ));
        }
    }

    let project = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, project = %project_slug, "db error resolving project");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &project.workspace_id,
        &project.id,
        WorkspaceRole::Editor,
        None,
        &request_id,
    )
    .await?;

    let current = fetch_task_detail(&state.pool, &project.id, &task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "db error fetching task for update");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "task not found"))?;

    if let Some(ref group_id) = body.group_id {
        let found: Option<(String,)> = sqlx::query_as(
            "SELECT CAST(id AS TEXT) AS id
             FROM task_groups
             WHERE CAST(id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
        )
        .bind(group_id)
        .bind(&project.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
        if found.is_none() {
            return Err(ApiError::not_found(&request_id, "task group not found"));
        }
    }

    if let Some(ref parent_task_id) = body.parent_task_id {
        if parent_task_id == &task_id {
            return Err(ApiError::validation_error(
                &request_id,
                "task cannot be its own parent",
            ));
        }
        let found: Option<(String,)> = sqlx::query_as(
            "SELECT CAST(id AS TEXT) AS id
             FROM tasks
             WHERE CAST(id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
        )
        .bind(parent_task_id)
        .bind(&project.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
        if found.is_none() {
            return Err(ApiError::not_found(&request_id, "parent task not found"));
        }
    }

    let next_title = body
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or(&current.title);
    let next_description = body
        .description_md
        .as_deref()
        .or(current.description_md.as_deref());
    let next_priority = body.priority.as_deref().unwrap_or(&current.priority);
    let next_rank_key = body.rank_key.as_deref().unwrap_or(&current.rank_key);
    let next_status = body.status.as_deref().unwrap_or(&current.status);
    let next_assignee_type = body
        .assignee_type
        .as_deref()
        .or(current.assignee_type.as_deref());
    let next_assignee_id = body
        .assignee_id
        .as_deref()
        .or(current.assignee_id.as_deref());
    let next_group_id = body.group_id.as_deref().or(current.group_id.as_deref());
    let next_parent_task_id = body
        .parent_task_id
        .as_deref()
        .or(current.parent_task_id.as_deref());

    let update_sql = if state.db_backend == DatabaseBackend::Postgres {
        "UPDATE tasks
            SET title = $1,
                description_md = $2,
                status = $3,
                priority = $4,
                rank_key = $5,
                group_id = CAST($6 AS UUID),
                parent_task_id = CAST($7 AS UUID),
                assignee_type = $8,
                assignee_id = CAST($9 AS UUID),
                updated_at = CURRENT_TIMESTAMP
            WHERE CAST(id AS UUID) = CAST($10 AS UUID)
              AND CAST(project_id AS UUID) = CAST($11 AS UUID)"
    } else {
        "UPDATE tasks
            SET title = $1,
                description_md = $2,
                status = $3,
                priority = $4,
                rank_key = $5,
                group_id = $6,
                parent_task_id = $7,
                assignee_type = $8,
                assignee_id = $9,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $10
              AND project_id = $11"
    };

    let affected = sqlx::query(update_sql)
        .bind(next_title)
        .bind(next_description)
        .bind(next_status)
        .bind(next_priority)
        .bind(next_rank_key)
        .bind(next_group_id)
        .bind(next_parent_task_id)
        .bind(next_assignee_type)
        .bind(next_assignee_id)
        .bind(&task_id)
        .bind(&project.id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "db error updating task");
            ApiError::internal(&request_id, "database error")
        })?
        .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "task not found"));
    }

    let task = fetch_task_detail(&state.pool, &project.id, &task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "db error fetching updated task");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::internal(&request_id, "task not found after update"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task.updated".to_string(),
            resource_kind: "task".to_string(),
            resource_id: task_id,
            payload: None,
        },
    )
    .await;

    Ok(ApiResponse {
        data: TaskDetail::from(task),
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

/// `POST /workspaces/:workspace_slug/projects/:project_slug/tasks`
///
/// Creates a new task in the given project.  The new task always starts with
/// `status = "todo"`.  Returns HTTP 201 with the created task and an audit event ID.
async fn create_task(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Json(body): Json<CreateTaskRequest>,
) -> Result<Created<TaskDetail>, ApiError> {
    // ── Input validation ──────────────────────────────────────────────────────
    if body.title.trim().is_empty() {
        return Err(ApiError::validation_error(
            &request_id,
            "title must not be empty",
        ));
    }
    if !["low", "normal", "high", "critical"].contains(&body.priority.as_str()) {
        return Err(ApiError::validation_error(
            &request_id,
            "priority must be one of: low, normal, high, critical",
        ));
    }
    if let Some(ref at) = body.assignee_type {
        if !["workspace_member", "agent"].contains(&at.as_str()) {
            return Err(ApiError::validation_error(
                &request_id,
                "assignee_type must be one of: workspace_member, agent",
            ));
        }
    }

    // ── Project resolution ────────────────────────────────────────────────────
    let project = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, project = %project_slug, "db error resolving project");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &project.workspace_id,
        &project.id,
        WorkspaceRole::Editor,
        None,
        &request_id,
    )
    .await?;

    // ── Insert ────────────────────────────────────────────────────────────────
    let new_id = Uuid::new_v4().to_string();

    let insert_task_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO tasks
                 (id, workspace_id, project_id, group_id, parent_task_id,
                  rank_key, title, description_md, status, priority,
                  assignee_type, assignee_id)
             VALUES (
                 CAST($1 AS UUID),
                 CAST($2 AS UUID),
                 CAST($3 AS UUID),
                 CAST($4 AS UUID),
                 CAST($5 AS UUID),
                 $6, $7, $8, 'todo', $9, $10, CAST($11 AS UUID)
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO tasks
                 (id, workspace_id, project_id, group_id, parent_task_id,
                  rank_key, title, description_md, status, priority,
                  assignee_type, assignee_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'todo', $9, $10, $11)"
        }
    };

    sqlx::query(insert_task_sql)
        .bind(new_id.clone())
        .bind(project.workspace_id)
        .bind(project.id.clone())
        .bind(body.group_id)
        .bind(body.parent_task_id)
        .bind(&body.rank_key)
        .bind(body.title.trim())
        .bind(&body.description_md)
        .bind(&body.priority)
        .bind(&body.assignee_type)
        .bind(body.assignee_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project_id = %project.id, "db error inserting task");
            ApiError::internal(&request_id, "database error")
        })?;

    // ── Fetch created task ────────────────────────────────────────────────────
    let task = fetch_task_detail(&state.pool, &project.id, &new_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %new_id, "db error fetching created task");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::internal(&request_id, "task not found after insert"))?;

    // ── Audit ─────────────────────────────────────────────────────────────────
    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task.created".to_string(),
            resource_kind: "task".to_string(),
            resource_id: new_id,
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: TaskDetail::from(task),
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

/// `PATCH /workspaces/:workspace_slug/projects/:project_slug/tasks/:task_id/status`
///
/// Updates only the `status` field of a task.  Both human users and agents with
/// the `tasks:write_status` scope may call this endpoint.
async fn update_task_status(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug, task_id)): Path<(String, String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Json(body): Json<UpdateTaskStatusRequest>,
) -> Result<ApiResponse<TaskDetail>, ApiError> {
    // ── Input validation ──────────────────────────────────────────────────────
    if !["todo", "in_progress", "done", "cancelled"].contains(&body.status.as_str()) {
        return Err(ApiError::validation_error(
            &request_id,
            "status must be one of: todo, in_progress, done, cancelled",
        ));
    }

    // ── Project resolution ────────────────────────────────────────────────────
    let project = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, project = %project_slug, "db error resolving project");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &project.workspace_id,
        &project.id,
        WorkspaceRole::Editor,
        Some("tasks:write_status"),
        &request_id,
    )
    .await?;

    // ── Update ────────────────────────────────────────────────────────────────
    let affected = sqlx::query(
        "UPDATE tasks
            SET status = $1, updated_at = CURRENT_TIMESTAMP
            WHERE CAST(id AS TEXT) = $2 AND CAST(project_id AS TEXT) = $3",
    )
    .bind(&body.status)
    .bind(task_id.clone())
    .bind(project.id.clone())
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, task_id = %task_id, "db error updating task status");
        ApiError::internal(&request_id, "database error")
    })?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "task not found"));
    }

    // ── Fetch updated task ────────────────────────────────────────────────────
    let task = fetch_task_detail(&state.pool, &project.id, &task_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, task_id = %task_id, "db error fetching updated task");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::internal(&request_id, "task not found after update"))?;

    // ── Audit ─────────────────────────────────────────────────────────────────
    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task.status_updated".to_string(),
            resource_kind: "task".to_string(),
            resource_id: task_id,
            payload: Some(serde_json::json!({ "new_status": &body.status })),
        },
    )
    .await;

    Ok(ApiResponse {
        data: TaskDetail::from(task),
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

/// `DELETE /workspaces/:workspace_slug/projects/:project_slug/tasks/:task_id`
///
/// Deletes a task within the project after clearing child task parent links and
/// related dependency/session linkage rows.
async fn delete_task(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug, task_id)): Path<(String, String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
) -> Result<StatusCode, ApiError> {
    let project = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, workspace = %workspace_slug, project = %project_slug, "db error resolving project");
            ApiError::internal(&request_id, "database error")
        })?
        .ok_or_else(|| ApiError::not_found(&request_id, "project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &project.workspace_id,
        &project.id,
        WorkspaceRole::Editor,
        None,
        &request_id,
    )
    .await?;

    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!(error = %e, task_id = %task_id, "delete_task tx begin failed");
        ApiError::internal(&request_id, "failed to delete task")
    })?;

    sqlx::query(
        "UPDATE tasks SET parent_task_id = NULL
         WHERE CAST(parent_task_id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&task_id)
    .bind(&project.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, task_id = %task_id, "delete_task child cleanup failed");
        ApiError::internal(&request_id, "failed to delete task")
    })?;

    for sql in [
        "DELETE FROM agent_session_tasks WHERE CAST(task_id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
        "DELETE FROM task_dependencies
         WHERE (CAST(predecessor_task_id AS TEXT) = $1 OR CAST(successor_task_id AS TEXT) = $1)
           AND CAST(project_id AS TEXT) = $2",
        "DELETE FROM tasks WHERE CAST(id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    ] {
        sqlx::query(sql)
            .bind(&task_id)
            .bind(&project.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, task_id = %task_id, "delete_task cascade failed");
                ApiError::internal(&request_id, "failed to delete task")
            })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, task_id = %task_id, "delete_task commit failed");
        ApiError::internal(&request_id, "failed to delete task")
    })?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "task.deleted".to_string(),
            resource_kind: "task".to_string(),
            resource_id: task_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// Two layers of tests:
//
// 1. `validation_tests` — Route-level tests exercising input validation via
//    a lazy (non-connecting) PgPool.  These succeed because validation returns
//    early before any database query is issued.
//
// 2. `repository_semantics` — Tests verifying SQL query logic using the
//    in-memory SQLite pool from `crate::testing`.  They use `?` parameter syntax
//    and SQLite boolean conventions (0/1) to stay compatible with the SQLite
//    driver; the production queries use Postgres-native syntax.

#[cfg(test)]
mod validation_tests {
    use crate::{app::build_router, state::AppState};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn app() -> axum::Router {
        build_router(AppState::new_lazy_for_test())
    }

    async fn post_task(
        app: axum::Router,
        workspace: &str,
        project: &str,
        body: serde_json::Value,
    ) -> axum::http::Response<Body> {
        let uri = format!("/api/v1/workspaces/{workspace}/projects/{project}/tasks");
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn patch_status(
        app: axum::Router,
        workspace: &str,
        project: &str,
        task_id: &str,
        body: serde_json::Value,
    ) -> axum::http::Response<Body> {
        let uri =
            format!("/api/v1/workspaces/{workspace}/projects/{project}/tasks/{task_id}/status");
        app.oneshot(
            Request::builder()
                .method("PATCH")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn create_task_rejects_empty_title() {
        let resp = post_task(app(), "ws", "proj", serde_json::json!({ "title": "" })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_task_rejects_whitespace_only_title() {
        let resp = post_task(app(), "ws", "proj", serde_json::json!({ "title": "   " })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_task_rejects_invalid_priority() {
        let resp = post_task(
            app(),
            "ws",
            "proj",
            serde_json::json!({ "title": "Do something", "priority": "urgent" }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn create_task_rejects_invalid_assignee_type() {
        let resp = post_task(
            app(),
            "ws",
            "proj",
            serde_json::json!({
                "title": "Do something",
                "assignee_type": "bot",
                "assignee_id": "00000000-0000-0000-0000-000000000001"
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn update_status_rejects_invalid_value() {
        let task_id = "00000000-0000-0000-0000-000000000001";
        let resp = patch_status(
            app(),
            "ws",
            "proj",
            task_id,
            serde_json::json!({ "status": "blocked" }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn update_status_rejects_empty_string() {
        let task_id = "00000000-0000-0000-0000-000000000001";
        let resp = patch_status(
            app(),
            "ws",
            "proj",
            task_id,
            serde_json::json!({ "status": "" }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;
    use crate::app::build_router;
    use crate::testing::{any_test_pool, fixtures};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    const RID: &str = "x-request-id";
    const TEST_RID: &str = "test-request-id";
    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";

    async fn setup() -> (
        AppState,
        axum::Router,
        String,
        String,
        String,
        String,
        String,
    ) {
        let pool = any_test_pool().await;
        let state = AppState::new(pool, crate::db::DatabaseBackend::Sqlite);
        let workspace_id = Uuid::new_v4().to_string();
        let member_id = Uuid::new_v4().to_string();
        let project_id = Uuid::new_v4().to_string();
        let parent_task_id = Uuid::new_v4().to_string();
        let child_task_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)")
            .bind(&workspace_id)
            .bind(fixtures::WORKSPACE_SLUG)
            .bind(fixtures::WORKSPACE_NAME)
            .execute(&state.pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&member_id)
        .bind(&workspace_id)
        .bind("test:member-1")
        .bind("Test Member")
        .bind("owner")
        .bind("active")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&project_id)
        .bind(&workspace_id)
        .bind(fixtures::PROJECT_SLUG)
        .bind(fixtures::PROJECT_NAME)
        .bind("active")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO tasks
             (id, workspace_id, project_id, rank_key, title, description_md, status, priority)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&parent_task_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind("rank-a")
        .bind("Parent task")
        .bind("body")
        .bind("todo")
        .bind("normal")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO tasks
             (id, workspace_id, project_id, parent_task_id, rank_key, title, description_md, status, priority)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&child_task_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind(&parent_task_id)
        .bind("rank-b")
        .bind("Child task")
        .bind("body")
        .bind("todo")
        .bind("normal")
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO task_dependencies
             (id, workspace_id, project_id, predecessor_task_id, successor_task_id, dependency_type, is_hard_block)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&workspace_id)
        .bind(&project_id)
        .bind(&parent_task_id)
        .bind(&child_task_id)
        .bind("blocks")
        .bind(true)
        .execute(&state.pool)
        .await
        .unwrap();

        let router = build_router(state.clone());

        (
            state,
            router,
            member_id,
            workspace_id,
            project_id,
            parent_task_id,
            child_task_id,
        )
    }

    #[tokio::test]
    async fn delete_task_removes_task_and_unlinks_children() {
        let (state, router, member_id, _workspace_id, project_id, parent_task_id, child_task_id) =
            setup().await;

        let resp = router
            .oneshot(
                Request::delete(format!(
                    "/api/v1/workspaces/{}/projects/{}/tasks/{parent_task_id}",
                    fixtures::WORKSPACE_SLUG,
                    fixtures::PROJECT_SLUG
                ))
                .header(RID, TEST_RID)
                .header(ACTOR_KIND, "human")
                .header(ACTOR_ID, &member_id)
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let (parent_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ? AND project_id = ?")
                .bind(&parent_task_id)
                .bind(&project_id)
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(parent_count, 0);

        let (child_parent_id,): (Option<String>,) =
            sqlx::query_as("SELECT parent_task_id FROM tasks WHERE id = ? AND project_id = ?")
                .bind(&child_task_id)
                .bind(&project_id)
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert!(child_parent_id.is_none());

        let (dependency_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM task_dependencies WHERE project_id = ?")
                .bind(&project_id)
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(dependency_count, 0);
    }
}

#[cfg(test)]
mod repository_semantics {
    //! Verify SQL query semantics using the in-memory SQLite test pool.
    //!
    //! These tests intentionally duplicate the query logic with SQLite-compatible
    //! syntax (`?` params, boolean stored as INTEGER 0/1) to validate the
    //! behavioural contracts that the production Postgres queries must satisfy.

    use crate::testing::{fixtures, sqlite_test_pool};
    use uuid::Uuid;

    // ── Project resolution ────────────────────────────────────────────────────

    #[tokio::test]
    async fn project_lookup_finds_existing_project() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT p.id, p.workspace_id
             FROM projects p
             JOIN workspaces w ON w.id = p.workspace_id
             WHERE w.slug = ? AND p.slug = ? AND p.status != 'archived'",
        )
        .bind(fixtures::WORKSPACE_SLUG)
        .bind(fixtures::PROJECT_SLUG)
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(row.is_some(), "should find the seeded project");
        let (project_id, workspace_id) = row.unwrap();
        assert_eq!(project_id, seed.project_id.to_string());
        assert_eq!(workspace_id, seed.workspace_id.to_string());
    }

    #[tokio::test]
    async fn project_lookup_returns_none_for_unknown_slugs() {
        let pool = sqlite_test_pool().await;
        let _seed = fixtures::seed_minimal(&pool).await;

        let row: Option<(String,)> = sqlx::query_as(
            "SELECT p.id
             FROM projects p
             JOIN workspaces w ON w.id = p.workspace_id
             WHERE w.slug = ? AND p.slug = ? AND p.status != 'archived'",
        )
        .bind("nonexistent-workspace")
        .bind("nonexistent-project")
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(row.is_none());
    }

    #[tokio::test]
    async fn project_lookup_excludes_archived_project() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        sqlx::query("UPDATE projects SET status = 'archived' WHERE id = ?")
            .bind(seed.project_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let row: Option<(String,)> = sqlx::query_as(
            "SELECT p.id
             FROM projects p
             JOIN workspaces w ON w.id = p.workspace_id
             WHERE w.slug = ? AND p.slug = ? AND p.status != 'archived'",
        )
        .bind(fixtures::WORKSPACE_SLUG)
        .bind(fixtures::PROJECT_SLUG)
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(row.is_none(), "archived project should be excluded");
    }

    // ── Blocked computation ───────────────────────────────────────────────────

    async fn is_task_blocked(pool: &sqlx::SqlitePool, task_id: &str) -> bool {
        let (v,): (i64,) = sqlx::query_as(
            "SELECT CASE WHEN EXISTS (
                 SELECT 1
                 FROM task_dependencies td
                 JOIN tasks blocker ON blocker.id = td.predecessor_task_id
                 WHERE td.successor_task_id = ?
                   AND td.is_hard_block = 1
                   AND blocker.status NOT IN ('done', 'cancelled')
             ) THEN 1 ELSE 0 END",
        )
        .bind(task_id)
        .fetch_one(pool)
        .await
        .unwrap();
        v == 1
    }

    #[tokio::test]
    async fn task_is_blocked_by_active_predecessor() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        // seed_minimal: task[0] (todo) hard-blocks task[1] (in_progress)
        assert!(
            is_task_blocked(&pool, &seed.task_ids[1].to_string()).await,
            "task[1] should be blocked by task[0] (todo)"
        );
    }

    #[tokio::test]
    async fn task_is_not_blocked_without_predecessors() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        assert!(
            !is_task_blocked(&pool, &seed.task_ids[0].to_string()).await,
            "task[0] has no predecessor, must not be blocked"
        );
    }

    #[tokio::test]
    async fn task_becomes_unblocked_when_predecessor_done() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        assert!(is_task_blocked(&pool, &seed.task_ids[1].to_string()).await);

        sqlx::query("UPDATE tasks SET status = 'done' WHERE id = ?")
            .bind(seed.task_ids[0].to_string())
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            !is_task_blocked(&pool, &seed.task_ids[1].to_string()).await,
            "task[1] should be unblocked after predecessor completes"
        );
    }

    #[tokio::test]
    async fn cancelled_predecessor_does_not_block_successor() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        sqlx::query("UPDATE tasks SET status = 'cancelled' WHERE id = ?")
            .bind(seed.task_ids[0].to_string())
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            !is_task_blocked(&pool, &seed.task_ids[1].to_string()).await,
            "cancelled predecessor should not block successor"
        );
    }

    // ── Task list ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_returns_all_project_tasks_ordered_by_rank_key() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM tasks WHERE project_id = ? ORDER BY rank_key")
                .bind(seed.project_id.to_string())
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, seed.task_ids[0].to_string()); // rank-a
        assert_eq!(rows[1].0, seed.task_ids[1].to_string()); // rank-b
        assert_eq!(rows[2].0, seed.task_ids[2].to_string()); // rank-c
    }

    #[tokio::test]
    async fn list_status_filter_narrows_results() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM tasks
             WHERE project_id = ?
               AND (? IS NULL OR status = ?)
             ORDER BY rank_key",
        )
        .bind(seed.project_id.to_string())
        .bind(Some("todo"))
        .bind(Some("todo"))
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, seed.task_ids[0].to_string());
    }

    // ── Status update ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn status_update_changes_exactly_one_row() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let affected = sqlx::query(
            "UPDATE tasks
             SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND project_id = ?",
        )
        .bind("in_progress")
        .bind(seed.task_ids[0].to_string())
        .bind(seed.project_id.to_string())
        .execute(&pool)
        .await
        .unwrap()
        .rows_affected();

        assert_eq!(affected, 1);

        let (status,): (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
            .bind(seed.task_ids[0].to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "in_progress");
    }

    #[tokio::test]
    async fn status_update_returns_zero_rows_for_wrong_project() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let affected = sqlx::query(
            "UPDATE tasks
             SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND project_id = ?",
        )
        .bind("done")
        .bind(seed.task_ids[0].to_string())
        .bind(Uuid::new_v4().to_string()) // wrong project
        .execute(&pool)
        .await
        .unwrap()
        .rows_affected();

        assert_eq!(
            affected, 0,
            "must not update a task belonging to a different project"
        );
    }
}
