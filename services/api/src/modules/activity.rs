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
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                actor_type,
                CAST(actor_id AS TEXT) AS actor_id,
                entity_type,
                CAST(entity_id AS TEXT) AS entity_id,
                event_type,
                CAST(payload_json AS TEXT) AS payload_json,
                CAST(occurred_at AS TEXT) AS occurred_at
         FROM audit_events
         WHERE CAST(workspace_id AS TEXT) = $1
         ORDER BY occurred_at DESC
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
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                actor_type,
                CAST(actor_id AS TEXT) AS actor_id,
                entity_type,
                CAST(entity_id AS TEXT) AS entity_id,
                event_type,
                CAST(payload_json AS TEXT) AS payload_json,
                CAST(occurred_at AS TEXT) AS occurred_at
         FROM audit_events
         WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2
         ORDER BY occurred_at DESC
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
