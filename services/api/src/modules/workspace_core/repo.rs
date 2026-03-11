use sqlx::SqlitePool;

use super::domain::{Project, Workspace};

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

pub async fn list_workspaces(pool: &SqlitePool) -> Result<Vec<Workspace>, sqlx::Error> {
    sqlx::query_as::<_, Workspace>(
        "SELECT id, slug, name, created_at, updated_at \
         FROM workspaces \
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_workspace_by_slug(
    pool: &SqlitePool,
    slug: &str,
) -> Result<Option<Workspace>, sqlx::Error> {
    sqlx::query_as::<_, Workspace>(
        "SELECT id, slug, name, created_at, updated_at \
         FROM workspaces \
         WHERE slug = ?",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn insert_workspace(
    pool: &SqlitePool,
    id: &str,
    slug: &str,
    name: &str,
) -> Result<Workspace, sqlx::Error> {
    sqlx::query(
        "INSERT INTO workspaces (id, slug, name) VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(slug)
    .bind(name)
    .execute(pool)
    .await?;

    // Return the created row (with server-generated timestamps).
    get_workspace_by_slug(pool, slug)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

pub async fn list_projects(
    pool: &SqlitePool,
    workspace_id: &str,
) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, workspace_id, slug, name, status, created_at, updated_at \
         FROM projects \
         WHERE workspace_id = ? \
         ORDER BY created_at DESC",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
}

pub async fn get_project_by_slug(
    pool: &SqlitePool,
    workspace_id: &str,
    project_slug: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, workspace_id, slug, name, status, created_at, updated_at \
         FROM projects \
         WHERE workspace_id = ? AND slug = ?",
    )
    .bind(workspace_id)
    .bind(project_slug)
    .fetch_optional(pool)
    .await
}

pub async fn insert_project(
    pool: &SqlitePool,
    id: &str,
    workspace_id: &str,
    slug: &str,
    name: &str,
) -> Result<Project, sqlx::Error> {
    sqlx::query(
        "INSERT INTO projects (id, workspace_id, slug, name, status) \
         VALUES (?, ?, ?, ?, 'active')",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(slug)
    .bind(name)
    .execute(pool)
    .await?;

    get_project_by_slug(pool, workspace_id, slug)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}
