CREATE TABLE bug_reports (
    id                BIGSERIAL PRIMARY KEY,
    unid              UUID NOT NULL DEFAULT gen_random_uuid(),
    bugtype           TEXT NOT NULL,
    similarityhash    INTEGER NOT NULL,
    message           TEXT,
    exceptionmessage  TEXT,
    stacktrace        TEXT,
    userlogin         TEXT,
    url               TEXT,
    useragent         TEXT,
    application       TEXT,
    created           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX bug_reports_created_idx ON bug_reports (created DESC);
CREATE INDEX bug_reports_type_idx ON bug_reports (bugtype);
CREATE INDEX bug_reports_similarity_idx ON bug_reports (similarityhash);
