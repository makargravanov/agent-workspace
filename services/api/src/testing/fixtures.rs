//! Typed fixture data and the [`seed_minimal`] function that inserts a
//! representative, navigable data set into a test SQLite pool.
//!
//! All UUIDs are generated fresh per call so tests are fully isolated from each
//! other even when multiple tests run concurrently.

use sqlx::SqlitePool;
use uuid::Uuid;

// Display names / slugs used by the fixture set — exposed so smoke tests can
// assert against them without duplicating the string literals.
pub const WORKSPACE_NAME: &str = "Dev Workspace";
pub const WORKSPACE_SLUG: &str = "dev-workspace";
pub const PROJECT_NAME: &str = "Main Project";
pub const PROJECT_SLUG: &str = "main-project";
pub const TASK_GROUP_TITLE: &str = "First Epic";

/// IDs of the row set inserted by [`seed_minimal`].
pub struct SeedResult {
    pub workspace_id:   Uuid,
    pub member_id:      Uuid,
    pub project_id:     Uuid,
    pub task_group_id:  Uuid,
    /// Three tasks in the order they were created.
    pub task_ids:       [Uuid; 3],
}

/// Insert one workspace, one owner member, one active project, one epic task
/// group, and three tasks (todo / in_progress / done) into `pool`.
///
/// Task 0 blocks task 1 via a `task_dependency` row.
///
/// Returns the IDs of every inserted row so callers can assert against them.
pub async fn seed_minimal(pool: &SqlitePool) -> SeedResult {
    let workspace_id  = Uuid::new_v4();
    let member_id     = Uuid::new_v4();
    let project_id    = Uuid::new_v4();
    let task_group_id = Uuid::new_v4();
    let task_ids      = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let dep_id        = Uuid::new_v4();

    // workspace
    sqlx::query(
        "INSERT INTO workspaces (id, slug, name) VALUES (?, ?, ?)",
    )
    .bind(workspace_id.to_string())
    .bind(WORKSPACE_SLUG)
    .bind(WORKSPACE_NAME)
    .execute(pool)
    .await
    .expect("insert workspace");

    // member (owner)
    sqlx::query(
        "INSERT INTO workspace_members \
         (id, workspace_id, external_subject, display_name, role, status) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(member_id.to_string())
    .bind(workspace_id.to_string())
    .bind("dev:owner-1")
    .bind("Dev Owner")
    .bind("owner")
    .bind("active")
    .execute(pool)
    .await
    .expect("insert workspace_member");

    // project
    sqlx::query(
        "INSERT INTO projects (id, workspace_id, slug, name, status) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(project_id.to_string())
    .bind(workspace_id.to_string())
    .bind(PROJECT_SLUG)
    .bind(PROJECT_NAME)
    .bind("active")
    .execute(pool)
    .await
    .expect("insert project");

    // task_group (epic)
    sqlx::query(
        "INSERT INTO task_groups \
         (id, workspace_id, project_id, kind, title, status, priority) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(task_group_id.to_string())
    .bind(workspace_id.to_string())
    .bind(project_id.to_string())
    .bind("epic")
    .bind(TASK_GROUP_TITLE)
    .bind("active")
    .bind(0i32)
    .execute(pool)
    .await
    .expect("insert task_group");

    // tasks
    let task_fixtures = [
        (task_ids[0], "rank-a", "Set up repository", "todo",        "high"),
        (task_ids[1], "rank-b", "Write initial tests",  "in_progress", "normal"),
        (task_ids[2], "rank-c", "Deploy to staging",   "done",        "normal"),
    ];

    for (id, rank_key, title, status, priority) in &task_fixtures {
        sqlx::query(
            "INSERT INTO tasks \
             (id, workspace_id, project_id, group_id, rank_key, title, status, priority) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(workspace_id.to_string())
        .bind(project_id.to_string())
        .bind(task_group_id.to_string())
        .bind(*rank_key)
        .bind(*title)
        .bind(*status)
        .bind(*priority)
        .execute(pool)
        .await
        .expect("insert task");
    }

    // task dependency: task[0] blocks task[1]
    sqlx::query(
        "INSERT INTO task_dependencies \
         (id, workspace_id, project_id, predecessor_task_id, successor_task_id, \
          dependency_type, is_hard_block) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(dep_id.to_string())
    .bind(workspace_id.to_string())
    .bind(project_id.to_string())
    .bind(task_ids[0].to_string())
    .bind(task_ids[1].to_string())
    .bind("blocks")
    .bind(1i32)
    .execute(pool)
    .await
    .expect("insert task_dependency");

    SeedResult {
        workspace_id,
        member_id,
        project_id,
        task_group_id,
        task_ids,
    }
}
