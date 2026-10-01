CREATE TABLE users (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    full_name       TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'Active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT users_status CHECK (status IN ('Active', 'Banned')),
    CONSTRAINT users_email_not_blank CHECK (length(trim(email)) > 3)
);

CREATE INDEX users_email_lower_idx ON users (lower(email));
