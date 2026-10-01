CREATE TABLE email_queue (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    booking_unid UUID REFERENCES bookings(unid) ON DELETE CASCADE,
    email       TEXT NOT NULL,
    template    TEXT NOT NULL,
    send_at     TIMESTAMPTZ NOT NULL,
    sent_at     TIMESTAMPTZ,
    attempts    INT NOT NULL DEFAULT 0,
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    payload     JSONB
);

CREATE INDEX email_queue_pending_idx
    ON email_queue (send_at)
    WHERE sent_at IS NULL;
