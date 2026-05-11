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
        actor::hash_secret,
        actor::ActorContext,
        audit::{record_audit, AuditEvent},
        error::ApiError,
        request_id::RequestId,
        response::{ApiResponse, Created, ListData, ResponseMeta},
    },
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentCredentialResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub agent_id: String,
    pub issued_by_member_id: String,
    pub label: String,
    pub secret_prefix: String,
    pub scope_policy: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatedCredentialResponse {
    pub credential: AgentCredentialResponse,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentCredentialRequest {
    pub label: String,
    pub project_id: Option<String>,
    #[serde(default)]
    pub scope_policy: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentCredentialRequest {
    pub label: Option<String>,
    pub project_id: Option<Option<String>>,
    pub scope_policy: Option<Vec<String>>,
    pub status: Option<String>,
    pub expires_at: Option<Option<String>>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/agents/{agent_id}/credentials",
            get(list_credentials).post(create_credential),
        )
        .route(
            "/workspaces/{workspace_slug}/agent-credentials/{credential_id}",
            get(get_credential)
                .patch(update_credential)
                .delete(delete_credential),
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

async fn resolve_agent(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    agent_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_as::<_, (String,)>(
        "SELECT CAST(id AS TEXT) FROM agents WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(workspace_id)
    .bind(agent_id)
    .fetch_optional(pool)
    .await
    .map(|row| row.map(|(id,)| id))
}

async fn resolve_project(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    project_id: &str,
) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM projects WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(workspace_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

async fn fetch_credential(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    credential_id: &str,
) -> Result<Option<AgentCredentialResponse>, sqlx::Error> {
    sqlx::query_as::<_, AgentCredentialResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                CAST(agent_id AS TEXT) AS agent_id,
                CAST(issued_by_member_id AS TEXT) AS issued_by_member_id,
                label,
                secret_prefix,
                CAST(scope_policy AS TEXT) AS scope_policy,
                status,
                CAST(expires_at AS TEXT) AS expires_at,
                CAST(last_used_at AS TEXT) AS last_used_at,
                CAST(created_at AS TEXT) AS created_at,
                CAST(revoked_at AS TEXT) AS revoked_at
         FROM agent_credentials
         WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(workspace_id)
    .bind(credential_id)
    .fetch_optional(pool)
    .await
}

fn validate_label(value: &str, request_id: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        Err(ApiError::validation_error(
            request_id,
            "label must not be empty",
        ))
    } else {
        Ok(())
    }
}

fn validate_status(value: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(value, "active" | "revoked") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "status must be one of: active, revoked",
        ))
    }
}

async fn list_credentials(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, agent_id)): Path<(String, String)>,
) -> Result<ApiResponse<ListData<AgentCredentialResponse>>, ApiError> {
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

    if resolve_agent(&state.pool, &workspace_id, &agent_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .is_none()
    {
        return Err(ApiError::not_found(&request_id, "agent not found"));
    }

    let items = sqlx::query_as::<_, AgentCredentialResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                CAST(agent_id AS TEXT) AS agent_id,
                CAST(issued_by_member_id AS TEXT) AS issued_by_member_id,
                label,
                secret_prefix,
                CAST(scope_policy AS TEXT) AS scope_policy,
                status,
                CAST(expires_at AS TEXT) AS expires_at,
                CAST(last_used_at AS TEXT) AS last_used_at,
                CAST(created_at AS TEXT) AS created_at,
                CAST(revoked_at AS TEXT) AS revoked_at
         FROM agent_credentials
         WHERE CAST(agent_id AS TEXT) = $1
         ORDER BY created_at DESC",
    )
    .bind(&agent_id)
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

async fn create_credential(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, agent_id)): Path<(String, String)>,
    Json(body): Json<CreateAgentCredentialRequest>,
) -> Result<Created<CreatedCredentialResponse>, ApiError> {
    validate_label(&body.label, &request_id)?;

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

    if resolve_agent(&state.pool, &workspace_id, &agent_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .is_none()
    {
        return Err(ApiError::not_found(&request_id, "agent not found"));
    }

    if let Some(ref project_id) = body.project_id {
        if !resolve_project(&state.pool, &workspace_id, project_id)
            .await
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        {
            return Err(ApiError::not_found(
                &request_id,
                "project not found in this workspace",
            ));
        }
    }

    let credential_id = Uuid::new_v4().to_string();
    let secret_prefix = Uuid::new_v4().simple().to_string()[..8].to_string();
    let secret = format!("awcred_{}_{}", secret_prefix, Uuid::new_v4().simple());
    let secret_hash = hash_secret(&secret);

    let insert_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO agent_credentials
             (id, workspace_id, project_id, agent_id, issued_by_member_id, label, secret_prefix, secret_hash, scope_policy, status, expires_at)
             VALUES (
                CAST($1 AS UUID),
                CAST($2 AS UUID),
                CAST($3 AS UUID),
                CAST($4 AS UUID),
                CAST($5 AS UUID),
                $6, $7, $8, CAST($9 AS JSONB), 'active', CAST($10 AS TIMESTAMPTZ)
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO agent_credentials
             (id, workspace_id, project_id, agent_id, issued_by_member_id, label, secret_prefix, secret_hash, scope_policy, status, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10)"
        }
    };

    sqlx::query(insert_sql)
        .bind(&credential_id)
        .bind(&workspace_id)
        .bind(&body.project_id)
        .bind(&agent_id)
        .bind(&actor.actor_id)
        .bind(&body.label)
        .bind(&secret_prefix)
        .bind(&secret_hash)
        .bind(serde_json::to_string(&body.scope_policy).unwrap())
        .bind(body.expires_at.as_deref())
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let credential = fetch_credential(&state.pool, &workspace_id, &credential_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "credential not found after insert"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "agent_credential.created".to_string(),
            resource_kind: "agent_credential".to_string(),
            resource_id: credential_id,
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: CreatedCredentialResponse { credential, secret },
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn get_credential(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, credential_id)): Path<(String, String)>,
) -> Result<ApiResponse<AgentCredentialResponse>, ApiError> {
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

    let credential = fetch_credential(&state.pool, &workspace_id, &credential_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "credential not found"))?;

    Ok(ApiResponse {
        data: credential,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn update_credential(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, credential_id)): Path<(String, String)>,
    Json(body): Json<UpdateAgentCredentialRequest>,
) -> Result<ApiResponse<AgentCredentialResponse>, ApiError> {
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

    if let Some(ref label) = body.label {
        validate_label(label, &request_id)?;
    }
    if let Some(ref status) = body.status {
        validate_status(status, &request_id)?;
    }
    if let Some(Some(ref project_id)) = body.project_id {
        if !resolve_project(&state.pool, &workspace_id, project_id)
            .await
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        {
            return Err(ApiError::not_found(
                &request_id,
                "project not found in this workspace",
            ));
        }
    }

    let update_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE agent_credentials
             SET label = COALESCE($1, label),
                 project_id = COALESCE(CAST($2 AS UUID), project_id),
                 scope_policy = COALESCE(CAST($3 AS JSONB), scope_policy),
                 status = COALESCE($4, status),
                 expires_at = COALESCE(CAST($5 AS TIMESTAMPTZ), expires_at),
                 revoked_at = CASE WHEN $4 = 'revoked' THEN CURRENT_TIMESTAMP ELSE revoked_at END
             WHERE CAST(id AS UUID) = CAST($6 AS UUID)
               AND CAST(workspace_id AS UUID) = CAST($7 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE agent_credentials
             SET label = COALESCE($1, label),
                 project_id = COALESCE($2, project_id),
                 scope_policy = COALESCE($3, scope_policy),
                 status = COALESCE($4, status),
                 expires_at = COALESCE($5, expires_at),
                 revoked_at = CASE WHEN $4 = 'revoked' THEN CURRENT_TIMESTAMP ELSE revoked_at END
             WHERE CAST(id AS TEXT) = $6
               AND CAST(workspace_id AS TEXT) = $7"
        }
    };

    sqlx::query(update_sql)
        .bind(body.label.as_deref())
        .bind(body.project_id.as_ref().and_then(|value| value.as_deref()))
        .bind(
            body.scope_policy
                .as_ref()
                .map(|items| serde_json::to_string(items).unwrap()),
        )
        .bind(body.status.as_deref())
        .bind(body.expires_at.as_ref().and_then(|value| value.as_deref()))
        .bind(&credential_id)
        .bind(&workspace_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let credential = fetch_credential(&state.pool, &workspace_id, &credential_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "credential not found after update"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "agent_credential.updated".to_string(),
            resource_kind: "agent_credential".to_string(),
            resource_id: credential_id,
            payload: None,
        },
    )
    .await;

    Ok(ApiResponse {
        data: credential,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn delete_credential(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, credential_id)): Path<(String, String)>,
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

    let affected = sqlx::query(
        "DELETE FROM agent_credentials
         WHERE CAST(id AS TEXT) = $1 AND CAST(workspace_id AS TEXT) = $2",
    )
    .bind(&credential_id)
    .bind(&workspace_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "credential not found"));
    }

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "agent_credential.deleted".to_string(),
            resource_kind: "agent_credential".to_string(),
            resource_id: credential_id,
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
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{
        app::build_router,
        db::DatabaseBackend,
        testing::{
            any_test_pool, assert_api_error_code, assert_status_with_body, json_request,
            seed_workspace_member_project,
        },
    };

    const WS_SLUG: &str = "ops-workspace";
    const PROJECT_SLUG: &str = "ops-project";

    async fn setup() -> (axum::Router, String) {
        let pool = any_test_pool().await;
        let scope = seed_workspace_member_project(
            &pool,
            WS_SLUG,
            "Ops Workspace",
            PROJECT_SLUG,
            "Ops Project",
            "test:credential-owner",
            "Credential Owner",
            "owner",
        )
        .await;
        let router = build_router(AppState::new(pool, DatabaseBackend::Sqlite));
        (router, scope.member_id)
    }

    fn json_req(
        builder: axum::http::request::Builder,
        member_id: &str,
        value: &serde_json::Value,
    ) -> Request<Body> {
        json_request(
            builder
                .header("x-actor-kind", "human")
                .header("x-actor-id", member_id),
            value,
        )
    }

    async fn create_agent(app: &axum::Router, member_id: &str) -> String {
        let response = app
            .clone()
            .oneshot(json_req(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/workspaces/{WS_SLUG}/agents")),
                member_id,
                &json!({ "key": "ops-bot", "display_name": "Ops Bot" }),
            ))
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::CREATED).await;
        body["data"]["id"].as_str().unwrap().to_string()
    }

    fn assert_scope_policy_json(value: &Value, expected: &[&str]) {
        let parsed: Value = serde_json::from_str(value.as_str().expect("scope policy string"))
            .expect("scope policy should be valid json");
        assert_eq!(parsed, json!(expected));
    }

    #[tokio::test]
    async fn create_and_get_credential_without_revealing_secret_again() {
        let (app, member_id) = setup().await;
        let agent_id = create_agent(&app, &member_id).await;

        let created = app
            .clone()
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/agents/{agent_id}/credentials"
                )),
                &member_id,
                &json!({ "label": "ops shell", "scope_policy": ["tasks:read", "tasks:write"] }),
            ))
            .await
            .unwrap();
        let created_body = assert_status_with_body(created, StatusCode::CREATED).await;
        assert!(created_body["data"]["secret"]
            .as_str()
            .unwrap()
            .starts_with("awcred_"));
        let credential_id = created_body["data"]["credential"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/agents/{agent_id}/credentials"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list_body = assert_status_with_body(listed, StatusCode::OK).await;
        assert_eq!(list_body["data"]["items"][0]["label"], "ops shell");
        assert!(list_body["data"]["items"][0].get("secret").is_none());

        let fetched = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/agent-credentials/{credential_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let fetched_body = assert_status_with_body(fetched, StatusCode::OK).await;
        assert_eq!(fetched_body["data"]["id"], credential_id);
        assert!(fetched_body["data"].get("secret").is_none());
    }

    #[tokio::test]
    async fn create_credential_rejects_unknown_project() {
        let (app, member_id) = setup().await;
        let agent_id = create_agent(&app, &member_id).await;

        let response = app
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/agents/{agent_id}/credentials"
                )),
                &member_id,
                &json!({
                    "label": "ops shell",
                    "project_id": Uuid::new_v4().to_string(),
                    "scope_policy": ["tasks:read"]
                }),
            ))
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::NOT_FOUND).await;
        assert_api_error_code(&body, "not_found");
    }

    #[tokio::test]
    async fn update_credential_changes_label_scopes_and_status() {
        let (app, member_id) = setup().await;
        let agent_id = create_agent(&app, &member_id).await;
        let created = app
            .clone()
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/agents/{agent_id}/credentials"
                )),
                &member_id,
                &json!({ "label": "ops shell", "scope_policy": ["tasks:read"] }),
            ))
            .await
            .unwrap();
        let created_body = assert_status_with_body(created, StatusCode::CREATED).await;
        let credential_id = created_body["data"]["credential"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let updated = app
            .oneshot(json_req(
                Request::builder().method("PATCH").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/agent-credentials/{credential_id}"
                )),
                &member_id,
                &json!({
                    "label": "ops shell v2",
                    "scope_policy": ["tasks:read", "tasks:write"],
                    "status": "revoked"
                }),
            ))
            .await
            .unwrap();
        let updated_body = assert_status_with_body(updated, StatusCode::OK).await;
        assert_eq!(updated_body["data"]["label"], "ops shell v2");
        assert_eq!(updated_body["data"]["status"], "revoked");
        assert_scope_policy_json(&updated_body["data"]["scope_policy"], &["tasks:read", "tasks:write"]);
    }

    #[tokio::test]
    async fn delete_credential_removes_it() {
        let (app, member_id) = setup().await;
        let agent_id = create_agent(&app, &member_id).await;
        let created = app
            .clone()
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/agents/{agent_id}/credentials"
                )),
                &member_id,
                &json!({ "label": "ops shell", "scope_policy": ["tasks:read"] }),
            ))
            .await
            .unwrap();
        let created_body = assert_status_with_body(created, StatusCode::CREATED).await;
        let credential_id = created_body["data"]["credential"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let deleted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/agent-credentials/{credential_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

        let fetched = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/agent-credentials/{credential_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = assert_status_with_body(fetched, StatusCode::NOT_FOUND).await;
        assert_api_error_code(&body, "not_found");
    }
}
