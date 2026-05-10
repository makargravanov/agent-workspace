
PRAGMA foreign_keys = OFF;

CREATE TABLE audit_events_new (
                                  id               TEXT    NOT NULL PRIMARY KEY,
                                  workspace_id     TEXT    NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
                                  project_id       TEXT    REFERENCES projects (id) ON DELETE CASCADE,
                                  agent_session_id TEXT    REFERENCES agent_sessions (id) ON DELETE SET NULL,
                                  actor_type       TEXT    NOT NULL
                                      CHECK (actor_type IN ('workspace_member', 'agent', 'integration')),
                                  actor_id         TEXT,
                                  entity_type      TEXT    NOT NULL,
                                  entity_id        TEXT,
                                  event_type       TEXT    NOT NULL,
                                  payload_json     TEXT,
                                  occurred_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO audit_events_new (
    id,
    workspace_id,
    project_id,
    agent_session_id,
    actor_type,
    actor_id,
    entity_type,
    entity_id,
    event_type,
    payload_json,
    occurred_at
)
SELECT
    id,
    workspace_id,
    project_id,
    agent_session_id,
    actor_type,
    actor_id,
    entity_type,
    entity_id,
    event_type,
    payload_json,
    occurred_at
FROM audit_events;

DROP TABLE audit_events;

ALTER TABLE audit_events_new RENAME TO audit_events;

CREATE INDEX idx_audit_events_workspace
    ON audit_events (workspace_id, occurred_at);
CREATE INDEX idx_audit_events_project
    ON audit_events (project_id, occurred_at);
CREATE INDEX idx_audit_events_entity
    ON audit_events (entity_type, entity_id, occurred_at);
CREATE INDEX idx_audit_events_actor
    ON audit_events (actor_type, actor_id, occurred_at);

PRAGMA foreign_keys = ON;