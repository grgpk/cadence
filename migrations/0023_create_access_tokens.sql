-- Plan 43: a host's AI agent reaches the MCP server with a token the host
-- made. Only its hash is kept; the token is shown once.
-- Plan 45: a host's OAuth consent makes one too, for the client it names. Such
-- a token runs out after an hour and is renewed with its refresh token.
CREATE TABLE access_tokens (
    unid         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_unid    UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    -- sha256 of the token, hex.
    token_hash   TEXT NOT NULL UNIQUE,
    -- The token's end, so the host can tell their tokens apart.
    last_four    TEXT NOT NULL,
    -- 'full' does everything the host can; 'read' only looks.
    scope        TEXT NOT NULL DEFAULT 'full',
    last_used_at TIMESTAMPTZ,
    oauth_client_unid UUID REFERENCES oauth_clients(unid) ON DELETE CASCADE,
    expires_at   TIMESTAMPTZ,
    -- sha256 of the refresh token, hex. Replaced every time it is used.
    refresh_hash TEXT UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT access_tokens_name CHECK (char_length(name) BETWEEN 1 AND 80),
    CONSTRAINT access_tokens_scope CHECK (scope IN ('full', 'read'))
);

CREATE INDEX access_tokens_user_idx ON access_tokens (user_unid, created_at);
