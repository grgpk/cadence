CREATE TABLE bookings (
    unid                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_unid           UUID NOT NULL REFERENCES users(unid) ON DELETE RESTRICT,
    lead_unid           UUID REFERENCES leads(unid) ON DELETE SET NULL,
    calling_visit_unid  UUID REFERENCES calling_visits(unid) ON DELETE SET NULL,
    slot_start          TIMESTAMPTZ NOT NULL,
    -- A call keeps the length it was booked with, whatever happens to the
    -- host's hours or timezone later.
    slot_end            TIMESTAMPTZ NOT NULL,
    timezone            TEXT NOT NULL DEFAULT 'UTC',
    invitee_name        TEXT NOT NULL,
    invitee_email       TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'confirmed',
    google_event_id     TEXT,
    google_meet_url     TEXT,
    -- The invitee's secret for changing their own call without an account.
    manage_token        TEXT NOT NULL UNIQUE DEFAULT encode(gen_random_bytes(24), 'hex'),
    cancelled_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT bookings_status CHECK (status IN ('pending', 'confirmed', 'cancelled')),
    CONSTRAINT bookings_slot_order CHECK (slot_end > slot_start)
);

-- A cancelled call gives its time back: only calls that still hold their
-- slot have to be unique per host.
CREATE UNIQUE INDEX bookings_slot_unique
    ON bookings (host_unid, slot_start)
    WHERE status <> 'cancelled';

CREATE INDEX bookings_host_slot_idx ON bookings (host_unid, slot_start);
CREATE INDEX bookings_lead_idx ON bookings (lead_unid);
