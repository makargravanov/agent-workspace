-- ============================================================
-- 004_members_access.sql  (SQLite)
-- Workspace invitations and human project access.
-- ============================================================

CREATE TABLE workspace_invites (
    id                   TEXT    NOT NULL PRIMARY KEY,
    workspace_id          TEXT    NOT NULL REFERENCES workspaces (id),
    github_login          TEXT,
    token_hash            TEXT    NOT NULL UNIQUE,
    role                  TEXT    NOT NULL CHECK (role IN ('editor', 'viewer')),
    project_access_json   TEXT    NOT NULL DEFAULT '[]',
    status                TEXT    NOT NULL CHECK (status IN ('pending', 'accepted', 'revoked', 'expired')),
    expires_at            TEXT,
    created_by_member_id  TEXT    NOT NULL REFERENCES workspace_members (id),
    accepted_by_member_id TEXT    REFERENCES workspace_members (id),
    accepted_at           TEXT,
    created_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_workspace_invites_ws_status
    ON workspace_invites (workspace_id, status, created_at);
CREATE INDEX idx_workspace_invites_github_login
    ON workspace_invites (github_login, status);

CREATE TABLE project_members (
    id                  TEXT    NOT NULL PRIMARY KEY,
    workspace_id        TEXT    NOT NULL REFERENCES workspaces (id),
    project_id          TEXT    NOT NULL REFERENCES projects (id),
    workspace_member_id TEXT    NOT NULL REFERENCES workspace_members (id),
    role                TEXT    NOT NULL CHECK (role IN ('editor', 'viewer')),
    status              TEXT    NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_project_members_member UNIQUE (project_id, workspace_member_id)
);

CREATE INDEX idx_project_members_project_role_status
    ON project_members (project_id, role, status);
CREATE INDEX idx_project_members_member_status
    ON project_members (workspace_member_id, status);

INSERT OR IGNORE INTO project_members (
    id,
    workspace_id,
    project_id,
    workspace_member_id,
    role,
    status
)
SELECT
    lower(hex(randomblob(16))),
    p.workspace_id,
    p.id,
    wm.id,
    wm.role,
    'active'
FROM projects p
JOIN workspace_members wm ON wm.workspace_id = p.workspace_id
WHERE wm.status = 'active'
  AND wm.role IN ('editor', 'viewer');
