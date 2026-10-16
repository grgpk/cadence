-- A host's paid plan or trial. No row means the Free plan. Rows with a
-- Stripe subscription mirror it; webhooks are the only writer of those.
CREATE TABLE subscriptions (
    user_unid               UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
    plan                    TEXT NOT NULL,
    -- Stripe's subscription statuses we act on, plus `comped` for a plan an
    -- admin gives without payment.
    status                  TEXT NOT NULL,
    -- NULL for a trial or a comped plan: nothing is billed.
    billing_interval        TEXT,
    trial_ends_at           TIMESTAMPTZ,
    current_period_end      TIMESTAMPTZ,
    cancel_at_period_end    BOOLEAN NOT NULL DEFAULT false,
    stripe_customer_id      TEXT UNIQUE,
    stripe_subscription_id  TEXT UNIQUE,
    -- `created` of the last Stripe event applied, so an older event that
    -- arrives late cannot undo a newer one.
    stripe_synced_at        TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT subscriptions_plan CHECK (plan IN ('pro', 'business')),
    CONSTRAINT subscriptions_status CHECK (
        status IN ('trialing', 'active', 'past_due', 'canceled', 'incomplete', 'comped')
    ),
    CONSTRAINT subscriptions_billing_interval CHECK (billing_interval IN ('month', 'year')),
    CONSTRAINT subscriptions_trial_end CHECK (status <> 'trialing' OR trial_ends_at IS NOT NULL)
);

-- Stripe events already applied. Stripe delivers at least once.
CREATE TABLE stripe_events (
    id          TEXT PRIMARY KEY,
    type        TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
