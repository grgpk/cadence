-- One row per "forgot password" email. Only a SHA-256 of the token is kept,
-- so a copy of this table cannot be used to take over an account.
CREATE TABLE password_resets (
    token_hash  TEXT PRIMARY KEY,
    user_unid   UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX password_resets_user_idx ON password_resets (user_unid);
