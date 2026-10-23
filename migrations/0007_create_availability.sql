CREATE TABLE availability (
    id              BIGSERIAL PRIMARY KEY,
    host_unid       UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    weekday         INT NOT NULL,
    start_time      TIME NOT NULL,
    end_time        TIME NOT NULL,
    slot_minutes    INT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT availability_weekday_range CHECK (weekday BETWEEN 0 AND 6),
    CONSTRAINT availability_time_order CHECK (start_time < end_time),
    CONSTRAINT availability_slot_minutes CHECK (slot_minutes BETWEEN 5 AND 240)
);

CREATE INDEX availability_host_weekday_idx
    ON availability (host_unid, weekday, start_time);
