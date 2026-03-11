//! `seed` — dev-only binary that seeds a PostgreSQL database with a minimal
//! representative fixture set.
//!
//! Run with:
//!   DATABASE_URL=postgres://... cargo run --bin seed
//!
//! The seed is **idempotent**: all inserts use `ON CONFLICT DO NOTHING`, so
//! running the binary multiple times against the same database is safe.
//!
//! Deterministic (hard-coded) UUIDs are used so the fixture rows have stable,
//! predictable IDs across runs and environments.

use agent_workspace_api::{
    db::{build_pool, DatabaseConfig},
    telemetry::init_tracing,
};
use tracing::info;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Deterministic fixture IDs
// Encoded in UUIDv4 format: xxxxxxxx-xxxx-4xxx-8xxx-xxxxxxxxxxxx
// ---------------------------------------------------------------------------
const WORKSPACE_ID:   &str = "00000001-0000-4000-8000-000000000001";
const MEMBER_ID:      &str = "00000001-0000-4000-8000-000000000002";
const PROJECT_ID:     &str = "00000001-0000-4000-8000-000000000003";
const TASK_GROUP_ID:  &str = "00000001-0000-4000-8000-000000000004";
const TASK_IDS: [&str; 3] = [
    "00000001-0000-4000-8000-000000000011",
    "00000001-0000-4000-8000-000000000012",
    "00000001-0000-4000-8000-000000000013",
];
const DEP_ID: &str = "00000001-0000-4000-8000-000000000021";

#[tokio::main]
async fn main() {
    init_tracing();

    let db_cfg = DatabaseConfig::from_env()
        .expect("DATABASE_URL must be set to run the seed binary");

    let pool = build_pool(&db_cfg)
        .await
        .expect("failed to connect to database");

    // Run pending migrations before seeding so the schema is always up to date.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run database migrations");

    info!("migrations applied; inserting fixture data");

    let workspace_id  = Uuid::parse_str(WORKSPACE_ID).unwrap();
    let member_id     = Uuid::parse_str(MEMBER_ID).unwrap();
    let project_id    = Uuid::parse_str(PROJECT_ID).unwrap();
    let task_group_id = Uuid::parse_str(TASK_GROUP_ID).unwrap();
    let task_uuids: Vec<Uuid> = TASK_IDS
        .iter()
        .map(|s| Uuid::parse_str(s).unwrap())
        .collect();
    let dep_id = Uuid::parse_str(DEP_ID).unwrap();

    // workspace
    sqlx::query(
        "INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(workspace_id)
    .bind("dev-workspace")
    .bind("Dev Workspace")
    .execute(&pool)
    .await
    .expect("insert workspace");

    // member (owner)
    sqlx::query(
        "INSERT INTO workspace_members \
         (id, workspace_id, external_subject, display_name, role, status) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
    )
    .bind(member_id)
    .bind(workspace_id)
    .bind("dev:owner-1")
    .bind("Dev Owner")
    .bind("owner")
    .bind("active")
    .execute(&pool)
    .await
    .expect("insert workspace_member");

    // project
    sqlx::query(
        "INSERT INTO projects (id, workspace_id, slug, name, status) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(project_id)
    .bind(workspace_id)
    .bind("main-project")
    .bind("Main Project")
    .bind("active")
    .execute(&pool)
    .await
    .expect("insert project");

    // task_group
    sqlx::query(
        "INSERT INTO task_groups \
         (id, workspace_id, project_id, kind, title, status, priority) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
    )
    .bind(task_group_id)
    .bind(workspace_id)
    .bind(project_id)
    .bind("epic")
    .bind("First Epic")
    .bind("active")
    .bind(0_i32)
    .execute(&pool)
    .await
    .expect("insert task_group");

    // tasks
    let task_fixtures: &[(&Uuid, &str, &str, &str, &str)] = &[
        (&task_uuids[0], "rank-a", "Set up repository",  "todo",        "high"),
        (&task_uuids[1], "rank-b", "Write initial tests", "in_progress", "normal"),
        (&task_uuids[2], "rank-c", "Deploy to staging",  "done",        "normal"),
    ];

    for (id, rank_key, title, status, priority) in task_fixtures {
        sqlx::query(
            "INSERT INTO tasks \
             (id, workspace_id, project_id, group_id, rank_key, title, status, priority) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
        )
        .bind(*id)
        .bind(workspace_id)
        .bind(project_id)
        .bind(task_group_id)
        .bind(*rank_key)
        .bind(*title)
        .bind(*status)
        .bind(*priority)
        .execute(&pool)
        .await
        .expect("insert task");
    }

    // task dependency: task[0] blocks task[1]
    sqlx::query(
        "INSERT INTO task_dependencies \
         (id, workspace_id, project_id, predecessor_task_id, successor_task_id, \
          dependency_type, is_hard_block) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
    )
    .bind(dep_id)
    .bind(workspace_id)
    .bind(project_id)
    .bind(task_uuids[0])
    .bind(task_uuids[1])
    .bind("blocks")
    .bind(true)
    .execute(&pool)
    .await
    .expect("insert task_dependency");

    info!(
        workspace_id = %workspace_id,
        project_id   = %project_id,
        "dev fixture data seeded successfully"
    );
}
