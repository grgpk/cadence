-- Plan 52: money an admin gave back on a crypto payment, in part or in full.
-- A payment is `refunded` (its time taken back) once these cover all of it.

CREATE TABLE crypto_refunds (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_unid    UUID NOT NULL REFERENCES crypto_payments(unid) ON DELETE CASCADE,
    amount_cents    INTEGER NOT NULL,
    -- Where the host claims it; NULL for a provider that refunds by hand.
    claim_url       TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crypto_refunds_amount CHECK (amount_cents > 0)
);

CREATE INDEX crypto_refunds_payment_idx ON crypto_refunds (payment_unid);
