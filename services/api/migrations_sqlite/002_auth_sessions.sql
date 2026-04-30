-- ============================================================
-- 002_auth_sessions.sql  (SQLite)
-- Human identity and opaque server-side session tables.
-- ============================================================

CREATE TABLE human_identities (
    id                  TEXT    NOT NULL PRIMARY KEY,
    workspace_member_id TEXT    NOT NULL REFERENCES workspace_members (id),
    provider            TEXT    NOT NULL CHECK (provider IN ('dev', 'github')),
    provider_subject    TEXT    NOT NULL,
    display_name        TEXT    NOT NULL,
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT uq_human_identities_provider_subject UNIQUE (provider, provider_subject)
);

CREATE INDEX idx_human_identities_member
    ON human_identities (workspace_member_id);

CREATE TABLE human_sessions (
    id                  TEXT    NOT NULL PRIMARY KEY,
    workspace_member_id TEXT    NOT NULL REFERENCES workspace_members (id),
    token_hash          TEXT    NOT NULL UNIQUE,
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at          TEXT    NOT NULL,
    revoked_at          TEXT,
    last_seen_at        TEXT,
    user_agent          TEXT,
    ip_address          TEXT
);

CREATE INDEX idx_human_sessions_member_expires
    ON human_sessions (workspace_member_id, expires_at);
CREATE INDEX idx_human_sessions_active
    ON human_sessions (token_hash, expires_at)
    WHERE revoked_at IS NULL;
