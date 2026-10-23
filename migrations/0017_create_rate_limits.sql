CREATE TABLE rate_limits (
    action          TEXT NOT NULL,
    subject         TEXT NOT NULL,
    window_start    TIMESTAMPTZ NOT NULL,
    request_count   INT NOT NULL DEFAULT 1,
    PRIMARY KEY (action, subject, window_start)
);

CREATE INDEX rate_limits_window_idx ON rate_limits (window_start);
