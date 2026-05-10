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

pub async fn record_audit(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    event: AuditEvent,
) -> Result<(), sqlx::Error> {
    emit_audit(event.clone());

    let Some(workspace_id) = event.actor.workspace_id.as_deref() else {
        return Ok(());
    };

    let project_id = event.actor.project_id.clone();
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
                $8
             )"
        }
        DatabaseBackend::Sqlite => {
            "INSERT INTO audit_events
             (workspace_id, project_id, actor_type, actor_id, entity_type, entity_id, event_type, payload_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        }
    };

    sqlx::query(sql)
        .bind(workspace_id)
        .bind(project_id.as_deref())
        .bind(match event.actor.actor_kind {
            super::actor::ActorKind::Human => "human",
            super::actor::ActorKind::Agent => "agent",
            super::actor::ActorKind::System => "system",
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
