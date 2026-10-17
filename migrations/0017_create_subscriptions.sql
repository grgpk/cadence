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
    -- The host's newest Checkout page. Opening another closes it, so two
    -- tabs cannot both be paid into two subscriptions.
    stripe_checkout_session_id TEXT,
    -- Stripe's `pause_collection.resumes_at`: the plan stays on and nothing
    -- is charged until then.
    paused_until            TIMESTAMPTZ,
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

-- Why a host cancelled, paused or moved to yearly from the cancel flow, for
-- the admin to read.
CREATE TABLE cancellation_feedback (
    unid        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_unid   UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    reason      TEXT NOT NULL,
    note        TEXT,
    outcome     TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT cancellation_feedback_reason CHECK (reason IN (
        'too_expensive', 'not_enough_bookings', 'missing_feature', 'switching', 'pausing', 'other'
    )),
    CONSTRAINT cancellation_feedback_note CHECK (char_length(note) <= 500),
    CONSTRAINT cancellation_feedback_outcome CHECK (outcome IN ('cancelled', 'paused', 'yearly'))
);

CREATE INDEX cancellation_feedback_recent_idx ON cancellation_feedback (created_at DESC);
CREATE INDEX cancellation_feedback_user_idx ON cancellation_feedback (user_unid);
