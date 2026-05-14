use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::db::DatabaseBackend;
use crate::http::{
    access::{require_project_access, WorkspaceRole},
    actor::ActorContext,
    audit::{record_audit, AuditEvent},
    changes::{wait_for_project_change, ChangePollQuery, ChangePollResponse},
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

#[derive(Debug, Serialize)]
pub struct RepairDocumentCyclesResponse {
    pub repaired_document_ids: Vec<String>,
    pub cycle_groups: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MoveDocumentRequest {
    pub target_parent_document_id: Option<String>,
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
            "/workspaces/{workspace_slug}/projects/{project_slug}/documents/changes",
            get(wait_document_changes),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/documents/repair-cycles",
            post(repair_document_cycles),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/documents/{document_id}",
            get(get_document)
                .patch(update_document)
                .delete(delete_document),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/documents/{document_id}/move",
            post(move_document),
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

async fn fetch_project_documents(
    pool: &sqlx::AnyPool,
    project_id: &str,
) -> Result<Vec<DocumentResponse>, sqlx::Error> {
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
         WHERE CAST(project_id AS TEXT) = $1
         ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

fn detect_document_cycle_groups(documents: &[DocumentResponse]) -> Vec<Vec<String>> {
    let parent_by_id: HashMap<String, Option<String>> = documents
        .iter()
        .map(|document| (document.id.clone(), document.parent_document_id.clone()))
        .collect();
    let mut index_by_id = HashMap::<String, usize>::new();
    let mut low_link_by_id = HashMap::<String, usize>::new();
    let mut stack = Vec::<String>::new();
    let mut on_stack = HashSet::<String>::new();
    let mut components = Vec::<Vec<String>>::new();
    let mut index = 0usize;

    fn strong_connect(
        document_id: &str,
        parent_by_id: &HashMap<String, Option<String>>,
        index_by_id: &mut HashMap<String, usize>,
        low_link_by_id: &mut HashMap<String, usize>,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        components: &mut Vec<Vec<String>>,
        index: &mut usize,
    ) {
        index_by_id.insert(document_id.to_string(), *index);
        low_link_by_id.insert(document_id.to_string(), *index);
        *index += 1;
        stack.push(document_id.to_string());
        on_stack.insert(document_id.to_string());

        if let Some(Some(parent_id)) = parent_by_id.get(document_id) {
            if parent_by_id.contains_key(parent_id) {
                if !index_by_id.contains_key(parent_id) {
                    strong_connect(
                        parent_id,
                        parent_by_id,
                        index_by_id,
                        low_link_by_id,
                        stack,
                        on_stack,
                        components,
                        index,
                    );
                    let next_low = (*low_link_by_id.get(document_id).unwrap())
                        .min(*low_link_by_id.get(parent_id).unwrap());
                    low_link_by_id.insert(document_id.to_string(), next_low);
                } else if on_stack.contains(parent_id) {
                    let next_low = (*low_link_by_id.get(document_id).unwrap())
                        .min(*index_by_id.get(parent_id).unwrap());
                    low_link_by_id.insert(document_id.to_string(), next_low);
                }
            }
        }

        if low_link_by_id.get(document_id) != index_by_id.get(document_id) {
            return;
        }

        let mut component = Vec::<String>::new();
        while let Some(current_id) = stack.pop() {
            on_stack.remove(&current_id);
            component.push(current_id.clone());
            if current_id == document_id {
                break;
            }
        }

        let self_loop = component.len() == 1
            && parent_by_id
                .get(&component[0])
                .and_then(|parent_id| parent_id.as_ref())
                .map(|parent_id| parent_id == &component[0])
                .unwrap_or(false);
        if component.len() > 1 || self_loop {
            components.push(component);
        }
    }

    for document in documents {
        if !index_by_id.contains_key(&document.id) {
            strong_connect(
                &document.id,
                &parent_by_id,
                &mut index_by_id,
                &mut low_link_by_id,
                &mut stack,
                &mut on_stack,
                &mut components,
                &mut index,
            );
        }
    }

    components
}

fn choose_cycle_repair_target(cycle_group: &[String], documents: &[DocumentResponse]) -> String {
    let document_by_id: HashMap<&str, &DocumentResponse> = documents
        .iter()
        .map(|document| (document.id.as_str(), document))
        .collect();

    cycle_group
        .iter()
        .max_by(|left_id, right_id| {
            let left = document_by_id.get(left_id.as_str()).copied();
            let right = document_by_id.get(right_id.as_str()).copied();

            left.map(|document| document.updated_at.as_str())
                .cmp(&right.map(|document| document.updated_at.as_str()))
                .then_with(|| left_id.cmp(right_id))
        })
        .cloned()
        .unwrap_or_else(|| cycle_group[0].clone())
}

fn would_create_document_cycle(
    documents: &[DocumentResponse],
    document_id: &str,
    candidate_parent_id: &str,
) -> bool {
    let parent_by_id: HashMap<&str, Option<&str>> = documents
        .iter()
        .map(|document| (document.id.as_str(), document.parent_document_id.as_deref()))
        .collect();
    let mut current_id = Some(candidate_parent_id);
    let mut visited = HashSet::<&str>::new();

    while let Some(parent_id) = current_id {
        if parent_id == document_id {
            return true;
        }
        if !visited.insert(parent_id) {
            return false;
        }
        current_id = parent_by_id.get(parent_id).copied().flatten();
    }

    false
}

fn is_descendant_of(
    documents: &[DocumentResponse],
    ancestor_document_id: &str,
    candidate_descendant_id: &str,
) -> bool {
    let parent_by_id: HashMap<&str, Option<&str>> = documents
        .iter()
        .map(|document| (document.id.as_str(), document.parent_document_id.as_deref()))
        .collect();
    let mut current_id = parent_by_id.get(candidate_descendant_id).copied().flatten();
    let mut visited = HashSet::<&str>::new();

    while let Some(parent_id) = current_id {
        if parent_id == ancestor_document_id {
            return true;
        }
        if !visited.insert(parent_id) {
            return false;
        }
        current_id = parent_by_id.get(parent_id).copied().flatten();
    }

    false
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

    let items = fetch_project_documents(&state.pool, &project_id)
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

async fn wait_document_changes(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    Query(query): Query<ChangePollQuery>,
) -> Result<ApiResponse<ChangePollResponse>, ApiError> {
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

    let data = wait_for_project_change(
        &state,
        query,
        &workspace_id,
        &project_id,
        "documents",
        &request_id,
    )
    .await?;

    Ok(ApiResponse {
        data,
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
        Some("documents:write"),
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
    let insert_document_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO documents
             (id, workspace_id, project_id, parent_document_id, slug, title, body_format, body_md, status, version)
             VALUES (
                CAST($1 AS UUID),
                CAST($2 AS UUID),
                CAST($3 AS UUID),
                CAST($4 AS UUID),
                $5, $6, $7, $8, $9, 1
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO documents
             (id, workspace_id, project_id, parent_document_id, slug, title, body_format, body_md, status, version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)"
        }
    };

    sqlx::query(insert_document_sql)
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

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "document.created".to_string(),
            resource_kind: "document".to_string(),
            resource_id: document_id,
            payload: None,
        },
    )
    .await;

    state
        .change_notifier
        .publish_project_change(&workspace_id, &project_id, "documents");

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
        Some("documents:write"),
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
            let all_documents = fetch_project_documents(&state.pool, &project_id)
                .await
                .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
            if would_create_document_cycle(&all_documents, &document_id, parent_id) {
                return Err(ApiError::validation_error(
                    &request_id,
                    "document cannot become a descendant of itself",
                ));
            }
            Some(parent_id.to_string())
        }
        Some(None) => None,
        None => current.parent_document_id,
    };

    let update_document_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE documents
             SET slug = COALESCE($1, slug),
                 title = COALESCE($2, title),
                 body_md = COALESCE($3, body_md),
                 body_format = COALESCE($4, body_format),
                 status = COALESCE($5, status),
                 parent_document_id = CAST($6 AS UUID),
                 version = version + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS UUID) = CAST($7 AS UUID)
               AND CAST(project_id AS UUID) = CAST($8 AS UUID)
               AND version = $9"
        }
        DatabaseBackend::Sqlite => {
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
               AND version = $9"
        }
    };

    sqlx::query(update_document_sql)
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

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "document.updated".to_string(),
            resource_kind: "document".to_string(),
            resource_id: document_id,
            payload: Some(serde_json::json!({ "version": updated.version })),
        },
    )
    .await;

    state
        .change_notifier
        .publish_project_change(&workspace_id, &project_id, "documents");

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
        Some("documents:write"),
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

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "document.deleted".to_string(),
            resource_kind: "document".to_string(),
            resource_id: document_id,
            payload: None,
        },
    )
    .await;

    state
        .change_notifier
        .publish_project_change(&workspace_id, &project_id, "documents");

    Ok(StatusCode::NO_CONTENT)
}

