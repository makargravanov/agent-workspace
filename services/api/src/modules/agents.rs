use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::DatabaseBackend,
    http::{
        access::{require_human_workspace_role, WorkspaceRole},
        actor::ActorContext,
        audit::{record_audit, AuditEvent},
        error::ApiError,
        request_id::RequestId,
        response::{ApiResponse, Created, ListData, ResponseMeta},
    },
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentResponse {
    pub id: String,
    pub workspace_id: String,
    pub created_by_member_id: String,
    pub key: String,
    pub display_name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub key: String,
    pub display_name: String,
    #[serde(default = "default_status")]
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub key: Option<String>,
    pub display_name: Option<String>,
    pub status: Option<String>,
}

fn default_status() -> String {
    "active".to_string()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/agents",
            get(list_agents).post(create_agent),
        )
        .route(
            "/workspaces/{workspace_slug}/agents/{agent_id}",
            get(get_agent).patch(update_agent).delete(delete_agent),
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

fn validate_key(value: &str, request_id: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::validation_error(
            request_id,
            "key must not be empty",
        ));
    }
    let valid = value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid || value.starts_with('-') || value.ends_with('-') {
        return Err(ApiError::validation_error(
            request_id,
            "key must be lowercase kebab-case (a-z, 0-9, hyphens; no leading/trailing hyphen)",
        ));
    }
    Ok(())
}

fn validate_status(value: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(value, "active" | "disabled") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "status must be one of: active, disabled",
        ))
    }
}

async fn fetch_agent(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    agent_id: &str,
) -> Result<Option<AgentResponse>, sqlx::Error> {
    sqlx::query_as::<_, AgentResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(created_by_member_id AS TEXT) AS created_by_member_id,
                key,
                display_name,
                status,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM agents
         WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(workspace_id)
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}

async fn list_agents(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<ListData<AgentResponse>>, ApiError> {
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

    let items = sqlx::query_as::<_, AgentResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(created_by_member_id AS TEXT) AS created_by_member_id,
                key,
                display_name,
                status,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM agents
         WHERE CAST(workspace_id AS TEXT) = $1
         ORDER BY created_at DESC",
    )
    .bind(&workspace_id)
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

async fn create_agent(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
    Json(body): Json<CreateAgentRequest>,
) -> Result<Created<AgentResponse>, ApiError> {
    validate_key(&body.key, &request_id)?;
    validate_status(&body.status, &request_id)?;

    let workspace_id = resolve_workspace(&state.pool, &workspace_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace not found"))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace_id,
        WorkspaceRole::Editor,
        &request_id,
    )
    .await?;

    let agent_id = Uuid::new_v4().to_string();
    let insert_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO agents
             (id, workspace_id, created_by_member_id, key, display_name, status)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), CAST($3 AS UUID), $4, $5, $6)
             RETURNING CAST(id AS TEXT) AS id,
                       CAST(workspace_id AS TEXT) AS workspace_id,
                       CAST(created_by_member_id AS TEXT) AS created_by_member_id,
                       key,
                       display_name,
                       status,
                       CAST(created_at AS TEXT) AS created_at,
                       CAST(updated_at AS TEXT) AS updated_at"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO agents
             (id, workspace_id, created_by_member_id, key, display_name, status)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id,
                       workspace_id,
                       created_by_member_id,
                       key,
                       display_name,
                       status,
                       created_at,
                       updated_at"
        }
    };

    let agent = sqlx::query_as::<_, AgentResponse>(insert_sql)
        .bind(&agent_id)
        .bind(&workspace_id)
        .bind(&actor.actor_id)
        .bind(&body.key)
        .bind(body.display_name.trim())
        .bind(&body.status)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "agent.created".to_string(),
            resource_kind: "agent".to_string(),
            resource_id: agent_id,
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: agent,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn get_agent(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, agent_id)): Path<(String, String)>,
) -> Result<ApiResponse<AgentResponse>, ApiError> {
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

    let agent = fetch_agent(&state.pool, &workspace_id, &agent_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "agent not found"))?;

    Ok(ApiResponse {
        data: agent,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn update_agent(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, agent_id)): Path<(String, String)>,
    Json(body): Json<UpdateAgentRequest>,
) -> Result<ApiResponse<AgentResponse>, ApiError> {
    let workspace_id = resolve_workspace(&state.pool, &workspace_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace not found"))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace_id,
        WorkspaceRole::Editor,
        &request_id,
    )
    .await?;

    if let Some(ref key) = body.key {
        validate_key(key, &request_id)?;
    }
    if let Some(ref status) = body.status {
        validate_status(status, &request_id)?;
    }
    if let Some(ref display_name) = body.display_name {
        if display_name.trim().is_empty() {
            return Err(ApiError::validation_error(
                &request_id,
                "display_name must not be empty",
            ));
        }
    }

    let current = fetch_agent(&state.pool, &workspace_id, &agent_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "agent not found"))?;

    sqlx::query(
        "UPDATE agents
         SET key = COALESCE($1, key),
             display_name = COALESCE($2, display_name),
             status = COALESCE($3, status),
             updated_at = CURRENT_TIMESTAMP
         WHERE CAST(id AS TEXT) = $4 AND CAST(workspace_id AS TEXT) = $5",
    )
    .bind(body.key.as_deref())
    .bind(body.display_name.as_deref().map(str::trim))
    .bind(body.status.as_deref())
    .bind(&agent_id)
    .bind(&workspace_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let updated = fetch_agent(&state.pool, &workspace_id, &agent_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "agent not found after update"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "agent.updated".to_string(),
            resource_kind: "agent".to_string(),
            resource_id: agent_id,
            payload: Some(serde_json::json!({ "previous_key": current.key })),
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

async fn delete_agent(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, agent_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let workspace_id = resolve_workspace(&state.pool, &workspace_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace not found"))?;

    require_human_workspace_role(
        &state.pool,
        &actor,
        &workspace_id,
        WorkspaceRole::Editor,
        &request_id,
    )
    .await?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    sqlx::query("DELETE FROM agent_credentials WHERE CAST(agent_id AS TEXT) = $1")
        .bind(&agent_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    sqlx::query("DELETE FROM agent_session_tasks WHERE CAST(agent_session_id AS TEXT) IN (SELECT CAST(id AS TEXT) FROM agent_sessions WHERE CAST(agent_id AS TEXT) = $1)")
        .bind(&agent_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    sqlx::query("DELETE FROM agent_sessions WHERE CAST(agent_id AS TEXT) = $1")
        .bind(&agent_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let affected = sqlx::query(
        "DELETE FROM agents
         WHERE CAST(id AS TEXT) = $1 AND CAST(workspace_id AS TEXT) = $2",
    )
    .bind(&agent_id)
    .bind(&workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "agent not found"));
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
            action: "agent.deleted".to_string(),
            resource_kind: "agent".to_string(),
            resource_id: agent_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
