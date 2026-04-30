//! Test infrastructure: in-memory SQLite pool, fixture helpers, and smoke tests.
//!
//! This module is compiled only in test builds (`#[cfg(test)]` gate in `lib.rs`).
//! Use [`sqlite_test_pool`] to obtain a fresh, isolated database with all
//! migrations applied in each test.
//!
//! Use [`any_test_pool`] when you need an [`sqlx::AnyPool`] — for example, to
//! build an [`crate::app::AppState`] in handler-level integration tests.

pub mod fixtures;

use sqlx::SqlitePool;

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
