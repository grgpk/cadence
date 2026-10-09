-- The invitee's secret for changing their own call without an account.
-- Existing rows get one from the default.
ALTER TABLE bookings
    ADD COLUMN manage_token TEXT NOT NULL UNIQUE DEFAULT encode(gen_random_bytes(24), 'hex');
