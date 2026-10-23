-- A person who filled in a host's booking form or booked a call. Leads and
-- bookings point here, so one person is one row per host however often they
-- come back. The email can wait: the form saves a name before it has one.
CREATE TABLE contacts (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_unid       UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    email           TEXT,
    first_name      TEXT,
    last_name       TEXT,
    phone           TEXT,
    country_code    TEXT,
    -- The host sent this person one "here are my times" email. At most one
    -- is ever sent.
    followed_up_at  TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX contacts_host_email_unique
    ON contacts (host_unid, lower(email))
    WHERE email IS NOT NULL;
