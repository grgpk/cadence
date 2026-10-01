CREATE TABLE leads (
    unid                    UUID PRIMARY KEY,
    host_unid               UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    first_name              TEXT,
    last_name               TEXT,
    email                   TEXT,
    phone                   TEXT,
    country_code            TEXT,
    investment_comfort      TEXT,
    what_stopping_you       TEXT,
    source_page             TEXT,
    utm_source              TEXT,
    utm_medium              TEXT,
    utm_campaign            TEXT,
    qualification_status    TEXT,
    form_submitted_at       TIMESTAMPTZ,
    ip_address              INET,
    user_agent              TEXT,
    CONSTRAINT leads_qualification_status CHECK (
        qualification_status IS NULL
        OR qualification_status IN ('Qualified', 'NotSure', 'Disqualified')
    )
);

CREATE INDEX leads_host_updated_idx ON leads (host_unid, updated_at DESC);
CREATE INDEX leads_submitted_idx ON leads (form_submitted_at);
