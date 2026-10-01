CREATE TABLE bookings (
    unid                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_unid           UUID NOT NULL REFERENCES users(unid) ON DELETE RESTRICT,
    lead_unid           UUID REFERENCES leads(unid) ON DELETE SET NULL,
    calling_visit_unid  UUID REFERENCES calling_visits(unid) ON DELETE SET NULL,
    slot_start          TIMESTAMPTZ NOT NULL,
    timezone            TEXT NOT NULL DEFAULT 'UTC',
    invitee_name        TEXT NOT NULL,
    invitee_email       TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'confirmed',
    google_event_id     TEXT,
    google_meet_url     TEXT,
    cancelled_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT bookings_status CHECK (status IN ('pending', 'confirmed', 'cancelled')),
    CONSTRAINT bookings_slot_unique UNIQUE (host_unid, slot_start)
);

CREATE INDEX bookings_host_slot_idx ON bookings (host_unid, slot_start);
CREATE INDEX bookings_lead_idx ON bookings (lead_unid);
