use serde::Serialize;

use super::actor::ActorContext;
use crate::db::DatabaseBackend;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub request_id: String,
    pub actor: ActorContext,
    pub action: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub payload: Option<serde_json::Value>,
}

// TODO BL-50: persist to audit_events table
pub fn emit_audit(event: AuditEvent) {
    tracing::info!(
        request_id = %event.request_id,
        actor_kind = ?event.actor.actor_kind,
        actor_id = %event.actor.actor_id,
        action = %event.action,
        resource_kind = %event.resource_kind,
        resource_id = %event.resource_id,
        payload = ?event.payload,
        "audit_event",
    );
}

async fn resolve_resource_scope(
    pool: &sqlx::AnyPool,
    event: &AuditEvent,
) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    if event.resource_kind == "workspace" {
        return Ok(Some((event.resource_id.clone(), None)));
    }

    let sql = match event.resource_kind.as_str() {
        "workspace_member" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(NULL AS TEXT)
             FROM workspace_members
             WHERE CAST(id AS TEXT) = $1"
        }
        "project" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(id AS TEXT)
             FROM projects
             WHERE CAST(id AS TEXT) = $1"
        }
        "task_group" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM task_groups
             WHERE CAST(id AS TEXT) = $1"
        }
        "task" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM tasks
             WHERE CAST(id AS TEXT) = $1"
        }
        "task_dependency" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM task_dependencies
             WHERE CAST(id AS TEXT) = $1"
        }
        "document" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM documents
             WHERE CAST(id AS TEXT) = $1"
        }
        "asset" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM assets
             WHERE CAST(id AS TEXT) = $1"
        }
        "agent" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(NULL AS TEXT)
             FROM agents
             WHERE CAST(id AS TEXT) = $1"
        }
        "agent_credential" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM agent_credentials
             WHERE CAST(id AS TEXT) = $1"
        }
        "agent_session" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM agent_sessions
             WHERE CAST(id AS TEXT) = $1"
        }
        "agent_session_task" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM agent_session_tasks
             WHERE CAST(id AS TEXT) = $1"
        }
        "note" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM notes
             WHERE CAST(id AS TEXT) = $1"
        }
        "link" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM links
             WHERE CAST(id AS TEXT) = $1"
        }
        "integration_connection" => {
            "SELECT CAST(workspace_id AS TEXT), CAST(project_id AS TEXT)
             FROM integration_connections
             WHERE CAST(id AS TEXT) = $1"
        }
        _ => return Ok(None),
    };

    sqlx::query_as::<_, (String, Option<String>)>(sql)
        .bind(&event.resource_id)
        .fetch_optional(pool)
        .await
}

pub async fn record_audit(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    event: AuditEvent,
) -> Result<(), sqlx::Error> {
    emit_audit(event.clone());

    let (workspace_id, project_id) = match event.actor.workspace_id.clone() {
        Some(workspace_id) => (workspace_id, event.actor.project_id.clone()),
        None => {
            let Some((workspace_id, project_id)) = resolve_resource_scope(pool, &event).await? else {
                return Ok(());
            };
            (workspace_id, project_id)
        }
    };

    let actor_id = if event.actor.actor_id == "anonymous" {
        None
    } else {
        Some(event.actor.actor_id.clone())
    };
    let entity_id = Some(event.resource_id.clone());
    let payload_json = event
        .payload
        .as_ref()
        .map(|value| serde_json::to_string(value))
        .transpose()
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize audit payload: {e}")))?;

    let sql = match db_backend {
        DatabaseBackend::Postgres => {
            "INSERT INTO audit_events
             (workspace_id, project_id, actor_type, actor_id, entity_type, entity_id, event_type, payload_json)
             VALUES (
                CAST($1 AS UUID),
                CAST($2 AS UUID),
                $3,
                CAST($4 AS UUID),
                $5,
                CAST($6 AS UUID),
                $7,
                CAST($8 AS JSONB)
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO audit_events
             (workspace_id, project_id, actor_type, actor_id, entity_type, entity_id, event_type, payload_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        }
    };

    sqlx::query(sql)
        .bind(&workspace_id)
        .bind(project_id.as_deref())
        .bind(match event.actor.actor_kind {
            super::actor::ActorKind::Human => "workspace_member",
            super::actor::ActorKind::Agent => "agent",
            super::actor::ActorKind::System => "integration",
        })
        .bind(actor_id.as_deref())
        .bind(event.resource_kind)
        .bind(entity_id.as_deref())
        .bind(event.action)
        .bind(payload_json.as_deref())
        .execute(pool)
        .await?;

    Ok(())
}
