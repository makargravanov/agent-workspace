-- ============================================================
-- 002_auth_sessions.sql  (PostgreSQL)
-- Human identity and opaque server-side session tables.
-- ============================================================

CREATE TABLE human_identities (
    id                  UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    workspace_member_id UUID        NOT NULL REFERENCES workspace_members (id),
    provider            TEXT        NOT NULL CHECK (provider IN ('dev', 'github')),
    provider_subject    TEXT        NOT NULL,
    display_name        TEXT        NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_human_identities_provider_subject UNIQUE (provider, provider_subject)
);

CREATE INDEX idx_human_identities_member
    ON human_identities (workspace_member_id);

CREATE TABLE human_sessions (
    id                  UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    workspace_member_id UUID        NOT NULL REFERENCES workspace_members (id),
    token_hash          TEXT        NOT NULL UNIQUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL,
    revoked_at          TIMESTAMPTZ,
    last_seen_at        TIMESTAMPTZ,
    user_agent          TEXT,
    ip_address          TEXT
);

CREATE INDEX idx_human_sessions_member_expires
    ON human_sessions (workspace_member_id, expires_at);
CREATE INDEX idx_human_sessions_active
    ON human_sessions (token_hash, expires_at)
    WHERE revoked_at IS NULL;
