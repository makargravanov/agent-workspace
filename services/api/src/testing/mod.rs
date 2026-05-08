//! Test infrastructure: in-memory SQLite pool, fixture helpers, and smoke tests.
//!
//! This module is compiled only in test builds (`#[cfg(test)]` gate in `lib.rs`).
//! Use [`sqlite_test_pool`] to obtain a fresh, isolated database with all
//! migrations applied in each test.
//!
//! Use [`any_test_pool`] when you need an [`sqlx::AnyPool`] — for example, to
//! build an [`crate::app::AppState`] in handler-level integration tests.

pub mod fixtures;

use reqwest::Url;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Build an in-memory SQLite pool, apply the SQLite migration set, and enable
/// foreign key enforcement for the connection.
///
/// Every call returns a completely fresh database — no state leaks between tests.
pub async fn sqlite_test_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("in-memory SQLite pool should open");

    // SQLite foreign keys are off by default; turn them on for every connection
    // in the pool so constraint violations are caught during tests.
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("PRAGMA foreign_keys = ON should execute");

    sqlx::migrate!("./migrations_sqlite")
        .run(&pool)
        .await
        .expect("SQLite migrations should apply without error");

    pool
}

/// Build an in-memory SQLite [`sqlx::AnyPool`], apply the SQLite migration set,
/// and enable foreign key enforcement.
///
/// Use this helper when a test needs to construct an [`crate::app::AppState`] for
/// handler-level integration tests.  Calls
/// [`sqlx::any::install_default_drivers`] internally so it is safe to call it
/// multiple times.
pub async fn any_test_pool() -> sqlx::AnyPool {
    sqlx::any::install_default_drivers();

    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory AnyPool (SQLite) should open");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("PRAGMA foreign_keys = ON should execute");

    sqlx::migrate!("./migrations_sqlite")
        .run(&pool)
        .await
        .expect("SQLite migrations should apply without error");

    pool
}

pub struct PostgresTestDb {
    pub pool: sqlx::AnyPool,
    admin_url: String,
    db_name: String,
}

impl PostgresTestDb {
    pub async fn cleanup(self) {
        self.pool.close().await;

        if let Ok(admin_pool) = sqlx::PgPool::connect(&self.admin_url).await {
            let drop_sql = format!(r#"DROP DATABASE IF EXISTS "{}" WITH (FORCE)"#, self.db_name);
            let _ = sqlx::query(&drop_sql).execute(&admin_pool).await;
            admin_pool.close().await;
        }
    }
}

pub async fn postgres_test_db() -> Option<PostgresTestDb> {
    let admin_url = std::env::var("TEST_DATABASE_URL").ok()?;
    sqlx::any::install_default_drivers();

    let admin_pool = sqlx::PgPool::connect(&admin_url).await.ok()?;
    let db_name = format!("agent_workspace_test_{}", Uuid::new_v4().simple());
    let create_sql = format!(r#"CREATE DATABASE "{}""#, db_name);
    sqlx::query(&create_sql).execute(&admin_pool).await.ok()?;
    admin_pool.close().await;

    let mut db_url = Url::parse(&admin_url).ok()?;
    db_url.set_path(&format!("/{}", db_name));

    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(4)
        .connect(db_url.as_str())
        .await
        .ok()?;

    sqlx::migrate!("./migrations").run(&pool).await.ok()?;

    Some(PostgresTestDb {
        pool,
        admin_url,
        db_name,
    })
}


#[cfg(test)]
mod smoke {
    use super::*;

    // ------------------------------------------------------------------
    // Pool and migration smoke tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pool_opens_and_migrations_run() {
        // Verifies that the SQLite migration set is syntactically valid and
        // executes without error against a fresh in-memory database.
        let pool = sqlite_test_pool().await;
        pool.close().await;
    }

    #[tokio::test]
    async fn all_core_tables_exist() {
        let pool = sqlite_test_pool().await;

        let tables = [
            "workspaces",
            "workspace_members",
            "human_identities",
            "human_sessions",
            "projects",
            "task_groups",
            "tasks",
            "task_dependencies",
            "documents",
            "assets",
            "agents",
            "agent_credentials",
            "agent_sessions",
            "agent_session_tasks",
            "notes",
            "links",
            "integration_connections",
            "audit_events",
        ];

        for table in &tables {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|_| panic!("sqlite_master query failed for table '{table}'"));

            assert_eq!(count, 1, "table '{table}' should exist after migrations");
        }

        pool.close().await;
    }

    // ------------------------------------------------------------------
    // Fixture roundtrip smoke tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn workspace_insert_and_select_roundtrip() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let (name,): (String,) =
            sqlx::query_as("SELECT name FROM workspaces WHERE id = ?")
                .bind(seed.workspace_id.to_string())
                .fetch_one(&pool)
                .await
                .expect("seeded workspace should be selectable");

