use std::{env, error::Error};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::Method;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

const DEFAULT_API_BASE_URL: &str = "http://localhost:8080";

#[derive(Clone)]
struct ApiClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl ApiClient {
    fn from_env() -> Result<Self, String> {
        let base_url = env::var("AGENT_WORKSPACE_API_URL")
            .unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string());
        let token = env::var("AGENT_WORKSPACE_AGENT_TOKEN")
            .map_err(|_| "AGENT_WORKSPACE_AGENT_TOKEN is required".to_string())?;

        if token.trim().is_empty() {
            return Err("AGENT_WORKSPACE_AGENT_TOKEN must not be empty".to_string());
        }

        Ok(Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    async fn get(
        &self,
        path: String,
        query: Vec<(&str, String)>,
    ) -> Result<CallToolResult, ErrorData> {
        self.request(Method::GET, path, query, None).await
    }

    async fn post<T: Serialize>(
        &self,
        path: String,
        body: &T,
    ) -> Result<CallToolResult, ErrorData> {
        self.request(Method::POST, path, Vec::new(), Some(json_body(body)?))
            .await
    }

    async fn patch<T: Serialize>(
        &self,
        path: String,
        body: &T,
    ) -> Result<CallToolResult, ErrorData> {
        self.request(Method::PATCH, path, Vec::new(), Some(json_body(body)?))
            .await
    }

    async fn delete(&self, path: String) -> Result<CallToolResult, ErrorData> {
        self.request(Method::DELETE, path, Vec::new(), None).await
    }

    async fn download_base64(
        &self,
        path: String,
        query: Vec<(&str, String)>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.http.get(url).bearer_auth(&self.token);

        if !query.is_empty() {
            request = request.query(&query);
        }

        let response = request.send().await.map_err(|e| {
            ErrorData::internal_error(
                "failed to call Agent Workspace API",
                Some(json!({ "error": e.to_string() })),
            )
        })?;
        let status = response.status();
        let media_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        let content_disposition = response
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let parsed = serde_json::from_str::<Value>(&text).ok();
            let message = parsed
                .as_ref()
                .and_then(|value| value.pointer("/error/message"))
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    status
                        .canonical_reason()
                        .unwrap_or("Agent Workspace API error")
                })
                .to_string();
            return Err(ErrorData::internal_error(
                message,
                Some(json!({
                    "status": status.as_u16(),
                    "body": parsed.unwrap_or_else(|| json!({ "raw": text })),
                })),
            ));
        }

        let bytes = response.bytes().await.map_err(|e| {
            ErrorData::internal_error(
                "failed to read Agent Workspace API response",
                Some(json!({ "error": e.to_string() })),
            )
        })?;

        Ok(CallToolResult::structured(json!({
            "content_base64": STANDARD.encode(&bytes),
            "size_bytes": bytes.len(),
            "media_type": media_type,
            "content_disposition": content_disposition,
        })))
    }

    async fn request(
        &self,
        method: Method,
        path: String,
        query: Vec<(&str, String)>,
        body: Option<Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.http.request(method, url).bearer_auth(&self.token);

        if !query.is_empty() {
            request = request.query(&query);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|e| {
            ErrorData::internal_error(
                "failed to call Agent Workspace API",
                Some(json!({ "error": e.to_string() })),
            )
        })?;
        let status = response.status();
        let text = response.text().await.map_err(|e| {
            ErrorData::internal_error(
                "failed to read Agent Workspace API response",
                Some(json!({ "error": e.to_string() })),
            )
        })?;

        if status.is_success() {
            if text.trim().is_empty() {
                return Ok(CallToolResult::structured(json!({ "status": "ok" })));
            }
            let value = parse_json_response(&text)?;
            return Ok(CallToolResult::structured(value));
        }

        let parsed = serde_json::from_str::<Value>(&text).ok();
        let message = parsed
            .as_ref()
            .and_then(|value| value.pointer("/error/message"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                status
                    .canonical_reason()
                    .unwrap_or("Agent Workspace API error")
            })
            .to_string();

        Err(ErrorData::internal_error(
            message,
            Some(json!({
                "status": status.as_u16(),
                "body": parsed.unwrap_or_else(|| json!({ "raw": text })),
            })),
        ))
    }
}

fn json_body<T: Serialize>(body: &T) -> Result<Value, ErrorData> {
    serde_json::to_value(body).map_err(|e| {
        ErrorData::invalid_params(
            "failed to serialize tool payload",
            Some(json!({ "error": e.to_string() })),
        )
    })
}

