CREATE TABLE calling_visits (
    unid                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_page             TEXT,
    referer                 TEXT,
    ip_address              INET,
    user_agent              TEXT,
    accept_language         TEXT,
    is_mobile               BOOLEAN,
    country                 TEXT,
    city                    TEXT,
    utm_source              TEXT,
    utm_medium              TEXT,
    utm_campaign            TEXT,
    duration_seconds        INT,
    form_started            BOOLEAN NOT NULL DEFAULT false,
    form_submitted          BOOLEAN NOT NULL DEFAULT false,
    booked_call             BOOLEAN NOT NULL DEFAULT false,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX calling_visits_created_at_idx ON calling_visits (created_at DESC);
CREATE INDEX calling_visits_ip_idx ON calling_visits (ip_address);
