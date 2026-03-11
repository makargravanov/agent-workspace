use serde::{Deserialize, Serialize};

/// Workspace summary returned by list and detail endpoints.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Workspace {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Project summary returned by list and detail endpoints.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub workspace_id: String,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Body for `POST /workspaces`.
#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub slug: String,
    pub name: String,
}

/// Body for `POST /workspaces/{workspaceSlug}/projects`.
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub slug: String,
    pub name: String,
}
