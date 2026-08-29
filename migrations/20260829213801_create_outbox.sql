CREATE TABLE outbox (
                        id          BIGSERIAL PRIMARY KEY,
                        booking_id  BIGINT NOT NULL REFERENCES bookings(id),
                        to_email    TEXT NOT NULL,
                        subject     TEXT NOT NULL,
                        body        TEXT NOT NULL,
                        created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                        sent_at     TIMESTAMPTZ                      -- NULL = still queued
);