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
        DatabaseBackend::Sqlite => "INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)",
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

pub async fn update_workspace(
    pool: &AnyPool,
    db_backend: DatabaseBackend,
    workspace_id: &str,
    new_slug: Option<&str>,
    new_name: Option<&str>,
) -> Result<Workspace, sqlx::Error> {
    let update_sql = match db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE workspaces
             SET slug = COALESCE($2, slug),
                 name = COALESCE($3, name),
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS UUID) = CAST($1 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE workspaces
             SET slug = COALESCE($2, slug),
                 name = COALESCE($3, name),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $1"
        }
    };

    sqlx::query(update_sql)
        .bind(workspace_id)
        .bind(new_slug)
        .bind(new_name)
        .execute(pool)
        .await?;

    sqlx::query_as::<_, Workspace>(
        "SELECT CAST(id AS TEXT) AS id, slug, name,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM workspaces
         WHERE CAST(id AS TEXT) = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
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
         WHERE CAST(workspace_id AS TEXT) = $1 \
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
         WHERE CAST(workspace_id AS TEXT) = $1 AND slug = $2",
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

pub async fn update_project(
    pool: &AnyPool,
    db_backend: DatabaseBackend,
    project_id: &str,
    new_slug: Option<&str>,
    new_name: Option<&str>,
    new_status: Option<&str>,
) -> Result<Project, sqlx::Error> {
    let update_sql = match db_backend {
        DatabaseBackend::Postgres => {
            "UPDATE projects
             SET slug = COALESCE($2, slug),
                 name = COALESCE($3, name),
                 status = COALESCE($4, status),
                 updated_at = CURRENT_TIMESTAMP
             WHERE CAST(id AS UUID) = CAST($1 AS UUID)"
        }
        DatabaseBackend::Sqlite => {
            "UPDATE projects
             SET slug = COALESCE($2, slug),
                 name = COALESCE($3, name),
                 status = COALESCE($4, status),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $1"
        }
    };

    sqlx::query(update_sql)
        .bind(project_id)
        .bind(new_slug)
        .bind(new_name)
        .bind(new_status)
        .execute(pool)
        .await?;

    sqlx::query_as::<_, Project>(
        "SELECT CAST(id AS TEXT) AS id, CAST(workspace_id AS TEXT) AS workspace_id,
                slug, name, status,
                CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM projects
         WHERE CAST(id AS TEXT) = $1",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
}
