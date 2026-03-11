-- ============================================================
-- 001_initial_schema.sql  (SQLite)
-- SQLite-compatible schema for local/dev and smoke-test use.
-- UUIDs stored as TEXT, timestamps as TEXT (ISO-8601/RFC-3339),
-- booleans as INTEGER (0/1), JSON as TEXT.
-- Foreign keys are declared but require  PRAGMA foreign_keys = ON
-- per-connection to be enforced at runtime.
-- Canonical source: docs/specification/database-schema.md §6
-- ============================================================

-- ------------------------------------------------------------
-- workspaces
-- ------------------------------------------------------------
CREATE TABLE workspaces (
    id         TEXT    NOT NULL PRIMARY KEY,
    slug       TEXT    NOT NULL UNIQUE,
    name       TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ------------------------------------------------------------
-- workspace_members
-- ------------------------------------------------------------
CREATE TABLE workspace_members (
    id               TEXT    NOT NULL PRIMARY KEY,
    workspace_id     TEXT    NOT NULL REFERENCES workspaces (id),
    external_subject TEXT    NOT NULL,
    display_name     TEXT    NOT NULL,
    role             TEXT    NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    status           TEXT    NOT NULL CHECK (status IN ('active', 'invited', 'disabled')),
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_workspace_members_subject UNIQUE (workspace_id, external_subject)
);

CREATE INDEX idx_workspace_members_ws_role_status
    ON workspace_members (workspace_id, role, status);

-- ------------------------------------------------------------
-- projects
-- ------------------------------------------------------------
CREATE TABLE projects (
    id           TEXT    NOT NULL PRIMARY KEY,
    workspace_id TEXT    NOT NULL REFERENCES workspaces (id),
    slug         TEXT    NOT NULL,
    name         TEXT    NOT NULL,
    status       TEXT    NOT NULL CHECK (status IN ('active', 'on_hold', 'archived')),
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_projects_workspace_slug UNIQUE (workspace_id, slug)
);

CREATE INDEX idx_projects_workspace_status ON projects (workspace_id, status);

-- ------------------------------------------------------------
-- task_groups
-- ------------------------------------------------------------
CREATE TABLE task_groups (
    id             TEXT    NOT NULL PRIMARY KEY,
    workspace_id   TEXT    NOT NULL REFERENCES workspaces (id),
    project_id     TEXT    NOT NULL REFERENCES projects (id),
    kind           TEXT    NOT NULL CHECK (kind IN ('initiative', 'epic')),
    title          TEXT    NOT NULL,
    description_md TEXT,
    status         TEXT    NOT NULL CHECK (status IN ('draft', 'active', 'done', 'archived')),
    priority       INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_task_groups_project_status
    ON task_groups (project_id, status);
CREATE INDEX idx_task_groups_project_kind_priority
    ON task_groups (project_id, kind, priority);

-- ------------------------------------------------------------
-- tasks
-- ------------------------------------------------------------
CREATE TABLE tasks (
    id             TEXT    NOT NULL PRIMARY KEY,
    workspace_id   TEXT    NOT NULL REFERENCES workspaces (id),
    project_id     TEXT    NOT NULL REFERENCES projects (id),
    group_id       TEXT    REFERENCES task_groups (id),
    parent_task_id TEXT    REFERENCES tasks (id),
    rank_key       TEXT    NOT NULL,
    starts_at      TEXT,
    due_at         TEXT,
    assignee_type  TEXT    CHECK (assignee_type IN ('workspace_member', 'agent')),
    assignee_id    TEXT,
    title          TEXT    NOT NULL,
    description_md TEXT,
    status         TEXT    NOT NULL CHECK (status IN ('todo', 'in_progress', 'done', 'cancelled')),
    priority       TEXT    NOT NULL CHECK (priority IN ('low', 'normal', 'high', 'critical')),
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_tasks_project_status_priority
    ON tasks (project_id, status, priority);
CREATE INDEX idx_tasks_project_group_rank
    ON tasks (project_id, group_id, rank_key);
CREATE INDEX idx_tasks_project_assignee
    ON tasks (project_id, assignee_type, assignee_id);
CREATE INDEX idx_tasks_project_parent
    ON tasks (project_id, parent_task_id);

-- ------------------------------------------------------------
-- task_dependencies
-- ------------------------------------------------------------
CREATE TABLE task_dependencies (
    id                  TEXT    NOT NULL PRIMARY KEY,
    workspace_id        TEXT    NOT NULL REFERENCES workspaces (id),
    project_id          TEXT    NOT NULL REFERENCES projects (id),
    predecessor_task_id TEXT    NOT NULL REFERENCES tasks (id),
    successor_task_id   TEXT    NOT NULL REFERENCES tasks (id),
    dependency_type     TEXT    NOT NULL CHECK (dependency_type IN ('blocks')),
    is_hard_block       INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_task_dependencies
        UNIQUE (project_id, predecessor_task_id, successor_task_id, dependency_type),
    CONSTRAINT chk_task_dep_no_self_loop
        CHECK (predecessor_task_id <> successor_task_id)
);

CREATE INDEX idx_task_deps_project_successor
    ON task_dependencies (project_id, successor_task_id);
CREATE INDEX idx_task_deps_project_predecessor
    ON task_dependencies (project_id, predecessor_task_id);

-- ------------------------------------------------------------
-- documents
-- ------------------------------------------------------------
CREATE TABLE documents (
    id                 TEXT    NOT NULL PRIMARY KEY,
    workspace_id       TEXT    NOT NULL REFERENCES workspaces (id),
    project_id         TEXT    NOT NULL REFERENCES projects (id),
    parent_document_id TEXT    REFERENCES documents (id),
    slug               TEXT    NOT NULL,
    title              TEXT    NOT NULL,
    body_format        TEXT    NOT NULL CHECK (body_format IN ('markdown')),
    body_md            TEXT    NOT NULL,
    status             TEXT    NOT NULL CHECK (status IN ('draft', 'published', 'archived')),
    version            INTEGER NOT NULL DEFAULT 1,
    created_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_documents_parent_slug
        UNIQUE (project_id, parent_document_id, slug)
);

CREATE INDEX idx_documents_project_status
    ON documents (project_id, status);
CREATE INDEX idx_documents_project_parent
    ON documents (project_id, parent_document_id);

-- ------------------------------------------------------------
-- assets
-- ------------------------------------------------------------
CREATE TABLE assets (
    id                    TEXT    NOT NULL PRIMARY KEY,
    workspace_id          TEXT    NOT NULL REFERENCES workspaces (id),
    project_id            TEXT    NOT NULL REFERENCES projects (id),
    uploaded_by_member_id TEXT    REFERENCES workspace_members (id),
    file_name             TEXT    NOT NULL,
    media_type            TEXT    NOT NULL,
    size_bytes            INTEGER NOT NULL,
    sha256                TEXT,
    storage_backend       TEXT    NOT NULL CHECK (storage_backend IN ('local', 's3')),
    storage_key           TEXT    NOT NULL,
    created_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_assets_project_file_name  ON assets (project_id, file_name);
CREATE INDEX idx_assets_project_media_type ON assets (project_id, media_type);
CREATE INDEX idx_assets_project_created_at ON assets (project_id, created_at);
CREATE INDEX idx_assets_project_sha256     ON assets (project_id, sha256);

-- ------------------------------------------------------------
-- agents
-- ------------------------------------------------------------
CREATE TABLE agents (
    id                   TEXT    NOT NULL PRIMARY KEY,
    workspace_id         TEXT    NOT NULL REFERENCES workspaces (id),
    created_by_member_id TEXT    NOT NULL REFERENCES workspace_members (id),
    key                  TEXT    NOT NULL,
    display_name         TEXT    NOT NULL,
    status               TEXT    NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at           TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_agents_workspace_key UNIQUE (workspace_id, key)
);

CREATE INDEX idx_agents_workspace_status ON agents (workspace_id, status);

-- ------------------------------------------------------------
-- agent_credentials
-- ------------------------------------------------------------
CREATE TABLE agent_credentials (
    id                  TEXT    NOT NULL PRIMARY KEY,
    workspace_id        TEXT    NOT NULL REFERENCES workspaces (id),
    project_id          TEXT    REFERENCES projects (id),
    agent_id            TEXT    NOT NULL REFERENCES agents (id),
    issued_by_member_id TEXT    NOT NULL REFERENCES workspace_members (id),
    label               TEXT    NOT NULL,
    secret_prefix       TEXT    NOT NULL,
    secret_hash         TEXT    NOT NULL,
    scope_policy        TEXT    NOT NULL DEFAULT '[]',
    status              TEXT    NOT NULL CHECK (status IN ('active', 'revoked')),
    expires_at          TEXT,
    last_used_at        TEXT,
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    revoked_at          TEXT,
    CONSTRAINT uq_agent_credentials_prefix UNIQUE (workspace_id, secret_prefix)
);

CREATE INDEX idx_agent_credentials_agent_status
    ON agent_credentials (agent_id, status);
CREATE INDEX idx_agent_credentials_ws_proj_status
    ON agent_credentials (workspace_id, project_id, status);
CREATE INDEX idx_agent_credentials_ws_expires
    ON agent_credentials (workspace_id, expires_at);

-- ------------------------------------------------------------
-- agent_sessions
-- ------------------------------------------------------------
CREATE TABLE agent_sessions (
    id           TEXT    NOT NULL PRIMARY KEY,
    workspace_id TEXT    NOT NULL REFERENCES workspaces (id),
    project_id   TEXT    NOT NULL REFERENCES projects (id),
    agent_id     TEXT    NOT NULL REFERENCES agents (id),
    status       TEXT    NOT NULL
        CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    started_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    finished_at  TEXT,
    summary_text TEXT
);

CREATE INDEX idx_agent_sessions_project_status
    ON agent_sessions (project_id, status, started_at);
CREATE INDEX idx_agent_sessions_agent
    ON agent_sessions (agent_id, started_at);

-- ------------------------------------------------------------
-- agent_session_tasks
-- ------------------------------------------------------------
CREATE TABLE agent_session_tasks (
    id               TEXT    NOT NULL PRIMARY KEY,
    workspace_id     TEXT    NOT NULL REFERENCES workspaces (id),
    project_id       TEXT    NOT NULL REFERENCES projects (id),
    agent_session_id TEXT    NOT NULL REFERENCES agent_sessions (id),
    task_id          TEXT    NOT NULL REFERENCES tasks (id),
    relation_type    TEXT    NOT NULL
        CHECK (relation_type IN ('primary_context', 'touched', 'created', 'updated')),
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_agent_session_tasks
        UNIQUE (agent_session_id, task_id, relation_type)
);

CREATE INDEX idx_agent_session_tasks_project_task
    ON agent_session_tasks (project_id, task_id);
CREATE INDEX idx_agent_session_tasks_project_session
    ON agent_session_tasks (project_id, agent_session_id);

-- ------------------------------------------------------------
-- notes
-- ------------------------------------------------------------
CREATE TABLE notes (
    id               TEXT    NOT NULL PRIMARY KEY,
    workspace_id     TEXT    NOT NULL REFERENCES workspaces (id),
    project_id       TEXT    NOT NULL REFERENCES projects (id),
    agent_session_id TEXT    REFERENCES agent_sessions (id),
    kind             TEXT    NOT NULL
        CHECK (kind IN ('context', 'worklog', 'decision', 'result')),
    author_type      TEXT    NOT NULL
        CHECK (author_type IN ('workspace_member', 'agent', 'integration')),
    author_id        TEXT    NOT NULL,
    title            TEXT,
    body_md          TEXT    NOT NULL,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_notes_project_kind
    ON notes (project_id, kind, created_at);
CREATE INDEX idx_notes_project_author
    ON notes (project_id, author_type, author_id);
CREATE INDEX idx_notes_project_session
    ON notes (project_id, agent_session_id);

-- ------------------------------------------------------------
-- links
-- ------------------------------------------------------------
CREATE TABLE links (
    id           TEXT    NOT NULL PRIMARY KEY,
    workspace_id TEXT    NOT NULL REFERENCES workspaces (id),
    project_id   TEXT    NOT NULL REFERENCES projects (id),
    source_type  TEXT    NOT NULL,
    source_id    TEXT    NOT NULL,
    target_type  TEXT    NOT NULL,
    target_id    TEXT,
    target_url   TEXT,
    label        TEXT,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_links_project_source
    ON links (project_id, source_type, source_id);
CREATE INDEX idx_links_project_target
    ON links (project_id, target_type, target_id);

-- ------------------------------------------------------------
-- integration_connections
-- ------------------------------------------------------------
CREATE TABLE integration_connections (
    id                TEXT    NOT NULL PRIMARY KEY,
    workspace_id      TEXT    NOT NULL REFERENCES workspaces (id),
    project_id        TEXT    REFERENCES projects (id),
    provider          TEXT    NOT NULL CHECK (provider IN ('github')),
    scope_kind        TEXT    NOT NULL CHECK (scope_kind IN ('workspace', 'project')),
    status            TEXT    NOT NULL CHECK (status IN ('active', 'disabled', 'error')),
    config_json       TEXT,
    secret_ciphertext TEXT,
    last_synced_at    TEXT,
    created_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_integration_connections_ws
    ON integration_connections (workspace_id, provider, status);
CREATE INDEX idx_integration_connections_proj
    ON integration_connections (project_id, provider, status);

-- ------------------------------------------------------------
-- audit_events
-- ------------------------------------------------------------
CREATE TABLE audit_events (
    id               TEXT    NOT NULL PRIMARY KEY,
    workspace_id     TEXT    NOT NULL REFERENCES workspaces (id),
    project_id       TEXT    REFERENCES projects (id),
    agent_session_id TEXT    REFERENCES agent_sessions (id),
    actor_type       TEXT    NOT NULL
        CHECK (actor_type IN ('workspace_member', 'agent', 'integration')),
    actor_id         TEXT,
    entity_type      TEXT    NOT NULL,
    entity_id        TEXT,
    event_type       TEXT    NOT NULL,
    payload_json     TEXT,
    occurred_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_audit_events_workspace
    ON audit_events (workspace_id, occurred_at);
CREATE INDEX idx_audit_events_project
    ON audit_events (project_id, occurred_at);
CREATE INDEX idx_audit_events_entity
    ON audit_events (entity_type, entity_id, occurred_at);
CREATE INDEX idx_audit_events_actor
    ON audit_events (actor_type, actor_id, occurred_at);
