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
    -- Plan 49: `period` buys an interval; `upgrade` moves the time already
    -- paid to a higher plan and ends with it.
    kind                TEXT NOT NULL DEFAULT 'period',
    plan                TEXT NOT NULL,
    billing_interval    TEXT NOT NULL,
    amount_cents        INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending',
    paid_at             TIMESTAMPTZ,
    -- Plan 55: the coin it was paid in, such as BTC, LTC or USDT, when the
    -- provider says. Set when paid.
    coin                TEXT,
    -- Set when paid: the plan applies until then.
    period_end          TIMESTAMPTZ,
    refunded_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crypto_payments_kind CHECK (kind IN ('period', 'upgrade')),
    CONSTRAINT crypto_payments_plan CHECK (plan IN ('pro', 'business')),
    CONSTRAINT crypto_payments_billing_interval CHECK (billing_interval IN ('month', 'year')),
    CONSTRAINT crypto_payments_amount CHECK (amount_cents > 0),
    CONSTRAINT crypto_payments_status CHECK (
        status IN ('pending', 'processing', 'paid', 'underpaid', 'expired', 'failed', 'refunded')
    ),
    CONSTRAINT crypto_payments_paid CHECK (
        status NOT IN ('paid', 'refunded') OR (paid_at IS NOT NULL AND period_end IS NOT NULL)
    ),
    CONSTRAINT crypto_payments_refunded CHECK ((status = 'refunded') = (refunded_at IS NOT NULL)),
    CONSTRAINT crypto_payments_invoice UNIQUE (provider, provider_invoice_id)
);

CREATE INDEX crypto_payments_user_idx ON crypto_payments (user_unid, period_end DESC);
