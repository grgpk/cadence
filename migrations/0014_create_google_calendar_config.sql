CREATE TABLE google_calendar_config (
    host_unid       UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
    access_token    TEXT NOT NULL,
    refresh_token   TEXT NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    calendar_id     TEXT NOT NULL DEFAULT 'primary',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
