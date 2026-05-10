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

    async fn seed_member(
        pool: &sqlx::AnyPool,
        workspace_slug: &str,
        workspace_name: &str,
        external_subject: &str,
        display_name: &str,
        role: &str,
    ) -> (String, String) {
        let workspace_id = Uuid::new_v4().to_string();
        let member_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO workspaces (id, slug, name)
             VALUES (CAST($1 AS UUID), $2, $3)",
        )
        .bind(&workspace_id)
        .bind(workspace_slug)
        .bind(workspace_name)
        .execute(pool)
        .await
        .expect("insert workspace");

        sqlx::query(
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, $5, 'active')",
        )
        .bind(&member_id)
        .bind(&workspace_id)
        .bind(external_subject)
        .bind(display_name)
        .bind(role)
        .execute(pool)
        .await
        .expect("insert workspace member");

        (workspace_id, member_id)
    }

    async fn fetch_id_by_slug(pool: &sqlx::AnyPool, table: &str, slug: &str) -> String {
        let query = match table {
            "workspaces" => "SELECT CAST(id AS TEXT) AS id FROM workspaces WHERE slug = $1",
            "projects" => "SELECT CAST(id AS TEXT) AS id FROM projects WHERE slug = $1",
            other => panic!("unsupported table for slug lookup: {other}"),
        };

        let row: (String,) = sqlx::query_as(query)
            .bind(slug)
            .fetch_one(pool)
            .await
            .expect("fetch id by slug");
        row.0
    }

    async fn fetch_workspace_membership(
        pool: &sqlx::AnyPool,
        workspace_id: &str,
    ) -> (String, String, String) {
        sqlx::query_as(
            "SELECT role, external_subject, display_name
             FROM workspace_members
             WHERE workspace_id = CAST($1 AS UUID) AND status = 'active'",
        )
        .bind(workspace_id)
        .fetch_one(pool)
        .await
        .expect("fetch workspace membership")
    }

    async fn insert_task_group(
        pool: &sqlx::AnyPool,
        workspace_id: &str,
        project_id: &str,
        title: &str,
    ) -> String {
        let task_group_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO task_groups
             (id, workspace_id, project_id, kind, title, status, priority)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), CAST($3 AS UUID), 'epic', $4, 'active', 0)",
        )
        .bind(&task_group_id)
        .bind(workspace_id)
        .bind(project_id)
        .bind(title)
        .execute(pool)
        .await
        .expect("insert task group");

        task_group_id
    }

    #[tokio::test]
    async fn postgres_workspace_project_task_note_flow() {
        let Some(db) = postgres_test_db().await else {
            eprintln!("skipping postgres smoke test: TEST_DATABASE_URL is not set");
            return;
        };

        let (_workspace_id, member_id) = seed_member(
            &db.pool,
            "seed-ws",
            "Seed Workspace",
            "test:owner-1",
            "Test Owner",
            "owner",
        )
        .await;
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
        let list_workspaces_body = body_json(list_workspaces.into_body()).await;
        assert_eq!(list_workspaces_body["data"]["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            list_workspaces_body["data"]["items"][0]["slug"],
            "seed-ws"
        );

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

        let child_workspace_id = fetch_id_by_slug(&db.pool, "workspaces", "pg-child").await;
        let (role, external_subject, display_name) =
            fetch_workspace_membership(&db.pool, &child_workspace_id).await;
        assert_eq!(role, "owner");
        assert_eq!(external_subject, "test:owner-1");
        assert_eq!(display_name, "Test Owner");

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

        let project_id = fetch_id_by_slug(&db.pool, "projects", "pg-proj").await;
        let task_group_id =
            insert_task_group(&db.pool, &child_workspace_id, &project_id, "Grouped Tasks").await;

        let grouped_task = app
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
                            "priority": "normal",
                            "group_id": task_group_id,
                            "assignee_type": "workspace_member",
                            "assignee_id": member_id
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(grouped_task.status(), StatusCode::CREATED);
        let grouped_task_body = body_json(grouped_task.into_body()).await;
        let grouped_task_id = grouped_task_body["data"]["id"]
            .as_str()
            .expect("task id")
            .to_string();
        assert_eq!(grouped_task_body["data"]["group_id"].as_str().unwrap(), task_group_id);
        assert_eq!(
            grouped_task_body["data"]["assignee_id"].as_str().unwrap(),
            member_id
        );

        let second_task = app
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
                            "title": "Postgres task 2",
                            "description_md": "Created in postgres smoke test",
                            "priority": "normal"
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(second_task.status(), StatusCode::CREATED);
        let second_task_body = body_json(second_task.into_body()).await;
        let second_task_id = second_task_body["data"]["id"]
            .as_str()
            .expect("task id")
            .to_string();

        let child_task = app
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
                            "title": "Postgres child task",
                            "description_md": "Created in postgres smoke test",
                            "priority": "normal",
                            "parent_task_id": grouped_task_id
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(child_task.status(), StatusCode::CREATED);
        let child_task_body = body_json(child_task.into_body()).await;
        let child_task_id = child_task_body["data"]["id"]
            .as_str()
            .expect("task id")
            .to_string();

        sqlx::query(
            "INSERT INTO task_dependencies
             (id, workspace_id, project_id, predecessor_task_id, successor_task_id, dependency_type, is_hard_block)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), CAST($3 AS UUID), CAST($4 AS UUID), CAST($5 AS UUID), $6, $7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&child_workspace_id)
        .bind(&project_id)
        .bind(&grouped_task_id)
        .bind(&child_task_id)
        .bind("blocks")
        .bind(true)
        .execute(&db.pool)
        .await
        .expect("insert postgres dependency");

        let list_tasks = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/workspaces/pg-child/projects/pg-proj/tasks?status=todo&group_id={task_group_id}&assignee_id={member_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(list_tasks.status(), StatusCode::OK);
        let list_tasks_body = body_json(list_tasks.into_body()).await;
        let items = list_tasks_body["data"]["items"]
            .as_array()
            .expect("tasks list items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"].as_str().unwrap(), grouped_task_id);

        let update_task = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/workspaces/pg-child/projects/pg-proj/tasks/{second_task_id}/status"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::from(json!({ "status": "done" }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(update_task.status(), StatusCode::OK);

        let (status,): (String,) = sqlx::query_as(
            "SELECT status FROM tasks WHERE CAST(id AS TEXT) = $1",
        )
        .bind(&second_task_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch updated task status");
        assert_eq!(status, "done");

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
        let create_note_body = body_json(create_note.into_body()).await;
        let note_id = create_note_body["data"]["id"]
            .as_str()
            .expect("note id")
            .to_string();

        let list_notes = app
            .clone()
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
        let list_notes_body = body_json(list_notes.into_body()).await;
        assert_eq!(list_notes_body["data"]["items"].as_array().unwrap().len(), 1);

        let delete_note = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/pg-child/projects/pg-proj/notes/{note_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(delete_note.status(), StatusCode::NO_CONTENT);

        let get_deleted_note = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/workspaces/pg-child/projects/pg-proj/notes/{note_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(get_deleted_note.status(), StatusCode::NOT_FOUND);

        let list_notes_after_delete = app
            .clone()
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
        assert_eq!(list_notes_after_delete.status(), StatusCode::OK);
        let list_notes_after_delete_body = body_json(list_notes_after_delete.into_body()).await;
        assert_eq!(
            list_notes_after_delete_body["data"]["items"].as_array().unwrap().len(),
            0
        );

        let delete_task = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/workspaces/pg-child/projects/pg-proj/tasks/{grouped_task_id}"
                    ))
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(delete_task.status(), StatusCode::NO_CONTENT);

        let (deleted_task_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tasks WHERE CAST(id AS TEXT) = $1",
        )
        .bind(&grouped_task_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch deleted task count");
        assert_eq!(deleted_task_count, 0);

        let (child_parent_task_id,): (Option<String>,) = sqlx::query_as(
            "SELECT CAST(parent_task_id AS TEXT) FROM tasks WHERE CAST(id AS TEXT) = $1",
        )
        .bind(&child_task_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch child parent task id");
        assert!(child_parent_task_id.is_none());

        let (dependency_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM task_dependencies WHERE CAST(project_id AS TEXT) = $1",
        )
        .bind(&project_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch dependency count");
        assert_eq!(dependency_count, 0);

        let delete_project = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/workspaces/pg-child/projects/pg-proj")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(delete_project.status(), StatusCode::NO_CONTENT);

        let (project_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM projects WHERE CAST(id AS TEXT) = $1",
        )
        .bind(&project_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch project count");
        assert_eq!(project_remaining, 0);

        let (workspace_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM workspaces WHERE CAST(id AS TEXT) = $1",
        )
        .bind(&child_workspace_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch workspace count");
        assert_eq!(workspace_remaining, 1);

        let delete_workspace = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/workspaces/pg-child")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", &member_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(delete_workspace.status(), StatusCode::NO_CONTENT);

        let (workspace_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM workspaces WHERE CAST(id AS TEXT) = $1",
        )
        .bind(&child_workspace_id)
        .fetch_one(&db.pool)
        .await
        .expect("fetch workspace count after delete");
        assert_eq!(workspace_remaining, 0);

        db.cleanup().await;
    }

    #[tokio::test]
    async fn postgres_detached_human_cannot_create_workspace() {
        let Some(db) = postgres_test_db().await else {
            eprintln!("skipping postgres smoke test: TEST_DATABASE_URL is not set");
            return;
        };

        let app = build_router(AppState::new(db.pool.clone(), DatabaseBackend::Postgres));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workspaces")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("x-actor-kind", "human")
                    .header("x-actor-id", Uuid::new_v4().to_string())
                    .body(Body::from(
                        json!({ "slug": "detached", "name": "Detached" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn postgres_workspaces_are_listed_for_shared_identity() {
        let Some(db) = postgres_test_db().await else {
            eprintln!("skipping postgres smoke test: TEST_DATABASE_URL is not set");
            return;
        };

        let (_workspace_id, member_id) = seed_member(
            &db.pool,
            "seed-ws",
            "Seed Workspace",
            "test:shared-identity",
            "Test Shared",
            "owner",
        )
        .await;

        let second_workspace_id = Uuid::new_v4().to_string();
        let second_member_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO workspaces (id, slug, name) VALUES (CAST($1 AS UUID), $2, $3)")
            .bind(&second_workspace_id)
            .bind("other-ws")
            .bind("Other Workspace")
            .execute(&db.pool)
            .await
            .expect("insert second workspace");

        sqlx::query(
            "INSERT INTO workspace_members
             (id, workspace_id, external_subject, display_name, role, status)
             VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, 'viewer', 'active')",
        )
        .bind(&second_member_id)
        .bind(&second_workspace_id)
        .bind("test:shared-identity")
        .bind("Test Shared")
        .execute(&db.pool)
        .await
        .expect("insert second membership");

        let app = build_router(AppState::new(db.pool.clone(), DatabaseBackend::Postgres));
        let response = app
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

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response.into_body()).await;
        let slugs: Vec<String> = body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["slug"].as_str().unwrap().to_string())
            .collect();

        assert!(slugs.contains(&"seed-ws".to_string()));
        assert!(slugs.contains(&"other-ws".to_string()));

        db.cleanup().await;
    }
}