fn parse_json_response(text: &str) -> Result<Value, ErrorData> {
    serde_json::from_str(text).map_err(|e| {
        ErrorData::internal_error(
            "Agent Workspace API returned invalid JSON",
            Some(json!({ "error": e.to_string() })),
        )
    })
}

pub struct AgentWorkspaceMcp {
    api: ApiClient,
    default_workspace_slug: Option<String>,
    default_project_slug: Option<String>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl AgentWorkspaceMcp {
    fn from_env() -> Result<Self, String> {
        Ok(Self {
            api: ApiClient::from_env()?,
            default_workspace_slug: env::var("AGENT_WORKSPACE_WORKSPACE_SLUG").ok(),
            default_project_slug: env::var("AGENT_WORKSPACE_PROJECT_SLUG").ok(),
            tool_router: Self::tool_router(),
        })
    }

    fn project_path(
        &self,
        workspace_slug: Option<&str>,
        project_slug: Option<&str>,
        suffix: &str,
    ) -> Result<String, ErrorData> {
        let workspace_slug = self.resolve_workspace_slug(workspace_slug)?;
        let project_slug = self.resolve_project_slug(project_slug)?;
        Ok(format!(
            "/api/v1/workspaces/{workspace_slug}/projects/{project_slug}{suffix}"
        ))
    }

    fn workspace_path(
        &self,
        workspace_slug: Option<&str>,
        suffix: &str,
    ) -> Result<String, ErrorData> {
        let workspace_slug = self.resolve_workspace_slug(workspace_slug)?;
        Ok(format!("/api/v1/workspaces/{workspace_slug}{suffix}"))
    }

    fn resolve_workspace_slug(&self, value: Option<&str>) -> Result<String, ErrorData> {
        resolve_value(
            value,
            self.default_workspace_slug.as_deref(),
            "workspace_slug",
        )
    }

    fn resolve_project_slug(&self, value: Option<&str>) -> Result<String, ErrorData> {
        resolve_value(value, self.default_project_slug.as_deref(), "project_slug")
    }
}

fn resolve_value(
    value: Option<&str>,
    default: Option<&str>,
    name: &'static str,
) -> Result<String, ErrorData> {
    value
        .or(default)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            ErrorData::invalid_params(
                format!("{name} is required when no environment default is configured"),
                Some(json!({ "missing": name })),
            )
        })
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProjectArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WorkspaceArgs {
    workspace_slug: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WorkspaceProjectArgs {
    workspace_slug: Option<String>,
    project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListTasksArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    status: Option<String>,
    group_id: Option<String>,
    assignee_id: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TaskIdArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    task_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateTaskArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    title: String,
    group_id: Option<String>,
    parent_task_id: Option<String>,
    description_md: Option<String>,
    priority: Option<String>,
    rank_key: Option<String>,
    assignee_type: Option<String>,
    assignee_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateTaskPayload {
    title: String,
    group_id: Option<String>,
    parent_task_id: Option<String>,
    description_md: Option<String>,
    priority: Option<String>,
    rank_key: Option<String>,
    assignee_type: Option<String>,
    assignee_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateTaskArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    task_id: String,
    title: Option<String>,
    group_id: Option<String>,
    parent_task_id: Option<String>,
    description_md: Option<String>,
    priority: Option<String>,
    rank_key: Option<String>,
    assignee_type: Option<String>,
    assignee_id: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateTaskPayload {
    title: Option<String>,
    group_id: Option<String>,
    parent_task_id: Option<String>,
    description_md: Option<String>,
    priority: Option<String>,
    rank_key: Option<String>,
    assignee_type: Option<String>,
    assignee_id: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateTaskStatusArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    task_id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct UpdateTaskStatusPayload {
    status: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DocumentIdArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    document_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateDocumentArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    slug: String,
    title: String,
    body_md: String,
    parent_document_id: Option<String>,
    body_format: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateDocumentPayload {
    slug: String,
    title: String,
    body_md: String,
    parent_document_id: Option<String>,
    body_format: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateDocumentArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    document_id: String,
    version: i32,
    slug: Option<String>,
    title: Option<String>,
    body_md: Option<String>,
    parent_document_id: Option<Option<String>>,
    body_format: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateDocumentPayload {
    version: i32,
    slug: Option<String>,
    title: Option<String>,
    body_md: Option<String>,
    parent_document_id: Option<Option<String>>,
    body_format: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MoveDocumentArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    document_id: String,
    target_parent_document_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct MoveDocumentPayload {
    target_parent_document_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListNotesArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NoteIdArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    note_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateNoteArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    kind: String,
    title: Option<String>,
    body_md: String,
    agent_session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateNotePayload {
    kind: String,
    title: Option<String>,
    body_md: String,
    agent_session_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateNoteArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    note_id: String,
    kind: Option<String>,
    title: Option<Option<String>>,
    body_md: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateNotePayload {
    kind: Option<String>,
    title: Option<Option<String>>,
    body_md: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TaskGroupIdArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    group_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateTaskGroupArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    kind: String,
    title: String,
    description_md: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Serialize)]
struct CreateTaskGroupPayload {
    kind: String,
    title: String,
    description_md: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateTaskGroupArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    group_id: String,
    kind: Option<String>,
    title: Option<String>,
    description_md: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Serialize)]
struct UpdateTaskGroupPayload {
    kind: Option<String>,
    title: Option<String>,
    description_md: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AssetIdArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    asset_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateAssetArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    file_name: String,
    media_type: String,
    content_base64: String,
    sha256: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateAssetPayload {
    file_name: String,
    media_type: String,
    content_base64: String,
    sha256: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateAssetArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    asset_id: String,
    file_name: Option<String>,
    media_type: Option<String>,
    content_base64: Option<String>,
    sha256: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateAssetPayload {
    file_name: Option<String>,
    media_type: Option<String>,
    content_base64: Option<String>,
    sha256: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DownloadAssetArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    asset_id: String,
    disposition: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListActivityArgs {
    workspace_slug: Option<String>,
    project_slug: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[tool_router]
impl AgentWorkspaceMcp {
    #[tool(description = "List workspaces available to the configured credential.")]
    async fn list_workspaces(&self) -> Result<CallToolResult, ErrorData> {
        self.api
            .get("/api/v1/workspaces".to_string(), Vec::new())
            .await
    }

    #[tool(description = "Get one workspace by slug, using the configured default if omitted.")]
    async fn get_workspace(
        &self,
        Parameters(args): Parameters<WorkspaceArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let workspace_slug = self.resolve_workspace_slug(args.workspace_slug.as_deref())?;
        self.api
            .get(format!("/api/v1/workspaces/{workspace_slug}"), Vec::new())
            .await
    }

    #[tool(description = "List projects available in a workspace to the configured credential.")]
    async fn list_projects(
        &self,
        Parameters(args): Parameters<WorkspaceArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.workspace_path(args.workspace_slug.as_deref(), "/projects")?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Get one project by slug.")]
    async fn get_project(
        &self,
        Parameters(args): Parameters<WorkspaceProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.workspace_path(
            args.workspace_slug.as_deref(),
            &format!("/projects/{}", args.project_slug),
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "List tasks in an Agent Workspace project.")]
    async fn list_tasks(
        &self,
        Parameters(args): Parameters<ListTasksArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Vec::new();
        if let Some(status) = args.status {
            query.push(("status", status));
        }
        if let Some(group_id) = args.group_id {
            query.push(("group_id", group_id));
        }
        if let Some(assignee_id) = args.assignee_id {
            query.push(("assignee_id", assignee_id));
        }
        if let Some(limit) = args.limit {
            query.push(("limit", limit.to_string()));
        }

        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/tasks",
        )?;
        self.api.get(path, query).await
    }

    #[tool(description = "Get one task by id.")]
    async fn get_task(
        &self,
        Parameters(args): Parameters<TaskIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/tasks/{}", args.task_id),
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Create a task.")]
    async fn create_task(
        &self,
        Parameters(args): Parameters<CreateTaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/tasks",
        )?;
        let payload = CreateTaskPayload {
            title: args.title,
            group_id: args.group_id,
            parent_task_id: args.parent_task_id,
            description_md: args.description_md,
            priority: args.priority,
            rank_key: args.rank_key,
            assignee_type: args.assignee_type,
            assignee_id: args.assignee_id,
        };
        self.api.post(path, &payload).await
    }

    #[tool(description = "Update task metadata.")]
    async fn update_task(
        &self,
        Parameters(args): Parameters<UpdateTaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/tasks/{}", args.task_id),
        )?;
        let payload = UpdateTaskPayload {
            title: args.title,
            group_id: args.group_id,
            parent_task_id: args.parent_task_id,
            description_md: args.description_md,
            priority: args.priority,
            rank_key: args.rank_key,
            assignee_type: args.assignee_type,
            assignee_id: args.assignee_id,
            status: args.status,
        };
        self.api.patch(path, &payload).await
    }

    #[tool(description = "Delete a task.")]
    async fn delete_task(
        &self,
        Parameters(args): Parameters<TaskIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/tasks/{}", args.task_id),
        )?;
        self.api.delete(path).await
    }

    #[tool(description = "Update only a task status.")]
    async fn update_task_status(
        &self,
        Parameters(args): Parameters<UpdateTaskStatusArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/tasks/{}/status", args.task_id),
        )?;
        self.api
            .patch(
                path,
                &UpdateTaskStatusPayload {
                    status: args.status,
                },
            )
            .await
    }

    #[tool(description = "List documents in an Agent Workspace project.")]
    async fn list_documents(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/documents",
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Get one document by id.")]
    async fn get_document(
        &self,
        Parameters(args): Parameters<DocumentIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/documents/{}", args.document_id),
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Create a markdown document.")]
    async fn create_document(
        &self,
        Parameters(args): Parameters<CreateDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/documents",
        )?;
        let payload = CreateDocumentPayload {
            slug: args.slug,
            title: args.title,
            body_md: args.body_md,
            parent_document_id: args.parent_document_id,
            body_format: args.body_format,
            status: args.status,
        };
        self.api.post(path, &payload).await
    }

    #[tool(description = "Update a document using its optimistic-lock version.")]
    async fn update_document(
        &self,
        Parameters(args): Parameters<UpdateDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/documents/{}", args.document_id),
        )?;
        let payload = UpdateDocumentPayload {
            version: args.version,
            slug: args.slug,
            title: args.title,
            body_md: args.body_md,
            parent_document_id: args.parent_document_id,
            body_format: args.body_format,
            status: args.status,
        };
        self.api.patch(path, &payload).await
    }

    #[tool(description = "Delete a document.")]
    async fn delete_document(
        &self,
        Parameters(args): Parameters<DocumentIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/documents/{}", args.document_id),
        )?;
        self.api.delete(path).await
    }

    #[tool(description = "Move a document under another document or to the project root.")]
    async fn move_document(
        &self,
        Parameters(args): Parameters<MoveDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/documents/{}/move", args.document_id),
        )?;
        self.api
            .post(
                path,
                &MoveDocumentPayload {
                    target_parent_document_id: args.target_parent_document_id,
                },
            )
            .await
    }

    #[tool(description = "List notes in an Agent Workspace project.")]
    async fn list_notes(
        &self,
        Parameters(args): Parameters<ListNotesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Vec::new();
        if let Some(page) = args.page {
            query.push(("page", page.to_string()));
        }
        if let Some(per_page) = args.per_page {
            query.push(("per_page", per_page.to_string()));
        }

        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/notes",
        )?;
        self.api.get(path, query).await
    }

    #[tool(description = "Get one note by id.")]
    async fn get_note(
        &self,
        Parameters(args): Parameters<NoteIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/notes/{}", args.note_id),
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Create a note.")]
    async fn create_note(
        &self,
        Parameters(args): Parameters<CreateNoteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/notes",
        )?;
        let payload = CreateNotePayload {
            kind: args.kind,
            title: args.title,
            body_md: args.body_md,
            agent_session_id: args.agent_session_id,
        };
        self.api.post(path, &payload).await
    }

    #[tool(description = "Update a note.")]
    async fn update_note(
        &self,
        Parameters(args): Parameters<UpdateNoteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/notes/{}", args.note_id),
        )?;
        let payload = UpdateNotePayload {
            kind: args.kind,
            title: args.title,
            body_md: args.body_md,
        };
        self.api.patch(path, &payload).await
    }

    #[tool(description = "Delete a note.")]
    async fn delete_note(
        &self,
        Parameters(args): Parameters<NoteIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/notes/{}", args.note_id),
        )?;
        self.api.delete(path).await
    }

    #[tool(description = "List task groups in an Agent Workspace project.")]
    async fn list_task_groups(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/task-groups",
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Get one task group by id.")]
    async fn get_task_group(
        &self,
        Parameters(args): Parameters<TaskGroupIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/task-groups/{}", args.group_id),
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Create a task group.")]
    async fn create_task_group(
        &self,
        Parameters(args): Parameters<CreateTaskGroupArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/task-groups",
        )?;
        let payload = CreateTaskGroupPayload {
            kind: args.kind,
            title: args.title,
            description_md: args.description_md,
            status: args.status,
            priority: args.priority,
        };
        self.api.post(path, &payload).await
    }

    #[tool(description = "Update a task group.")]
    async fn update_task_group(
        &self,
        Parameters(args): Parameters<UpdateTaskGroupArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/task-groups/{}", args.group_id),
        )?;
        let payload = UpdateTaskGroupPayload {
            kind: args.kind,
            title: args.title,
            description_md: args.description_md,
            status: args.status,
            priority: args.priority,
        };
        self.api.patch(path, &payload).await
    }

    #[tool(description = "Delete a task group.")]
    async fn delete_task_group(
        &self,
        Parameters(args): Parameters<TaskGroupIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/task-groups/{}", args.group_id),
        )?;
        self.api.delete(path).await
    }

    #[tool(description = "List assets in an Agent Workspace project.")]
    async fn list_assets(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/assets",
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Get one asset metadata record by id.")]
    async fn get_asset(
        &self,
        Parameters(args): Parameters<AssetIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/assets/{}", args.asset_id),
        )?;
        self.api.get(path, Vec::new()).await
    }

    #[tool(description = "Create an asset from base64 content.")]
    async fn create_asset(
        &self,
        Parameters(args): Parameters<CreateAssetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/assets",
        )?;
        let payload = CreateAssetPayload {
            file_name: args.file_name,
            media_type: args.media_type,
            content_base64: args.content_base64,
            sha256: args.sha256,
        };
        self.api.post(path, &payload).await
    }

    #[tool(description = "Update asset metadata and optionally its base64 content.")]
    async fn update_asset(
        &self,
        Parameters(args): Parameters<UpdateAssetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/assets/{}", args.asset_id),
        )?;
        let payload = UpdateAssetPayload {
            file_name: args.file_name,
            media_type: args.media_type,
            content_base64: args.content_base64,
            sha256: args.sha256,
        };
        self.api.patch(path, &payload).await
    }

    #[tool(description = "Delete an asset.")]
    async fn delete_asset(
        &self,
        Parameters(args): Parameters<AssetIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/assets/{}", args.asset_id),
        )?;
        self.api.delete(path).await
    }

    #[tool(description = "Download an asset and return its content as base64.")]
    async fn download_asset(
        &self,
        Parameters(args): Parameters<DownloadAssetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Vec::new();
        if let Some(disposition) = args.disposition {
            query.push(("disposition", disposition));
        }
        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            &format!("/assets/{}/download", args.asset_id),
        )?;
        self.api.download_base64(path, query).await
    }

    #[tool(description = "List recent workspace activity visible to the configured credential.")]
    async fn list_workspace_activity(
        &self,
        Parameters(args): Parameters<ListActivityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Vec::new();
        if let Some(page) = args.page {
            query.push(("page", page.to_string()));
        }
        if let Some(per_page) = args.per_page {
            query.push(("per_page", per_page.to_string()));
        }

        let path = self.workspace_path(args.workspace_slug.as_deref(), "/activity")?;
        self.api.get(path, query).await
    }

    #[tool(description = "List recent project activity visible to the configured credential.")]
    async fn list_project_activity(
        &self,
        Parameters(args): Parameters<ListActivityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Vec::new();
        if let Some(page) = args.page {
            query.push(("page", page.to_string()));
        }
        if let Some(per_page) = args.per_page {
            query.push(("per_page", per_page.to_string()));
        }

        let path = self.project_path(
            args.workspace_slug.as_deref(),
            args.project_slug.as_deref(),
            "/activity",
        )?;
        self.api.get(path, query).await
    }
}

#[tool_handler]
impl ServerHandler for AgentWorkspaceMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::LATEST)
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Access Agent Workspace discovery, tasks, task groups, documents, notes, assets, and activity through the configured API token.",
            )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let mode = env::args().nth(1).unwrap_or_else(|| "stdio".to_string());
    if mode != "stdio" {
        return Err(format!("unsupported mode '{mode}'; only 'stdio' is implemented").into());
    }

    let server = AgentWorkspaceMcp::from_env()?;
    info!(
        api_base_url = %server.api.base_url,
        tools = "discovery,tasks,task_groups,documents,notes,assets,activity",
        "agent-workspace-mcp starting"
    );

    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_workspace_mcp=info".into()),
        )
        .compact()
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_value_uses_explicit_value_before_default() {
        let value = resolve_value(Some("explicit"), Some("default"), "workspace_slug").unwrap();
        assert_eq!(value, "explicit");
    }

    #[test]
    fn resolve_value_rejects_missing_value_without_default() {
        let error = resolve_value(None, None, "project_slug").unwrap_err();
        assert!(error.message.contains("project_slug is required"));
    }
}
