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
pub struct IntegrationConnectionResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub provider: String,
    pub scope_kind: String,
    pub status: String,
    pub config_json: Option<String>,
    pub secret_ciphertext: Option<String>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateIntegrationConnectionRequest {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub scope_kind: String,
    pub project_id: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    pub config_json: Option<serde_json::Value>,
    pub secret_ciphertext: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIntegrationConnectionRequest {
    pub provider: Option<String>,
    pub scope_kind: Option<String>,
    pub project_id: Option<Option<String>>,
    pub status: Option<String>,
    pub config_json: Option<Option<serde_json::Value>>,
    pub secret_ciphertext: Option<Option<String>>,
}

fn default_provider() -> String {
    "github".to_string()
}

fn default_status() -> String {
    "active".to_string()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/integration-connections",
            get(list_connections).post(create_connection),
        )
        .route(
            "/workspaces/{workspace_slug}/integration-connections/{connection_id}",
            get(get_connection)
                .patch(update_connection)
                .delete(delete_connection),
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

fn validate_provider(provider: &str, request_id: &str) -> Result<(), ApiError> {
    if provider == "github" {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "provider must be github",
        ))
    }
}

fn validate_scope_kind(scope_kind: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(scope_kind, "workspace" | "project") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "scope_kind must be one of: workspace, project",
        ))
    }
}

fn validate_status(status: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(status, "active" | "disabled" | "error") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "status must be one of: active, disabled, error",
        ))
    }
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

