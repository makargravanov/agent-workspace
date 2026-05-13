use std::{fs, path::PathBuf};

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    db::DatabaseBackend,
    http::{
        access::{require_project_access, WorkspaceRole},
        actor::{ActorContext, ActorKind},
        audit::{record_audit, AuditEvent},
        error::ApiError,
        request_id::RequestId,
        response::{ApiResponse, Created, ListData, ResponseMeta},
    },
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AssetResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub uploaded_by_member_id: Option<String>,
    pub uploaded_by_github_login: Option<String>,
    pub file_name: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub sha256: Option<String>,
    pub storage_backend: String,
    pub storage_key: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub file_name: String,
    pub media_type: String,
    pub content_base64: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAssetRequest {
    pub file_name: Option<String>,
    pub media_type: Option<String>,
    pub content_base64: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DownloadAssetQuery {
    pub disposition: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/assets",
            get(list_assets).post(create_asset),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/assets/{asset_id}",
            get(get_asset).patch(update_asset).delete(delete_asset),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/assets/{asset_id}/download",
            get(download_asset),
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

fn storage_root(state: &AppState) -> PathBuf {
    state.asset_storage_dir.clone()
}

fn asset_path(state: &AppState, storage_key: &str) -> PathBuf {
    storage_root(state).join(storage_key)
}

fn validate_non_empty(value: &str, field: &str, request_id: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        Err(ApiError::validation_error(
            request_id,
            format!("{field} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

async fn fetch_asset(
    pool: &sqlx::AnyPool,
    project_id: &str,
    asset_id: &str,
) -> Result<Option<AssetResponse>, sqlx::Error> {
    sqlx::query_as::<_, AssetResponse>(
        "SELECT CAST(a.id AS TEXT) AS id,
                CAST(a.workspace_id AS TEXT) AS workspace_id,
                CAST(a.project_id AS TEXT) AS project_id,
                CAST(a.uploaded_by_member_id AS TEXT) AS uploaded_by_member_id,
                (
                    SELECT hi.display_name
                    FROM human_identities hi
                    WHERE hi.workspace_member_id = a.uploaded_by_member_id
                      AND hi.provider = 'github'
                    ORDER BY hi.updated_at DESC
                    LIMIT 1
                ) AS uploaded_by_github_login,
                a.file_name,
                a.media_type,
                a.size_bytes,
                a.sha256,
                a.storage_backend,
                a.storage_key,
                CAST(a.created_at AS TEXT) AS created_at
         FROM assets a
         WHERE CAST(a.project_id AS TEXT) = $1 AND CAST(a.id AS TEXT) = $2",
    )
    .bind(project_id)
    .bind(asset_id)
    .fetch_optional(pool)
    .await
}

async fn list_assets(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<ListData<AssetResponse>>, ApiError> {
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
        Some("assets:read"),
        &request_id,
    )
    .await?;

    let items = sqlx::query_as::<_, AssetResponse>(
        "SELECT CAST(a.id AS TEXT) AS id,
                CAST(a.workspace_id AS TEXT) AS workspace_id,
                CAST(a.project_id AS TEXT) AS project_id,
                CAST(a.uploaded_by_member_id AS TEXT) AS uploaded_by_member_id,
                (
                    SELECT hi.display_name
                    FROM human_identities hi
                    WHERE hi.workspace_member_id = a.uploaded_by_member_id
                      AND hi.provider = 'github'
                    ORDER BY hi.updated_at DESC
                    LIMIT 1
                ) AS uploaded_by_github_login,
                a.file_name,
                a.media_type,
                a.size_bytes,
                a.sha256,
                a.storage_backend,
                a.storage_key,
                CAST(a.created_at AS TEXT) AS created_at
         FROM assets a
         WHERE CAST(a.project_id AS TEXT) = $1
         ORDER BY a.created_at DESC",
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

async fn create_asset(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    Json(body): Json<CreateAssetRequest>,
) -> Result<Created<AssetResponse>, ApiError> {
    validate_non_empty(&body.file_name, "file_name", &request_id)?;
    validate_non_empty(&body.media_type, "media_type", &request_id)?;

    let content = STANDARD
        .decode(body.content_base64.as_bytes())
        .map_err(|_| {
            ApiError::validation_error(&request_id, "content_base64 must be valid base64")
        })?;

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
        Some("assets:write"),
        &request_id,
    )
    .await?;

    let asset_id = Uuid::new_v4().to_string();
    let storage_key = asset_id.clone();
    let uploaded_by_member_id = if actor.actor_kind == ActorKind::Human {
        Some(actor.actor_id.clone())
    } else {
        None
    };
    let digest = body.sha256.clone().or_else(|| {
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Some(hex::encode(hasher.finalize()))
    });

    let insert_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO assets
             (id, workspace_id, project_id, uploaded_by_member_id, file_name, media_type, size_bytes, sha256, storage_backend, storage_key)
             VALUES (
                CAST($1 AS UUID),
                CAST($2 AS UUID),
                CAST($3 AS UUID),
                CAST($4 AS UUID),
                $5, $6, $7, $8, 'local', $9
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO assets
             (id, workspace_id, project_id, uploaded_by_member_id, file_name, media_type, size_bytes, sha256, storage_backend, storage_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'local', $9)"
        }
    };

    sqlx::query(insert_sql)
        .bind(&asset_id)
        .bind(&workspace_id)
        .bind(&project_id)
        .bind(uploaded_by_member_id.as_deref())
        .bind(&body.file_name)
        .bind(&body.media_type)
        .bind(content.len() as i64)
        .bind(digest.as_deref())
        .bind(&storage_key)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    fs::create_dir_all(storage_root(&state))
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    fs::write(asset_path(&state, &storage_key), &content)
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let asset = fetch_asset(&state.pool, &project_id, &asset_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "asset not found after insert"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "asset.created".to_string(),
            resource_kind: "asset".to_string(),
            resource_id: asset_id,
            payload: None,
        },
    )
    .await;

    Ok(Created(ApiResponse {
        data: asset,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn get_asset(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, asset_id)): Path<(String, String, String)>,
) -> Result<ApiResponse<AssetResponse>, ApiError> {
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
        Some("assets:read"),
        &request_id,
    )
    .await?;

    let asset = fetch_asset(&state.pool, &project_id, &asset_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "asset not found"))?;

    Ok(ApiResponse {
        data: asset,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn update_asset(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, asset_id)): Path<(String, String, String)>,
    Json(body): Json<UpdateAssetRequest>,
) -> Result<ApiResponse<AssetResponse>, ApiError> {
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
        Some("assets:write"),
        &request_id,
    )
    .await?;

    if let Some(ref file_name) = body.file_name {
        validate_non_empty(file_name, "file_name", &request_id)?;
    }
    if let Some(ref media_type) = body.media_type {
        validate_non_empty(media_type, "media_type", &request_id)?;
    }

    let current = fetch_asset(&state.pool, &project_id, &asset_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "asset not found"))?;

    let content = if let Some(ref encoded) = body.content_base64 {
        Some(STANDARD.decode(encoded.as_bytes()).map_err(|_| {
            ApiError::validation_error(&request_id, "content_base64 must be valid base64")
        })?)
    } else {
        None
    };
    let sha256 = body.sha256.clone().or_else(|| {
        content.as_ref().map(|bytes| {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            hex::encode(hasher.finalize())
        })
    });

    let update_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE assets
             SET file_name = COALESCE($1, file_name),
                 media_type = COALESCE($2, media_type),
                 size_bytes = COALESCE($3, size_bytes),
                 sha256 = COALESCE($4, sha256)
             WHERE CAST(id AS UUID) = CAST($5 AS UUID)
               AND CAST(project_id AS UUID) = CAST($6 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE assets
             SET file_name = COALESCE($1, file_name),
                 media_type = COALESCE($2, media_type),
                 size_bytes = COALESCE($3, size_bytes),
                 sha256 = COALESCE($4, sha256)
             WHERE CAST(id AS TEXT) = $5
               AND CAST(project_id AS TEXT) = $6"
        }
    };

    let size_bytes = content.as_ref().map(|bytes| bytes.len() as i64);
    sqlx::query(update_sql)
        .bind(body.file_name.as_deref())
        .bind(body.media_type.as_deref())
        .bind(size_bytes)
        .bind(sha256.as_deref())
        .bind(&asset_id)
        .bind(&project_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    if let Some(bytes) = content {
        fs::write(asset_path(&state, &current.storage_key), bytes)
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    }

    let asset = fetch_asset(&state.pool, &project_id, &asset_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "asset not found after update"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "asset.updated".to_string(),
            resource_kind: "asset".to_string(),
            resource_id: asset_id,
            payload: None,
        },
    )
    .await;

    Ok(ApiResponse {
        data: asset,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn delete_asset(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, asset_id)): Path<(String, String, String)>,
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
        Some("assets:write"),
        &request_id,
    )
    .await?;

    let current = fetch_asset(&state.pool, &project_id, &asset_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "asset not found"))?;

    let affected = sqlx::query(
        "DELETE FROM assets
         WHERE CAST(id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&asset_id)
    .bind(&project_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "asset not found"));
    }

    let _ = fs::remove_file(asset_path(&state, &current.storage_key));

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "asset.deleted".to_string(),
            resource_kind: "asset".to_string(),
            resource_id: asset_id,
            payload: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn download_asset(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, asset_id)): Path<(String, String, String)>,
    Query(query): Query<DownloadAssetQuery>,
) -> Result<Response, ApiError> {
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
        Some("assets:read"),
        &request_id,
    )
    .await?;

    let asset = fetch_asset(&state.pool, &project_id, &asset_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "asset not found"))?;

    let bytes = fs::read(asset_path(&state, &asset.storage_key))
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let disposition = if query.disposition.as_deref() == Some("inline") {
        "inline"
    } else {
        "attachment"
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, asset.media_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("{disposition}; filename=\"{}\"", asset.file_name),
        )
        .body(axum::body::Body::from(bytes))
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    use crate::{
        app::build_router,
        db::DatabaseBackend,
        state::AppState,
        testing::{any_test_pool, fixtures},
    };

    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";

    async fn setup() -> (axum::Router, PathBuf, String, String, String) {
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

        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&project_id)
        .bind(&workspace_id)
        .bind(fixtures::PROJECT_SLUG)
        .bind(fixtures::PROJECT_NAME)
        .bind("active")
        .execute(&pool)
        .await
        .unwrap();

        let storage_dir = std::env::temp_dir().join(format!(
            "agent-workspace-assets-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let router = build_router(AppState::new_with_asset_storage(
            pool,
            DatabaseBackend::Sqlite,
            storage_dir.clone(),
        ));
        (router, storage_dir, workspace_id, project_id, member_id)
    }

    fn request(builder: axum::http::request::Builder, body: Body) -> Request<Body> {
        builder.body(body).unwrap()
    }

    #[tokio::test]
    async fn create_list_download_and_delete_asset() {
        let (app, storage_dir, _, _, member_id) = setup().await;
        let payload = json!({
            "file_name": "note.txt",
            "media_type": "text/plain",
            "content_base64": STANDARD.encode("hello assets"),
        });

        let response = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::from(serde_json::to_vec(&payload).unwrap()),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let asset_id = created["data"]["id"].as_str().unwrap().to_string();

        let list = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::empty(),
            ))
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);

        let download = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets/{}/download",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG,
                        asset_id
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::empty(),
            ))
            .await
            .unwrap();
        assert_eq!(download.status(), StatusCode::OK);
        let downloaded = to_bytes(download.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&downloaded[..], b"hello assets");

        let delete = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets/{}",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG,
                        asset_id
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::empty(),
            ))
            .await
            .unwrap();
        assert_eq!(delete.status(), StatusCode::NO_CONTENT);

        assert!(!storage_dir.join(&asset_id).exists());
    }

    #[tokio::test]
    async fn create_asset_rejects_invalid_base64() {
        let (app, _storage_dir, _, _, member_id) = setup().await;
        let payload = json!({
            "file_name": "note.txt",
            "media_type": "text/plain",
            "content_base64": "not base64!!!",
        });

        let response = app
            .oneshot(request(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::from(serde_json::to_vec(&payload).unwrap()),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn update_asset_changes_metadata_and_content() {
        let (app, _storage_dir, _, _, member_id) = setup().await;
        let created = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::from(
                    json!({
                        "file_name": "note.txt",
                        "media_type": "text/plain",
                        "content_base64": STANDARD.encode("hello assets"),
                    })
                    .to_string(),
                ),
            ))
            .await
            .unwrap();
        let created: serde_json::Value =
            serde_json::from_slice(&to_bytes(created.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let asset_id = created["data"]["id"].as_str().unwrap().to_string();

        let updated = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets/{}",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG,
                        asset_id
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::from(
                    json!({
                        "file_name": "note-v2.txt",
                        "content_base64": STANDARD.encode("hello assets v2"),
                    })
                    .to_string(),
                ),
            ))
            .await
            .unwrap();
        assert_eq!(updated.status(), StatusCode::OK);

        let fetched = app
            .clone()
            .oneshot(request(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/{}/projects/{}/assets/{}",
                        fixtures::WORKSPACE_SLUG,
                        fixtures::PROJECT_SLUG,
                        asset_id
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, member_id.clone()),
                Body::empty(),
            ))
            .await
            .unwrap();
        let fetched: serde_json::Value =
            serde_json::from_slice(&to_bytes(fetched.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(fetched["data"]["file_name"], "note-v2.txt");
    }
}
