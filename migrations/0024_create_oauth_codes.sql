-- Plan 45: the one-time code a host's consent hands the client, traded once
-- for an access token. Only its hash is kept.
CREATE TABLE oauth_codes (
    -- sha256 of the code, hex.
    code_hash          TEXT PRIMARY KEY,
    oauth_client_unid  UUID NOT NULL REFERENCES oauth_clients(unid) ON DELETE CASCADE,
    user_unid          UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    redirect_uri       TEXT NOT NULL,
    -- PKCE S256: base64url(sha256(code_verifier)).
    code_challenge     TEXT NOT NULL,
    scope              TEXT NOT NULL,
    expires_at         TIMESTAMPTZ NOT NULL,
    CONSTRAINT oauth_codes_scope CHECK (scope IN ('full', 'read'))
);
