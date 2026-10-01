CREATE TABLE oauth_states (
    state           TEXT PRIMARY KEY,
    user_unid       UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    provider        TEXT NOT NULL,
    redirect_uri    TEXT NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    consumed_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX oauth_states_expiry_idx ON oauth_states (expires_at);
