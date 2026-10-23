CREATE TABLE bookings (
    unid                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_unid           UUID NOT NULL REFERENCES users(unid) ON DELETE RESTRICT,
    -- The kind of call booked. A deleted type leaves its calls in place;
    -- they are read with the host's first type then.
    event_type_unid     UUID REFERENCES event_types(unid) ON DELETE SET NULL,
    -- Who booked: their name and email live on the contact.
    contact_unid        UUID NOT NULL REFERENCES contacts(unid) ON DELETE RESTRICT,
    lead_unid           UUID REFERENCES leads(unid) ON DELETE SET NULL,
    slot_start          TIMESTAMPTZ NOT NULL,
    -- A call keeps the length it was booked with, whatever happens to the
    -- host's hours or timezone later.
    slot_end            TIMESTAMPTZ NOT NULL,
    timezone            TEXT NOT NULL DEFAULT 'UTC',
    status              TEXT NOT NULL DEFAULT 'confirmed',
    google_event_id     TEXT,
    google_meet_url     TEXT,
    -- The invitee's secret for changing their own call without an account.
    manage_token        TEXT NOT NULL UNIQUE DEFAULT encode(gen_random_bytes(24), 'hex'),
    -- The invitee said they are coming, from the button in their reminder.
    -- A moved call starts unconfirmed again.
    attendance_confirmed_at TIMESTAMPTZ,
    -- The host marked, after the call, that the invitee did not come.
    no_show_at          TIMESTAMPTZ,
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
CREATE INDEX bookings_contact_idx ON bookings (contact_unid);
