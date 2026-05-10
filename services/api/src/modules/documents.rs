use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::http::{
    access::{require_project_access, WorkspaceRole},
    actor::ActorContext,
    audit::{emit_audit, AuditEvent},
    error::ApiError,
    request_id::RequestId,
    response::{ApiResponse, Created, ListData, ResponseMeta},
};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DocumentResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub parent_document_id: Option<String>,
    pub slug: String,
    pub title: String,
    pub body_format: String,
    pub body_md: String,
    pub status: String,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub slug: String,
    pub title: String,
    pub body_md: String,
    pub parent_document_id: Option<String>,
    #[serde(default = "default_body_format")]
    pub body_format: String,
    #[serde(default = "default_status")]
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDocumentRequest {
    pub version: i32,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub body_md: Option<String>,
    pub parent_document_id: Option<Option<String>>,
    pub body_format: Option<String>,
    pub status: Option<String>,
}

fn default_body_format() -> String {
    "markdown".to_string()
}

fn default_status() -> String {
    "draft".to_string()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/documents",
            get(list_documents).post(create_document),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/documents/{document_id}",
            get(get_document)
                .patch(update_document)
                .delete(delete_document),
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

fn validate_body_format(body_format: &str, request_id: &str) -> Result<(), ApiError> {
    if body_format == "markdown" {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "body_format must be markdown",
        ))
    }
}

fn validate_status(status: &str, request_id: &str) -> Result<(), ApiError> {
    if matches!(status, "draft" | "published" | "archived") {
        Ok(())
    } else {
        Err(ApiError::validation_error(
            request_id,
            "status must be one of: draft, published, archived",
        ))
    }
}

async fn fetch_document(
    pool: &sqlx::AnyPool,
    project_id: &str,
    document_id: &str,
) -> Result<Option<DocumentResponse>, sqlx::Error> {
    sqlx::query_as::<_, DocumentResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                CAST(parent_document_id AS TEXT) AS parent_document_id,
                slug,
                title,
                body_format,
                body_md,
                status,
                version,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM documents
         WHERE CAST(project_id AS TEXT) = $1 AND CAST(id AS TEXT) = $2",
    )
    .bind(project_id)
    .bind(document_id)
    .fetch_optional(pool)
    .await
}

async fn list_documents(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<ListData<DocumentResponse>>, ApiError> {
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
        Some("documents:read"),
        &request_id,
    )
    .await?;

    let items = sqlx::query_as::<_, DocumentResponse>(
        "SELECT CAST(id AS TEXT) AS id,
                CAST(workspace_id AS TEXT) AS workspace_id,
                CAST(project_id AS TEXT) AS project_id,
                CAST(parent_document_id AS TEXT) AS parent_document_id,
                slug,
                title,
                body_format,
                body_md,
                status,
                version,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM documents
         WHERE CAST(project_id AS TEXT) = $1
         ORDER BY created_at DESC",
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

async fn create_document(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    Json(body): Json<CreateDocumentRequest>,
) -> Result<Created<DocumentResponse>, ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::validation_error(
            &request_id,
            "title must not be empty",
        ));
    }
    validate_slug(&body.slug, &request_id)?;
    validate_body_format(&body.body_format, &request_id)?;
    validate_status(&body.status, &request_id)?;

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
        None,
        &request_id,
    )
    .await?;

    let parent_document_id = if let Some(parent_id) = body.parent_document_id.as_deref() {
        let parent = fetch_document(&state.pool, &project_id, parent_id)
            .await
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
        if parent.is_none() {
            return Err(ApiError::not_found(
                &request_id,
                "parent document not found in this project",
            ));
        }
        Some(parent_id.to_string())
    } else {
        None
    };

    let document_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO documents
         (id, workspace_id, project_id, parent_document_id, slug, title, body_format, body_md, status, version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)",
    )
    .bind(&document_id)
    .bind(&workspace_id)
    .bind(&project_id)
    .bind(&parent_document_id)
    .bind(&body.slug)
    .bind(body.title.trim())
    .bind(&body.body_format)
    .bind(&body.body_md)
    .bind(&body.status)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let document = fetch_document(&state.pool, &project_id, &document_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "document not found after insert"))?;

    emit_audit(AuditEvent {
        request_id: request_id.clone(),
        actor,
        action: "document.created".to_string(),
        resource_kind: "document".to_string(),
        resource_id: document_id,
        payload: None,
    });

    Ok(Created(ApiResponse {
        data: document,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    }))
}

