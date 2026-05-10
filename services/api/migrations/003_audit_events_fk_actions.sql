ALTER TABLE audit_events
DROP CONSTRAINT IF EXISTS audit_events_agent_session_id_fkey;

ALTER TABLE audit_events
    ADD CONSTRAINT audit_events_agent_session_id_fkey
        FOREIGN KEY (agent_session_id)
            REFERENCES agent_sessions (id)
            ON DELETE SET NULL;

ALTER TABLE audit_events
DROP CONSTRAINT IF EXISTS audit_events_project_id_fkey;

ALTER TABLE audit_events
    ADD CONSTRAINT audit_events_project_id_fkey
        FOREIGN KEY (project_id)
            REFERENCES projects (id)
            ON DELETE CASCADE;

ALTER TABLE audit_events
DROP CONSTRAINT IF EXISTS audit_events_workspace_id_fkey;

ALTER TABLE audit_events
    ADD CONSTRAINT audit_events_workspace_id_fkey
        FOREIGN KEY (workspace_id)
            REFERENCES workspaces (id)
            ON DELETE CASCADE;