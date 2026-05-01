use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::http::{
    access::{require_project_access, WorkspaceRole},
    actor::{ActorContext, ActorKind},
    audit::{emit_audit, AuditEvent},
    error::ApiError,
    pagination::PaginationParams,
    request_id::RequestId,
    response::{ApiResponse, Created, ListData, ResponseMeta},
};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    Context,
    Worklog,
    Decision,
    Result,
}
impl NoteKind {
    fn as_str(&self) -> &'static str {
        match self {
            NoteKind::Context => "context",
            NoteKind::Worklog => "worklog",
            NoteKind::Decision => "decision",
            NoteKind::Result => "result",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "context" => Some(NoteKind::Context),
            "worklog" => Some(NoteKind::Worklog),
            "decision" => Some(NoteKind::Decision),
            "result" => Some(NoteKind::Result),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorType {
    WorkspaceMember,
    Agent,
    Integration,
}

impl AuthorType {
    fn as_str(&self) -> &'static str {
        match self {
            AuthorType::WorkspaceMember => "workspace_member",
            AuthorType::Agent => "agent",
            AuthorType::Integration => "integration",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "workspace_member" => Some(AuthorType::WorkspaceMember),
            "agent" => Some(AuthorType::Agent),
            "integration" => Some(AuthorType::Integration),
            _ => None,
        }
    }
}

#[derive(Debug, FromRow)]
struct NoteRow {
    id: String,
    project_id: String,
    agent_session_id: Option<String>,
    kind: String,
    author_type: String,
    author_id: String,
    title: Option<String>,
    body_md: String,
    created_at: String,
    updated_at: String,
}

impl NoteRow {
    fn into_response(self) -> Result<NoteResponse, String> {
        let kind = NoteKind::from_str(&self.kind)
            .ok_or_else(|| format!("invalid note kind stored in db: {}", self.kind))?;
        let author_type = AuthorType::from_str(&self.author_type)
            .ok_or_else(|| format!("invalid author_type stored in db: {}", self.author_type))?;

        Ok(NoteResponse {
            id: self.id,
            project_id: self.project_id,
            agent_session_id: self.agent_session_id,
            kind,
            author_type,
            author_id: self.author_id,
            title: self.title,
            body_md: self.body_md,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteResponse {
    pub id: String,
    pub project_id: String,
    pub agent_session_id: Option<String>,
    pub kind: NoteKind,
    pub author_type: AuthorType,
    pub author_id: String,
    pub title: Option<String>,
    pub body_md: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotePayload {
    pub kind: NoteKind,
    pub title: Option<String>,
    pub body_md: String,
    pub agent_session_id: Option<String>,
}

async fn resolve_project(
    pool: &sqlx::AnyPool,
    workspace_slug: &str,
    project_slug: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT CAST(p.workspace_id AS TEXT), CAST(p.id AS TEXT) \
         FROM projects p \
         JOIN workspaces w ON w.id = p.workspace_id \
         WHERE w.slug = $1 AND p.slug = $2",
    )
    .bind(workspace_slug)
    .bind(project_slug)
    .fetch_optional(pool)
    .await
}

async fn list_notes(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    RequestId(request_id): RequestId,
    pagination: PaginationParams,
    actor: ActorContext,
) -> Result<ApiResponse<ListData<NoteResponse>>, ApiError> {
    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let (workspace_id, project_id) = ids
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Viewer,
        Some("notes:read"),
        &request_id,
    )
    .await?;

    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notes WHERE project_id = $1")
        .bind(&project_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let offset = ((pagination.page - 1) * pagination.per_page) as i64;
    let limit = pagination.per_page as i64;

    let rows: Vec<NoteRow> = sqlx::query_as(
        "SELECT CAST(id AS TEXT) AS id, CAST(project_id AS TEXT) AS project_id, \
            CAST(agent_session_id AS TEXT) AS agent_session_id, \
            kind, author_type, CAST(author_id AS TEXT) AS author_id, title, body_md, \
            CAST(created_at AS TEXT) AS created_at, CAST(updated_at AS TEXT) AS updated_at \
         FROM notes \
         WHERE project_id = $1 \
         ORDER BY created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(&project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let items: Vec<NoteResponse> = rows
        .into_iter()
        .map(|row| row.into_response())
        .collect::<Result<_, _>>()
        .map_err(|e| ApiError::internal(&request_id, e))?;

    let total_pages = if pagination.per_page == 0 {
        0u32
    } else {
        ((total as f64) / (pagination.per_page as f64)).ceil() as u32
    };
    let next_cursor = if pagination.page < total_pages {
        Some((pagination.page + 1).to_string())
    } else {
        None
    };

    Ok(ApiResponse {
        data: ListData { items, next_cursor },
        meta: ResponseMeta { request_id, audit_event_id: None },
    })
}

async fn create_note(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug)): Path<(String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
    Json(payload): Json<CreateNotePayload>,
) -> Result<Created<NoteResponse>, ApiError> {
    if payload.body_md.trim().is_empty() {
        return Err(ApiError::validation_error(&request_id, "body_md must not be empty"));
    }

    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let (workspace_id, project_id) = ids
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Editor,
        Some("notes:write"),
        &request_id,
    )
    .await?;

    let note_id = Uuid::new_v4().to_string();
    let author_type = match actor.actor_kind {
        ActorKind::Agent => AuthorType::Agent,
        _ => AuthorType::WorkspaceMember,
    };

    sqlx::query(
        "INSERT INTO notes \
         (id, workspace_id, project_id, agent_session_id, kind, author_type, author_id, title, body_md) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&note_id)
    .bind(&workspace_id)
    .bind(&project_id)
    .bind(&payload.agent_session_id)
    .bind(payload.kind.as_str())
    .bind(author_type.as_str())
    .bind(&actor.actor_id)
    .bind(&payload.title)
    .bind(&payload.body_md)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let row: NoteRow = sqlx::query_as(
        "SELECT CAST(id AS TEXT) AS id, CAST(project_id AS TEXT) AS project_id, \
            CAST(agent_session_id AS TEXT) AS agent_session_id, \
            kind, author_type, CAST(author_id AS TEXT) AS author_id, title, body_md, \
            CAST(created_at AS TEXT) AS created_at, CAST(updated_at AS TEXT) AS updated_at \
         FROM notes WHERE id = $1",
    )
    .bind(&note_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let note = row
        .into_response()
        .map_err(|e| ApiError::internal(&request_id, e))?;

    emit_audit(AuditEvent {
        request_id: request_id.clone(),
        actor,
        action: "note.created".to_string(),
        resource_kind: "note".to_string(),
        resource_id: note_id,
        payload: None,
    });

    Ok(Created(ApiResponse {
        data: note,
        meta: ResponseMeta { request_id, audit_event_id: None },
    }))
}

async fn get_note(
    State(state): State<AppState>,
    Path((workspace_slug, project_slug, note_id)): Path<(String, String, String)>,
    RequestId(request_id): RequestId,
    actor: ActorContext,
) -> Result<ApiResponse<NoteResponse>, ApiError> {
    let ids = resolve_project(&state.pool, &workspace_slug, &project_slug)
        .await
        .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let (workspace_id, project_id) = ids
        .ok_or_else(|| ApiError::not_found(&request_id, "workspace or project not found"))?;

    require_project_access(
        &state.pool,
        &actor,
        &workspace_id,
        &project_id,
        WorkspaceRole::Viewer,
        Some("notes:read"),
        &request_id,
    )
    .await?;

    let row: Option<NoteRow> = sqlx::query_as(
        "SELECT CAST(id AS TEXT) AS id, CAST(project_id AS TEXT) AS project_id, \
            CAST(agent_session_id AS TEXT) AS agent_session_id, \
            kind, author_type, CAST(author_id AS TEXT) AS author_id, title, body_md, \
            CAST(created_at AS TEXT) AS created_at, CAST(updated_at AS TEXT) AS updated_at \
         FROM notes \
         WHERE id = $1 AND project_id = $2",
    )
    .bind(&note_id)
    .bind(&project_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::internal(&request_id, e.to_string()))?;

    let note = row
        .ok_or_else(|| ApiError::not_found(&request_id, "note not found"))?
        .into_response()
        .map_err(|e| ApiError::internal(&request_id, e))?;

    Ok(ApiResponse {
        data: note,
        meta: ResponseMeta { request_id, audit_event_id: None },
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/notes",
            get(list_notes).post(create_note),
        )
        .route(
            "/workspaces/{workspace_slug}/projects/{project_slug}/notes/{note_id}",
            get(get_note),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::app::build_router;
    use crate::testing::{any_test_pool, fixtures};

    struct TestContext {
        router: axum::Router,
        member_id: String,
        workspace_id: String,
        project_id: String,
    }

    async fn setup() -> TestContext {
        let pool = any_test_pool().await;

        let ws_id = Uuid::new_v4().to_string();
        let member_id = Uuid::new_v4().to_string();
        let project_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)")
            .bind(&ws_id)
            .bind(fixtures::WORKSPACE_SLUG)
            .bind(fixtures::WORKSPACE_NAME)
            .execute(&pool)
            .await
            .expect("insert workspace");

        sqlx::query(
            "INSERT INTO workspace_members \
             (id, workspace_id, external_subject, display_name, role, status) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&member_id)
        .bind(&ws_id)
        .bind("test:member-1")
        .bind("Test Member")
        .bind("owner")
        .bind("active")
        .execute(&pool)
        .await
        .expect("insert member");

        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, status) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&project_id)
        .bind(&ws_id)
        .bind(fixtures::PROJECT_SLUG)
        .bind(fixtures::PROJECT_NAME)
        .bind("active")
        .execute(&pool)
        .await
        .expect("insert project");

        TestContext {
            router: build_router(AppState::new(pool)),
            member_id,
            workspace_id: ws_id,
            project_id,
        }
    }

    fn notes_url() -> String {
        format!(
            "/api/v1/workspaces/{}/projects/{}/notes",
            fixtures::WORKSPACE_SLUG,
            fixtures::PROJECT_SLUG,
        )
    }

    fn note_url(note_id: &str) -> String {
        format!(
            "/api/v1/workspaces/{}/projects/{}/notes/{}",
            fixtures::WORKSPACE_SLUG,
            fixtures::PROJECT_SLUG,
            note_id,
        )
    }

    async fn body_json(body: axum::body::Body) -> Value {
        let bytes = axum::body::to_bytes(body, 1024 * 1024)
            .await
            .expect("body should be readable");
        serde_json::from_slice(&bytes).expect("body should be valid JSON")
    }

    #[tokio::test]
    async fn list_notes_returns_empty_for_fresh_project() {
        let ctx = setup().await;
        let resp = ctx.router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(notes_url())
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &ctx.member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_note_returns_201_and_note_is_listable() {
        let ctx = setup().await;

        let payload = serde_json::json!({
            "kind": "decision",
            "title": "Use AnyPool",
            "body_md": "We decided to use sqlx AnyPool.",
            "agent_session_id": null
        });

        let create_resp = ctx.router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(notes_url())
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &ctx.member_id)
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let create_body = body_json(create_resp.into_body()).await;
        let note_id = create_body["data"]["id"].as_str().unwrap().to_string();
        assert!(!note_id.is_empty());
        assert_eq!(create_body["data"]["kind"].as_str().unwrap(), "decision");
        assert_eq!(
            create_body["data"]["author_type"].as_str().unwrap(),
            "workspace_member"
        );

        let list_resp = ctx.router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(notes_url())
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &ctx.member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list_body = body_json(list_resp.into_body()).await;
        assert_eq!(list_body["data"]["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_note_returns_200_with_correct_author_type() {
        let ctx = setup().await;

        let payload = serde_json::json!({
            "kind": "worklog",
            "title": null,
            "body_md": "Checked the pipeline.",
            "agent_session_id": null
        });

        let create_resp = ctx.router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(notes_url())
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "agent")
                    .header("x-actor-id", "agent-001")
                    .header("x-workspace-id", &ctx.workspace_id)
                    .header("x-project-id", &ctx.project_id)
                    .header("x-actor-scopes", "notes:write,notes:read")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let note_id = body_json(create_resp.into_body()).await["data"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let get_resp = ctx.router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(note_url(&note_id))
                    .header("x-actor-kind", "agent")
                    .header("x-actor-id", "agent-001")
                    .header("x-workspace-id", &ctx.workspace_id)
                    .header("x-project-id", &ctx.project_id)
                    .header("x-actor-scopes", "notes:read")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        let get_body = body_json(get_resp.into_body()).await;
        assert_eq!(get_body["data"]["id"].as_str().unwrap(), note_id);
        assert_eq!(get_body["data"]["author_type"].as_str().unwrap(), "agent");
    }

    #[tokio::test]
    async fn get_note_returns_404_for_unknown_id() {
        let ctx = setup().await;
        let resp = ctx.router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(note_url("00000000-0000-0000-0000-000000000000"))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &ctx.member_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_note_returns_404_for_unknown_workspace_or_project() {
        let ctx = setup().await;
        let payload = serde_json::json!({
            "kind": "context",
            "title": null,
            "body_md": "Some context.",
            "agent_session_id": null
        });

        let resp = ctx.router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/ghost/projects/ghost/notes")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_note_rejects_empty_body_md() {
        let ctx = setup().await;
        let payload = serde_json::json!({
            "kind": "context",
            "title": null,
            "body_md": "   ",
            "agent_session_id": null
        });

        let resp = ctx.router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(notes_url())
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