async fn fetch_connection(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    connection_id: &str,
) -> Result<Option<IntegrationConnectionResponse>, sqlx::Error> {
    sqlx::query_as::<_, IntegrationConnectionResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                provider,
                scope_kind,
                status,
                CAST(config_json AS TEXT) AS config_json,
                secret_ciphertext,
                CAST(last_synced_at AS TEXT) AS last_synced_at,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM integration_connections
         WHERE CAST(workspace_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(workspace_id)
    .bind(connection_id)
    .fetch_optional(pool)
    .await
}

async fn list_connections(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
) -> Result<ApiResponse<ListData<IntegrationConnectionResponse>>, ApiError> {
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

    let items = sqlx::query_as::<_, IntegrationConnectionResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                provider,
                scope_kind,
                status,
                CAST(config_json AS TEXT) AS config_json,
                secret_ciphertext,
                CAST(last_synced_at AS TEXT) AS last_synced_at,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM integration_connections
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

async fn create_connection(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path(workspace_slug): Path<String>,
    Json(body): Json<CreateIntegrationConnectionRequest>,
) -> Result<Created<IntegrationConnectionResponse>, ApiError> {
    validate_provider(&body.provider, &request_id)?;
    validate_scope_kind(&body.scope_kind, &request_id)?;
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

    if let Some(ref project_id) = body.project_id {
        if body.scope_kind != "project" {
            return Err(ApiError::validation_error(
                &request_id,
                "project_id is only allowed when scope_kind = project",
            ));
        }
        if !resolve_project(&state.pool, &workspace_id, project_id)
            .await
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        {
            return Err(ApiError::not_found(
                &request_id,
                "project not found in this workspace",
            ));
        }
    } else if body.scope_kind == "project" {
        return Err(ApiError::validation_error(
            &request_id,
            "project_id is required when scope_kind = project",
        ));
    }

    let connection_id = Uuid::new_v4().to_string();
    let insert_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO integration_connections
             (id, workspace_id, project_id, provider, scope_kind, status, config_json, secret_ciphertext)
             VALUES (
                CAST($1 AS UUID),
                CAST($2 AS UUID),
                CAST($3 AS UUID),
                $4, $5, $6, CAST($7 AS JSONB), $8
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO integration_connections
             (id, workspace_id, project_id, provider, scope_kind, status, config_json, secret_ciphertext)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        }
    };

    sqlx::query(insert_sql)
        .bind(&connection_id)
        .bind(&workspace_id)
        .bind(&body.project_id)
        .bind(&body.provider)
        .bind(&body.scope_kind)
        .bind(&body.status)
        .bind(
            body.config_json
                .as_ref()
                .map(|value| serde_json::to_string(value).unwrap()),
        )
        .bind(&body.secret_ciphertext)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let connection = fetch_connection(&state.pool, &workspace_id, &connection_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "connection not found after insert"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "integration_connection.created".to_string(),
            resource_kind: "integration_connection".to_string(),
            resource_id: connection_id,
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: connection,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn get_connection(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, connection_id)): Path<(String, String)>,
) -> Result<ApiResponse<IntegrationConnectionResponse>, ApiError> {
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

    let connection = fetch_connection(&state.pool, &workspace_id, &connection_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "connection not found"))?;

    Ok(ApiResponse {
        data: connection,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn update_connection(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, connection_id)): Path<(String, String)>,
    Json(body): Json<UpdateIntegrationConnectionRequest>,
) -> Result<ApiResponse<IntegrationConnectionResponse>, ApiError> {
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

    if let Some(ref provider) = body.provider {
        validate_provider(provider, &request_id)?;
    }
    if let Some(ref scope_kind) = body.scope_kind {
        validate_scope_kind(scope_kind, &request_id)?;
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

    sqlx::query(match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE integration_connections
             SET provider = COALESCE($1, provider),
                 scope_kind = COALESCE($2, scope_kind),
                 project_id = COALESCE(CAST($3 AS UUID), project_id),
                 status = COALESCE($4, status),
                 config_json = COALESCE(CAST($5 AS JSONB), config_json),
                 secret_ciphertext = COALESCE($6, secret_ciphertext),
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS TEXT) = $7 AND CAST(workspace_id AS TEXT) = $8"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE integration_connections
             SET provider = COALESCE($1, provider),
                 scope_kind = COALESCE($2, scope_kind),
                 project_id = COALESCE($3, project_id),
                 status = COALESCE($4, status),
                 config_json = COALESCE($5, config_json),
                 secret_ciphertext = COALESCE($6, secret_ciphertext),
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS TEXT) = $7 AND CAST(workspace_id AS TEXT) = $8"
        }
    })
    .bind(body.provider.as_deref())
    .bind(body.scope_kind.as_deref())
    .bind(body.project_id.as_ref().and_then(|value| value.as_deref()))
    .bind(body.status.as_deref())
    .bind(
        body.config_json
            .as_ref()
            .and_then(|value| value.as_ref())
            .map(|value| serde_json::to_string(value).unwrap()),
    )
    .bind(
        body.secret_ciphertext
            .as_ref()
            .and_then(|value| value.as_deref()),
    )
    .bind(&connection_id)
    .bind(&workspace_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let connection = fetch_connection(&state.pool, &workspace_id, &connection_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "connection not found after update"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "integration_connection.updated".to_string(),
            resource_kind: "integration_connection".to_string(),
            resource_id: connection_id,
            payload: None,
        },
    )
    .await;

    Ok(ApiResponse {
        data: connection,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn delete_connection(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, connection_id)): Path<(String, String)>,
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
        "DELETE FROM integration_connections
         WHERE CAST(id AS TEXT) = $1 AND CAST(workspace_id AS TEXT) = $2",
    )
    .bind(&connection_id)
    .bind(&workspace_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "connection not found"));
    }

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "integration_connection.deleted".to_string(),
            resource_kind: "integration_connection".to_string(),
            resource_id: connection_id,
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
        testing::{
            any_test_pool, assert_api_error_code, assert_status_with_body, json_request,
            seed_workspace_member_project,
        },
    };

    const WS_SLUG: &str = "ops-workspace";
    const PROJECT_SLUG: &str = "ops-project";

    async fn setup() -> (axum::Router, String, String) {
        let pool = any_test_pool().await;
        let scope = seed_workspace_member_project(
            &pool,
            WS_SLUG,
            "Ops Workspace",
            PROJECT_SLUG,
            "Ops Project",
            "test:integration-owner",
            "Integration Owner",
            "owner",
        )
        .await;
        let project_id = scope.project_id.clone().unwrap();
        let router = build_router(AppState::new(pool, DatabaseBackend::Sqlite));
        (router, scope.member_id, project_id)
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

    #[tokio::test]
    async fn create_workspace_scoped_connection() {
        let (app, member_id, _) = setup().await;
        let response = app
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/integration-connections"
                )),
                &member_id,
                &json!({
                    "provider": "github",
                    "scope_kind": "workspace",
                    "config_json": {"repo": "agent-workspace"}
                }),
            ))
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::CREATED).await;
        assert_eq!(body["data"]["scope_kind"], "workspace");
    }

    #[tokio::test]
    async fn create_project_scoped_connection() {
        let (app, member_id, project_id) = setup().await;
        let response = app
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/integration-connections"
                )),
                &member_id,
                &json!({
                    "provider": "github",
                    "scope_kind": "project",
                    "project_id": project_id,
                    "config_json": {"repo": "agent-workspace"}
                }),
            ))
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::CREATED).await;
        assert_eq!(body["data"]["scope_kind"], "project");
    }

    #[tokio::test]
    async fn create_rejects_missing_project_id_for_project_scope() {
        let (app, member_id, _) = setup().await;
        let response = app
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/integration-connections"
                )),
                &member_id,
                &json!({
                    "provider": "github",
                    "scope_kind": "project"
                }),
            ))
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::UNPROCESSABLE_ENTITY).await;
        assert_api_error_code(&body, "validation_error");
    }

    #[tokio::test]
    async fn create_rejects_unknown_project() {
        let (app, member_id, _) = setup().await;
        let response = app
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/integration-connections"
                )),
                &member_id,
                &json!({
                    "provider": "github",
                    "scope_kind": "project",
                    "project_id": Uuid::new_v4().to_string()
                }),
            ))
            .await
            .unwrap();
        let body = assert_status_with_body(response, StatusCode::NOT_FOUND).await;
        assert_api_error_code(&body, "not_found");
    }

    #[tokio::test]
    async fn update_and_delete_connection() {
        let (app, member_id, _) = setup().await;
        let created = app
            .clone()
            .oneshot(json_req(
                Request::builder().method("POST").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/integration-connections"
                )),
                &member_id,
                &json!({
                    "provider": "github",
                    "scope_kind": "workspace",
                    "config_json": {"repo": "agent-workspace"}
                }),
            ))
            .await
            .unwrap();
        let created_body = assert_status_with_body(created, StatusCode::CREATED).await;
        let connection_id = created_body["data"]["id"].as_str().unwrap().to_string();

        let updated = app
            .clone()
            .oneshot(json_req(
                Request::builder().method("PATCH").uri(format!(
                    "/api/v1/workspaces/{WS_SLUG}/integration-connections/{connection_id}"
                )),
                &member_id,
                &json!({
                    "status": "disabled",
                    "config_json": {"repo": "agent-workspace", "sync": false}
                }),
            ))
            .await
            .unwrap();
        let updated_body = assert_status_with_body(updated, StatusCode::OK).await;
        assert_eq!(updated_body["data"]["status"], "disabled");

        let deleted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/{WS_SLUG}/integration-connections/{connection_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    }
}
