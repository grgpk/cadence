ALTER TABLE outbox ADD COLUMN send_after TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE INDEX outbox_due_idx ON outbox (send_after) WHERE sent_at IS NULL;