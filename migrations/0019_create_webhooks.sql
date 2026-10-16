-- Plan 29: a host's own server hears about their calls. Each endpoint gets
-- every booking event, signed with its secret; deliveries queue and retry.
CREATE TABLE webhook_endpoints (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_unid       UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    -- Signs every delivery; the host checks it on their side.
    secret          TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT webhook_endpoints_url CHECK (url LIKE 'https://%' AND char_length(url) <= 500)
);

CREATE INDEX webhook_endpoints_host_idx ON webhook_endpoints (host_unid, created_at);

CREATE TABLE webhook_deliveries (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint_unid   UUID NOT NULL REFERENCES webhook_endpoints(unid) ON DELETE CASCADE,
    event           TEXT NOT NULL,
    -- The event's `data`; the worker wraps it with the delivery's id.
    payload         JSONB NOT NULL,
    attempts        INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at    TIMESTAMPTZ,
    -- Given up after the last retry.
    failed_at       TIMESTAMPTZ,
    -- The receiver's HTTP status, when it answered at all.
    last_status     INT,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT webhook_deliveries_event CHECK (
        event IN ('booking.created', 'booking.rescheduled', 'booking.cancelled', 'ping')
    )
);

-- What the worker reads: deliveries still waiting, oldest due first.
CREATE INDEX webhook_deliveries_due_idx ON webhook_deliveries (next_attempt_at)
    WHERE delivered_at IS NULL AND failed_at IS NULL;
CREATE INDEX webhook_deliveries_endpoint_idx ON webhook_deliveries (endpoint_unid, created_at DESC);