async fn get_document(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, document_id)): Path<(String, String, String)>,
) -> Result<ApiResponse<DocumentResponse>, ApiError> {
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
        Some("documents:read"),
        &request_id,
    )
    .await?;

    let document = fetch_document(&state.pool, &project_id, &document_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "document not found"))?;

    Ok(ApiResponse {
        data: document,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn update_document(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, document_id)): Path<(String, String, String)>,
    Json(body): Json<UpdateDocumentRequest>,
) -> Result<ApiResponse<DocumentResponse>, ApiError> {
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
        None,
        &request_id,
    )
    .await?;

    if let Some(ref slug) = body.slug {
        validate_slug(slug, &request_id)?;
    }
    if let Some(ref title) = body.title {
        if title.trim().is_empty() {
            return Err(ApiError::validation_error(
                &request_id,
                "title must not be empty",
            ));
        }
    }
    if let Some(ref body_format) = body.body_format {
        validate_body_format(body_format, &request_id)?;
    }
    if let Some(ref status) = body.status {
        validate_status(status, &request_id)?;
    }

    let current = fetch_document(&state.pool, &project_id, &document_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::not_found(&request_id, "document not found"))?;

    if current.version != body.version {
        return Err(ApiError::conflict(
            &request_id,
            "document version is stale; reload before updating",
        ));
    }

    let parent_document_id = match body.parent_document_id {
        Some(Some(ref parent_id)) => {
            if parent_id == &document_id {
                return Err(ApiError::validation_error(
                    &request_id,
                    "document cannot be its own parent",
                ));
            }
            let parent = fetch_document(&state.pool, &project_id, parent_id)
                .await
                .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
            if parent.is_none() {
                return Err(ApiError::not_found(
                    &request_id,
                    "parent document not found in this project",
                ));
            }
            Some(parent_id.to_string())
        }
        Some(None) => None,
        None => current.parent_document_id,
    };

    sqlx::query(
        "UPDATE documents
         SET slug = COALESCE($1, slug),
             title = COALESCE($2, title),
             body_md = COALESCE($3, body_md),
             body_format = COALESCE($4, body_format),
             status = COALESCE($5, status),
             parent_document_id = $6,
             version = version + 1,
             updated_at = CURRENT_TIMESTAMP
         WHERE CAST(id AS TEXT) = $7
           AND CAST(project_id AS TEXT) = $8
           AND version = $9",
    )
    .bind(body.slug.as_deref())
    .bind(body.title.as_deref().map(str::trim))
    .bind(body.body_md.as_deref())
    .bind(body.body_format.as_deref())
    .bind(body.status.as_deref())
    .bind(&parent_document_id)
    .bind(&document_id)
    .bind(&project_id)
    .bind(body.version)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let updated = fetch_document(&state.pool, &project_id, &document_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "document not found after update"))?;

    emit_audit(AuditEvent {
        request_id: request_id.clone(),
        actor,
        action: "document.updated".to_string(),
        resource_kind: "document".to_string(),
        resource_id: document_id,
        payload: Some(serde_json::json!({ "version": updated.version })),
    });

    Ok(ApiResponse {
        data: updated,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn delete_document(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, document_id)): Path<(String, String, String)>,
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
        None,
        &request_id,
    )
    .await?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    sqlx::query(
        "UPDATE documents SET parent_document_id = NULL
         WHERE CAST(parent_document_id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&document_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let affected = sqlx::query(
        "DELETE FROM documents
         WHERE CAST(id AS TEXT) = $1 AND CAST(project_id AS TEXT) = $2",
    )
    .bind(&document_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found(&request_id, "document not found"));
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    emit_audit(AuditEvent {
        request_id: request_id.clone(),
        actor,
        action: "document.deleted".to_string(),
        resource_kind: "document".to_string(),
        resource_id: document_id,
        payload: None,
    });

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
        state::AppState,
        testing::{any_test_pool, fixtures},
    };

    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";

    async fn setup() -> (axum::Router, String) {
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
        )
    }

    #[tokio::test]
    async fn document_roundtrip_and_version_conflict() {
        let (router, member_id) = setup().await;
        let create_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/documents")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "slug": "api-contract",
                            "title": "API Contract",
                            "body_md": "# Hello"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let created: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(create_resp.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let document_id = created["data"]["id"].as_str().unwrap().to_string();

        let patch_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{document_id}"
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(json!({
                        "version": 1,
                        "title": "API Contract v2",
                        "body_md": "# Updated"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_resp.status(), StatusCode::OK);

        let conflict_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{document_id}"
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(json!({
                        "version": 1,
                        "title": "Stale"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(conflict_resp.status(), StatusCode::CONFLICT);
    }
}
