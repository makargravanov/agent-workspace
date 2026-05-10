use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    http::{
        error::ApiError,
        request_id::RequestId,
        response::{ApiResponse, ListData, ResponseMeta},
    },
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SearchResult {
    pub kind: String,
    pub id: String,
    pub workspace_id: Option<String>,
    pub project_id: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub workspace_slug: Option<String>,
    pub project_slug: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/search", get(search))
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
    workspace_id: Option<&str>,
    project_slug: &str,
) -> Result<Option<String>, sqlx::Error> {
    let Some(workspace_id) = workspace_id else {
        return Ok(None);
    };

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

async fn search(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    Query(query): Query<SearchQuery>,
) -> Result<ApiResponse<ListData<SearchResult>>, ApiError> {
    let q = query.q.trim();
    if q.is_empty() {
        return Err(ApiError::validation_error(
            &request_id,
            "q must not be empty",
        ));
    }

    let workspace_id = match query.workspace_slug.as_deref() {
        Some(slug) => Some(
            resolve_workspace(&state.pool, slug)
                .await
                .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
                .ok_or_else(|| ApiError::not_found(&request_id, "workspace not found"))?,
        ),
        None => None,
    };
    let project_id = match query.project_slug.as_deref() {
        Some(slug) => resolve_project(&state.pool, workspace_id.as_deref(), slug)
            .await
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
        None => None,
    };

    let like = format!("%{q}%");

    let mut items = Vec::new();
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'workspace' AS kind,
                    CAST(id AS TEXT) AS id,
                    CAST(id AS TEXT) AS workspace_id,
                    NULL AS project_id,
                    name AS title,
                    slug AS summary,
                    CAST(updated_at AS TEXT) AS updated_at
             FROM workspaces
             WHERE (LOWER(name) LIKE LOWER($1) OR LOWER(slug) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(id AS TEXT) = $2)
             ORDER BY updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'project' AS kind,
                    CAST(p.id AS TEXT) AS id,
                    CAST(p.workspace_id AS TEXT) AS workspace_id,
                    CAST(p.id AS TEXT) AS project_id,
                    p.name AS title,
                    p.slug AS summary,
                    CAST(p.updated_at AS TEXT) AS updated_at
             FROM projects p
             WHERE (LOWER(p.name) LIKE LOWER($1) OR LOWER(p.slug) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(p.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(p.id AS TEXT) = $3)
             ORDER BY p.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'task' AS kind,
                    CAST(t.id AS TEXT) AS id,
                    CAST(t.workspace_id AS TEXT) AS workspace_id,
                    CAST(t.project_id AS TEXT) AS project_id,
                    t.title AS title,
                    COALESCE(t.description_md, '') AS summary,
                    CAST(t.updated_at AS TEXT) AS updated_at
             FROM tasks t
             WHERE (LOWER(t.title) LIKE LOWER($1) OR LOWER(COALESCE(t.description_md, '')) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(t.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(t.project_id AS TEXT) = $3)
             ORDER BY t.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'task_group' AS kind,
                    CAST(g.id AS TEXT) AS id,
                    CAST(g.workspace_id AS TEXT) AS workspace_id,
                    CAST(g.project_id AS TEXT) AS project_id,
                    g.title AS title,
                    COALESCE(g.description_md, '') AS summary,
                    CAST(g.updated_at AS TEXT) AS updated_at
             FROM task_groups g
             WHERE (LOWER(g.title) LIKE LOWER($1) OR LOWER(COALESCE(g.description_md, '')) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(g.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(g.project_id AS TEXT) = $3)
             ORDER BY g.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'note' AS kind,
                    CAST(n.id AS TEXT) AS id,
                    CAST(n.workspace_id AS TEXT) AS workspace_id,
                    CAST(n.project_id AS TEXT) AS project_id,
                    COALESCE(n.title, n.kind) AS title,
                    n.body_md AS summary,
                    CAST(n.updated_at AS TEXT) AS updated_at
             FROM notes n
             WHERE (LOWER(COALESCE(n.title, '')) LIKE LOWER($1) OR LOWER(n.body_md) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(n.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(n.project_id AS TEXT) = $3)
             ORDER BY n.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'document' AS kind,
                    CAST(d.id AS TEXT) AS id,
                    CAST(d.workspace_id AS TEXT) AS workspace_id,
                    CAST(d.project_id AS TEXT) AS project_id,
                    d.title AS title,
                    d.slug AS summary,
                    CAST(d.updated_at AS TEXT) AS updated_at
             FROM documents d
             WHERE (LOWER(d.title) LIKE LOWER($1) OR LOWER(d.slug) LIKE LOWER($1) OR LOWER(d.body_md) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(d.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(d.project_id AS TEXT) = $3)
             ORDER BY d.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'asset' AS kind,
                    CAST(a.id AS TEXT) AS id,
                    CAST(a.workspace_id AS TEXT) AS workspace_id,
                    CAST(a.project_id AS TEXT) AS project_id,
                    a.file_name AS title,
                    COALESCE(a.media_type, '') AS summary,
                    CAST(a.created_at AS TEXT) AS updated_at
             FROM assets a
             WHERE (LOWER(a.file_name) LIKE LOWER($1) OR LOWER(COALESCE(a.media_type, '')) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(a.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(a.project_id AS TEXT) = $3)
             ORDER BY a.created_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'agent' AS kind,
                    CAST(a.id AS TEXT) AS id,
                    CAST(a.workspace_id AS TEXT) AS workspace_id,
                    NULL AS project_id,
                    a.display_name AS title,
                    a.key AS summary,
                    CAST(a.updated_at AS TEXT) AS updated_at
             FROM agents a
             WHERE (LOWER(a.display_name) LIKE LOWER($1) OR LOWER(a.key) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(a.workspace_id AS TEXT) = $2)
             ORDER BY a.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );
    items.extend(
        sqlx::query_as::<_, SearchResult>(
            "SELECT 'integration_connection' AS kind,
                    CAST(c.id AS TEXT) AS id,
                    CAST(c.workspace_id AS TEXT) AS workspace_id,
                    CAST(c.project_id AS TEXT) AS project_id,
                    c.provider AS title,
                    c.status AS summary,
                    CAST(c.updated_at AS TEXT) AS updated_at
             FROM integration_connections c
             WHERE (LOWER(c.provider) LIKE LOWER($1) OR LOWER(c.status) LIKE LOWER($1))
               AND ($2 IS NULL OR CAST(c.workspace_id AS TEXT) = $2)
               AND ($3 IS NULL OR CAST(c.project_id AS TEXT) = $3)
             ORDER BY c.updated_at DESC",
        )
        .bind(&like)
        .bind(workspace_id.as_deref())
        .bind(project_id.as_deref())
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?,
    );

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

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
