CREATE TABLE bookings (
    id                  BIGSERIAL PRIMARY KEY,
    host_id             BIGINT NOT NULL REFERENCES hosts(id),
    slot_start          TIMESTAMPTZ NOT NULL,
    invitee_name        TEXT NOT NULL,
    invitee_email       TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (host_id, slot_start)
);

