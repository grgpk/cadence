-- Plan 48: a host pays for a month or a year of a plan in crypto, opting in
-- from Billing. Crypto is never charged again by itself, so each row is one
-- prepaid period. Stripe's `subscriptions` table is not involved.

CREATE TABLE crypto_payments (
    -- Also the order id the provider is given and echoes back in its events.
    unid                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_unid           UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    -- Which processor took it, so swapping providers leaves old rows readable.
    -- No CHECK: `CryptoProvider` in Rust owns the values, and adding a
    -- provider should not need a schema change.
    provider            TEXT NOT NULL,
    -- NULL until the provider has made the invoice.
    provider_invoice_id TEXT,
    plan                TEXT NOT NULL,
    billing_interval    TEXT NOT NULL,
    amount_cents        INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending',
    paid_at             TIMESTAMPTZ,
    -- Set when paid: the plan applies until then.
    period_end          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crypto_payments_plan CHECK (plan IN ('pro', 'business')),
    CONSTRAINT crypto_payments_billing_interval CHECK (billing_interval IN ('month', 'year')),
    CONSTRAINT crypto_payments_amount CHECK (amount_cents > 0),
    CONSTRAINT crypto_payments_status CHECK (
        status IN ('pending', 'processing', 'paid', 'underpaid', 'expired', 'failed')
    ),
    CONSTRAINT crypto_payments_paid CHECK (
        status <> 'paid' OR (paid_at IS NOT NULL AND period_end IS NOT NULL)
    ),
    CONSTRAINT crypto_payments_invoice UNIQUE (provider, provider_invoice_id)
);

CREATE INDEX crypto_payments_user_idx ON crypto_payments (user_unid, period_end DESC);