async fn repair_document_cycles(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
) -> Result<ApiResponse<RepairDocumentCyclesResponse>, ApiError> {
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

    let documents = fetch_project_documents(&state.pool, &project_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let cycle_groups = detect_document_cycle_groups(&documents);
    let repaired_document_ids = cycle_groups
        .iter()
        .map(|cycle_group| choose_cycle_repair_target(cycle_group, &documents))
        .collect::<Vec<_>>();

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let clear_parent_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE documents
             SET parent_document_id = NULL,
                 version = version + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS UUID) = CAST($1 AS UUID)
               AND CAST(project_id AS UUID) = CAST($2 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE documents
             SET parent_document_id = NULL,
                 version = version + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS TEXT) = $1
               AND CAST(project_id AS TEXT) = $2"
        }
    };

    for document_id in &repaired_document_ids {
        sqlx::query(clear_parent_sql)
            .bind(document_id)
            .bind(&project_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
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
            action: "document.cycles_repaired".to_string(),
            resource_kind: "project".to_string(),
            resource_id: project_id.clone(),
            payload: Some(serde_json::json!({
                "repaired_document_ids": repaired_document_ids,
                "cycle_group_count": cycle_groups.len()
            })),
        },
    )
    .await;

    state
        .change_notifier
        .publish_project_change(&workspace_id, &project_id, "documents");

    Ok(ApiResponse {
        data: RepairDocumentCyclesResponse {
            repaired_document_ids,
            cycle_groups,
        },
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

async fn move_document(
    State(state): State<AppState>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Path((workspace_slug, project_slug, document_id)): Path<(String, String, String)>,
    Json(body): Json<MoveDocumentRequest>,
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
        Some("documents:write"),
        &request_id,
    )
    .await?;

    let documents = fetch_project_documents(&state.pool, &project_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
    let current = documents
        .iter()
        .find(|document| document.id == document_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found(&request_id, "document not found"))?;

    let target_parent_document_id = match body.target_parent_document_id.as_deref() {
        Some(parent_id) => {
            if parent_id == document_id {
                return Err(ApiError::validation_error(
                    &request_id,
                    "document cannot be its own parent",
                ));
            }
            let parent_exists = documents.iter().any(|document| document.id == parent_id);
            if !parent_exists {
                return Err(ApiError::not_found(
                    &request_id,
                    "parent document not found in this project",
                ));
            }
            Some(parent_id.to_string())
        }
        None => None,
    };

    if current.parent_document_id == target_parent_document_id {
        return Ok(ApiResponse {
            data: current,
            meta: ResponseMeta {
                request_id,
                audit_event_id: None,
            },
        });
    }

    let moving_into_descendant = target_parent_document_id
        .as_deref()
        .map(|target_parent_id| is_descendant_of(&documents, &document_id, target_parent_id))
        .unwrap_or(false);
    let direct_child_ids = if moving_into_descendant {
        documents
            .iter()
            .filter(|document| document.parent_document_id.as_deref() == Some(document_id.as_str()))
            .map(|document| document.id.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let update_parent_sql = match state.db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE documents
             SET parent_document_id = CAST($1 AS UUID),
                 version = version + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS UUID) = CAST($2 AS UUID)
               AND CAST(project_id AS UUID) = CAST($3 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE documents
             SET parent_document_id = $1,
                 version = version + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS TEXT) = $2
               AND CAST(project_id AS TEXT) = $3"
        }
    };

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    if moving_into_descendant {
        for child_id in &direct_child_ids {
            sqlx::query(update_parent_sql)
                .bind(&current.parent_document_id)
                .bind(child_id)
                .bind(&project_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;
        }
    } else if let Some(ref target_parent_id) = target_parent_document_id {
        if would_create_document_cycle(&documents, &document_id, target_parent_id) {
            return Err(ApiError::validation_error(
                &request_id,
                "document cannot become a descendant of itself",
            ));
        }
    }

    sqlx::query(update_parent_sql)
        .bind(&target_parent_document_id)
        .bind(&document_id)
        .bind(&project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let updated = fetch_document(&state.pool, &project_id, &document_id)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?
        .ok_or_else(|| ApiError::internal(&request_id, "document not found after move"))?;

    let _ = record_audit(
        &state.pool,
        state.db_backend,
        AuditEvent {
            request_id: request_id.clone(),
            actor,
            action: "document.moved".to_string(),
            resource_kind: "document".to_string(),
            resource_id: document_id,
            payload: Some(serde_json::json!({
                "target_parent_document_id": target_parent_document_id,
                "reparented_direct_children": direct_child_ids,
            })),
        },
    )
    .await;

    state
        .change_notifier
        .publish_project_change(&workspace_id, &project_id, "documents");

    Ok(ApiResponse {
        data: updated,
        meta: ResponseMeta {
            request_id,
            audit_event_id: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;
    use tower::ServiceExt;

    use crate::{
        app::build_router,
        db::DatabaseBackend,
        state::AppState,
        testing::{any_test_pool, fixtures},
    };

    const ACTOR_KIND: &str = "x-actor-kind";
    const ACTOR_ID: &str = "x-actor-id";
    const WORKSPACE_ID: &str = "x-workspace-id";
    const PROJECT_ID: &str = "x-project-id";
    const ACTOR_SCOPES: &str = "x-actor-scopes";

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

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        serde_json::from_slice(
            &to_bytes(response.into_body(), 1024 * 1024)
                .await
                .expect("response body"),
        )
        .expect("json response")
    }

    #[tokio::test]
    async fn document_changes_wait_wakes_after_update() {
        let (router, member_id, _workspace_id, _project_id) = setup().await;
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
                            "slug": "long-poll-doc",
                            "title": "Long Poll Doc",
                            "body_md": "# Long Poll"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let created = response_json(create_resp).await;
        let document_id = created["data"]["id"].as_str().unwrap().to_string();

        let initial = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/documents/changes")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let initial_body = response_json(initial).await;
        let cursor = initial_body["data"]["cursor"].as_str().unwrap().to_string();

        let waiter = router.clone().oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/workspaces/dev-workspace/projects/main-project/documents/changes?cursor={cursor}&timeout_ms=30000"
                ))
                .header(ACTOR_KIND, "human")
                .header(ACTOR_ID, &member_id)
                .body(Body::empty())
                .unwrap(),
        );

        sleep(Duration::from_millis(50)).await;

        let update_resp = router
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{document_id}"
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "version": 1,
                            "title": "Long Poll Doc Updated"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);

        let waited = waiter.await.unwrap();
        assert_eq!(waited.status(), StatusCode::OK);
        let waited_body = response_json(waited).await;
        assert_eq!(waited_body["data"]["changed"], true);
    }

    #[tokio::test]
    async fn document_roundtrip_and_version_conflict() {
        let (router, member_id, _workspace_id, _project_id) = setup().await;
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

    #[tokio::test]
    async fn create_get_and_delete_document() {
        let (router, member_id, _workspace_id, _project_id) = setup().await;
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
                            "slug": "ops-runbook",
                            "title": "Ops Runbook",
                            "body_md": "# Runbook"
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

        let get_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{document_id}"
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let delete_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{document_id}"
                    ))
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn create_document_rejects_empty_title() {
        let (router, member_id, _workspace_id, _project_id) = setup().await;
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/documents")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "human")
                    .header(ACTOR_ID, &member_id)
                    .body(Body::from(
                        json!({
                            "slug": "ops-runbook",
                            "title": "   ",
                            "body_md": "# Runbook"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn agent_with_documents_write_can_create_update_move_and_delete_document() {
        let (router, _member_id, workspace_id, project_id) = setup().await;

        let create_parent = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/documents")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "agent")
                    .header(ACTOR_ID, Uuid::new_v4().to_string())
                    .header(WORKSPACE_ID, &workspace_id)
                    .header(PROJECT_ID, &project_id)
                    .header(ACTOR_SCOPES, "documents:write")
                    .body(Body::from(
                        json!({
                            "slug": "parent-doc",
                            "title": "Parent",
                            "body_md": "# Parent"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_parent.status(), StatusCode::CREATED);
        let parent: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(create_parent.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let parent_id = parent["data"]["id"].as_str().unwrap().to_string();

        let create_child = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/documents")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "agent")
                    .header(ACTOR_ID, Uuid::new_v4().to_string())
                    .header(WORKSPACE_ID, &workspace_id)
                    .header(PROJECT_ID, &project_id)
                    .header(ACTOR_SCOPES, "documents:write")
                    .body(Body::from(
                        json!({
                            "slug": "child-doc",
                            "title": "Child",
                            "body_md": "# Child"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_child.status(), StatusCode::CREATED);
        let child: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(create_child.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let child_id = child["data"]["id"].as_str().unwrap().to_string();

        let update_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{child_id}"
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "agent")
                    .header(ACTOR_ID, Uuid::new_v4().to_string())
                    .header(WORKSPACE_ID, &workspace_id)
                    .header(PROJECT_ID, &project_id)
                    .header(ACTOR_SCOPES, "documents:write")
                    .body(Body::from(
                        json!({ "version": 1, "title": "Child v2" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);

        let move_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{child_id}/move"
                    ))
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "agent")
                    .header(ACTOR_ID, Uuid::new_v4().to_string())
                    .header(WORKSPACE_ID, &workspace_id)
                    .header(PROJECT_ID, &project_id)
                    .header(ACTOR_SCOPES, "documents:write")
                    .body(Body::from(
                        json!({ "target_parent_document_id": parent_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(move_resp.status(), StatusCode::OK);

        let delete_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/dev-workspace/projects/main-project/documents/{child_id}"
                    ))
                    .header(ACTOR_KIND, "agent")
                    .header(ACTOR_ID, Uuid::new_v4().to_string())
                    .header(WORKSPACE_ID, &workspace_id)
                    .header(PROJECT_ID, &project_id)
                    .header(ACTOR_SCOPES, "documents:write")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn agent_with_documents_read_cannot_mutate_documents() {
        let (router, _member_id, workspace_id, project_id) = setup().await;
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/dev-workspace/projects/main-project/documents")
                    .header("content-type", "application/json")
                    .header(ACTOR_KIND, "agent")
                    .header(ACTOR_ID, Uuid::new_v4().to_string())
                    .header(WORKSPACE_ID, &workspace_id)
                    .header(PROJECT_ID, &project_id)
                    .header(ACTOR_SCOPES, "documents:read")
                    .body(Body::from(
                        json!({
                            "slug": "forbidden-doc",
                            "title": "Forbidden",
                            "body_md": "# Forbidden"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
