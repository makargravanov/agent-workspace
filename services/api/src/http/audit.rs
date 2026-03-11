use serde::Serialize;

use super::actor::ActorContext;

#[derive(Debug, Serialize)]
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