        assert_eq!(name, fixtures::WORKSPACE_NAME);
        pool.close().await;
    }

    #[tokio::test]
    async fn project_belongs_to_workspace() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let (ws_id,): (String,) =
            sqlx::query_as("SELECT workspace_id FROM projects WHERE id = ?")
                .bind(seed.project_id.to_string())
                .fetch_one(&pool)
                .await
                .expect("seeded project should be selectable");

        assert_eq!(ws_id, seed.workspace_id.to_string());
        pool.close().await;
    }

    #[tokio::test]
    async fn tasks_belong_to_project_and_workspace() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND workspace_id = ?",
        )
        .bind(seed.project_id.to_string())
        .bind(seed.workspace_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("task count query should succeed");

        assert!(count >= 3, "at least 3 tasks should be seeded; got {count}");
        pool.close().await;
    }

    #[tokio::test]
    async fn task_dependency_links_two_seeded_tasks() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let (dep_type,): (String,) = sqlx::query_as(
            "SELECT dependency_type FROM task_dependencies \
             WHERE predecessor_task_id = ? AND successor_task_id = ?",
        )
        .bind(seed.task_ids[0].to_string())
        .bind(seed.task_ids[1].to_string())
        .fetch_one(&pool)
        .await
        .expect("seeded task dependency should be selectable");

        assert_eq!(dep_type, "blocks");
        pool.close().await;
    }

    #[tokio::test]
    async fn task_group_kind_is_epic() {
        let pool = sqlite_test_pool().await;
        let seed = fixtures::seed_minimal(&pool).await;

        let (kind,): (String,) =
            sqlx::query_as("SELECT kind FROM task_groups WHERE id = ?")
                .bind(seed.task_group_id.to_string())
                .fetch_one(&pool)
                .await
                .expect("seeded task_group should be selectable");

        assert_eq!(kind, "epic");
        pool.close().await;
    }
}

#[cfg(test)]
mod postgres_smoke {
    use super::postgres_test_db;
    use crate::{app::build_router, db::DatabaseBackend, state::AppState};
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn body_json(body: Body) -> Value {
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("body should be readable");
        serde_json::from_slice(&bytes).expect("body should be json")
    }

    async fn seed_owner_member(pool: &sqlx::AnyPool) -> (String, String) {
        let workspace_id = Uuid::new_v4().to_string();
        let member_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO workspaces (id, slug, name)
             VALUES (CAST($1 AS UUID), $2, $3)",
        )
        .bind(&workspace_id)
        .bind("seed-ws")
        .bind("Seed Workspace")
        .execute(pool)
        .await
        .expect("insert workspace");

        sqlx::query(
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, 'owner', 'active')",
        )
        .bind(&member_id)
        .bind(&workspace_id)
        .bind("test:owner-1")
        .bind("Test Owner")
        .execute(pool)
        .await
        .expect("insert workspace member");

        (workspace_id, member_id)
    }

    #[tokio::test]
    async fn postgres_workspace_project_task_note_flow() {
        let Some(db) = postgres_test_db().await else {
            eprintln!("skipping postgres smoke test: TEST_DATABASE_URL is not set");
            return;
        };

        let (_workspace_id, member_id) = seed_owner_member(&db.pool).await;
        let app = build_router(AppState::new(db.pool.clone(), DatabaseBackend::Postgres));

        let list_workspaces = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(list_workspaces.status(), StatusCode::OK);

        let create_workspace = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::from(
                        json!({ "slug": "pg-child", "name": "PG Child" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_workspace.status(), StatusCode::CREATED);

        let create_project = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/pg-child/projects")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::from(
                        json!({ "slug": "pg-proj", "name": "PG Project" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_project.status(), StatusCode::CREATED);

        let create_task = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/pg-child/projects/pg-proj/tasks")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::from(
                        json!({
                            "title": "Postgres task",
                            "description_md": "Created in postgres smoke test",
                            "priority": "normal"
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_task.status(), StatusCode::CREATED);
        let task_body = body_json(create_task.into_body()).await;
        let task_id = task_body["data"]["id"].as_str().expect("task id").to_string();

        let list_tasks = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/pg-child/projects/pg-proj/tasks")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(list_tasks.status(), StatusCode::OK);

        let update_task = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/workspaces/pg-child/projects/pg-proj/tasks/{task_id}/status"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::from(json!({ "status": "done" }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(update_task.status(), StatusCode::OK);

        let create_note = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces/pg-child/projects/pg-proj/notes")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::from(
                        json!({
                            "kind": "decision",
                            "title": "Postgres note",
                            "body_md": "Created in postgres smoke test"
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_note.status(), StatusCode::CREATED);

        let list_notes = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspaces/pg-child/projects/pg-proj/notes")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(list_notes.status(), StatusCode::OK);

        db.cleanup().await;
    }
}
