ALTER TABLE workspace_invites
    ADD COLUMN invite_token TEXT;

CREATE UNIQUE INDEX idx_workspace_invites_invite_token
    ON workspace_invites (invite_token);
