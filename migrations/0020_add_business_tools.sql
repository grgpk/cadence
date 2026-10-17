-- Plan 33: Business tools that turn bookings into clients.

-- The invitee said they are coming, from the button in their reminder.
-- A moved call starts unconfirmed again.
ALTER TABLE bookings ADD COLUMN attendance_confirmed_at TIMESTAMPTZ;
-- The host marked, after the call, that the invitee did not come.
ALTER TABLE bookings ADD COLUMN no_show_at TIMESTAMPTZ;

-- The host sent this unfinished booking one "here are my times" email.
-- At most one is ever sent.
ALTER TABLE leads ADD COLUMN followed_up_at TIMESTAMPTZ;
