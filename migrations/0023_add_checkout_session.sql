-- Plan 37: the host's newest Stripe Checkout page. Opening another closes
-- it, so two tabs cannot both be paid into two subscriptions.
ALTER TABLE subscriptions ADD COLUMN stripe_checkout_session_id TEXT;
