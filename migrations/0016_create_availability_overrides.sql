-- Hours for one date that replace the weekly hours on that date. A date is
-- either one row with no times (off all day) or one row per range.
CREATE TABLE availability_overrides (
    id              BIGSERIAL PRIMARY KEY,
    host_unid       UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    -- A wall-clock date in the host's timezone.
    date            DATE NOT NULL,
    start_time      TIME,
    end_time        TIME,
    slot_minutes    INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT availability_overrides_shape CHECK (
        (start_time IS NULL AND end_time IS NULL AND slot_minutes IS NULL)
        OR (start_time < end_time AND slot_minutes BETWEEN 5 AND 240)
    )
);

CREATE INDEX availability_overrides_host_date_idx
    ON availability_overrides (host_unid, date, start_time);
