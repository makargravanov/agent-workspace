-- ============================================================
-- 004_members_access.sql  (PostgreSQL)
-- Workspace invitations and human project access.
-- ============================================================

CREATE TABLE workspace_invites (
    id                   UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    workspace_id          UUID        NOT NULL REFERENCES workspaces (id),
    github_login          TEXT,
    token_hash            TEXT        NOT NULL UNIQUE,
    role                  TEXT        NOT NULL CHECK (role IN ('editor', 'viewer')),
    project_access_json   JSONB       NOT NULL DEFAULT '[]',
    status                TEXT        NOT NULL CHECK (status IN ('pending', 'accepted', 'revoked', 'expired')),
    expires_at            TIMESTAMPTZ,
    created_by_member_id  UUID        NOT NULL REFERENCES workspace_members (id),
    accepted_by_member_id UUID        REFERENCES workspace_members (id),
    accepted_at           TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workspace_invites_ws_status
    ON workspace_invites (workspace_id, status, created_at);
CREATE INDEX idx_workspace_invites_github_login
    ON workspace_invites (lower(github_login), status);

CREATE TABLE project_members (
    id                  UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    workspace_id        UUID        NOT NULL REFERENCES workspaces (id),
    project_id          UUID        NOT NULL REFERENCES projects (id),
    workspace_member_id UUID        NOT NULL REFERENCES workspace_members (id),
    role                TEXT        NOT NULL CHECK (role IN ('editor', 'viewer')),
    status              TEXT        NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_project_members_member UNIQUE (project_id, workspace_member_id)
);

CREATE INDEX idx_project_members_project_role_status
    ON project_members (project_id, role, status);
CREATE INDEX idx_project_members_member_status
    ON project_members (workspace_member_id, status);

INSERT INTO project_members (
    id,
    workspace_id,
    project_id,
    workspace_member_id,
    role,
    status
)
SELECT
    gen_random_uuid(),
    p.workspace_id,
    p.id,
    wm.id,
    wm.role,
    'active'
FROM projects p
JOIN workspace_members wm ON wm.workspace_id = p.workspace_id
WHERE wm.status = 'active'
  AND wm.role IN ('editor', 'viewer')
ON CONFLICT (project_id, workspace_member_id) DO NOTHING;
