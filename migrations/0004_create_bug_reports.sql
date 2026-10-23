CREATE TABLE bug_reports (
    unid              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bug_type          TEXT NOT NULL,
    similarity_hash   INTEGER NOT NULL,
    message           TEXT,
    exception_message TEXT,
    stack_trace       TEXT,
    user_login        TEXT,
    url               TEXT,
    user_agent        TEXT,
    application       TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX bug_reports_created_idx ON bug_reports (created_at DESC);
CREATE INDEX bug_reports_type_idx ON bug_reports (bug_type);
CREATE INDEX bug_reports_similarity_idx ON bug_reports (similarity_hash);
