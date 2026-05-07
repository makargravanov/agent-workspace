use sqlx::AnyPool;

use crate::db::DatabaseBackend;

use super::domain::{Project, Workspace};

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

pub async fn get_workspace_by_slug(
    pool: &AnyPool,
    slug: &str,
) -> Result<Option<Workspace>, sqlx::Error> {
    sqlx::query_as::<_, Workspace>(
        "SELECT CAST(id AS TEXT) AS id, slug, name, \
                CAST(created_at AS TEXT) AS created_at, \
                CAST(updated_at AS TEXT) AS updated_at \
         FROM workspaces \
         WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn insert_workspace(
    pool: &AnyPool,
    db_backend: DatabaseBackend,
    id: &str,
    slug: &str,
    name: &str,
) -> Result<Workspace, sqlx::Error> {
    let query = match db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO workspaces (id, slug, name) VALUES (CAST($1 AS UUID), $2, $3)"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)"
        }
    };

    sqlx::query(query)
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
    pool: &AnyPool,
    workspace_id: &str,
) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT CAST(id AS TEXT) AS id, CAST(workspace_id AS TEXT) AS workspace_id, \
                slug, name, status, \
                CAST(created_at AS TEXT) AS created_at, \
                CAST(updated_at AS TEXT) AS updated_at \
         FROM projects \
         WHERE workspace_id = $1 \
         ORDER BY created_at DESC",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
}

pub async fn get_project_by_slug(
    pool: &AnyPool,
    workspace_id: &str,
    project_slug: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT CAST(id AS TEXT) AS id, CAST(workspace_id AS TEXT) AS workspace_id, \
                slug, name, status, \
                CAST(created_at AS TEXT) AS created_at, \
                CAST(updated_at AS TEXT) AS updated_at \
         FROM projects \
         WHERE workspace_id = $1 AND slug = $2",
    )
    .bind(workspace_id)
    .bind(project_slug)
    .fetch_optional(pool)
    .await
}

pub async fn insert_project(
    pool: &AnyPool,
    db_backend: DatabaseBackend,
    id: &str,
    workspace_id: &str,
    slug: &str,
    name: &str,
) -> Result<Project, sqlx::Error> {
    let query = match db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO projects (id, workspace_id, slug, name, status) \
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, 'active')"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO projects (id, workspace_id, slug, name, status) \
             VALUES ($1, $2, $3, $4, 'active')"
        }
    };

    sqlx::query(query)
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
