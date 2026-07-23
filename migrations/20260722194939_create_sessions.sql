CREATE TABLE sessions (
    token               TEXT PRIMARY KEY,
    host_id             BIGINT NOT NULL REFERENCES hosts(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);