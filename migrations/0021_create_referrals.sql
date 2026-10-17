-- Plan 36: a host invites another; once the invited host pays, both get a
-- month of Pro as Stripe credit.

CREATE TABLE referrals (
    -- One inviter per account, recorded at sign-up.
    referred_unid           UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
    referrer_unid           UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Each side is credited once. The inviter's credit waits until they
    -- have a Stripe account of their own.
    referred_credited_at    TIMESTAMPTZ,
    referrer_credited_at    TIMESTAMPTZ,
    -- The invoice that earned the credits; refunding it takes them back.
    referred_invoice_id     TEXT,
    revoked_at              TIMESTAMPTZ,
    CONSTRAINT referrals_not_self CHECK (referred_unid <> referrer_unid)
);

CREATE INDEX referrals_referrer_idx ON referrals (referrer_unid);
