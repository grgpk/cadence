# Plan 16: hardening pass

Status: implemented. Google Calendar paths and SMTP sending were not run
against the real services.

## Goal

Nothing new. What five quick rounds built is made safe to run: no published
login in production, no overlapping calls, sessions that end when a password
changes, an email worker that sends once, and screens that hold up under a
keyboard, a slow network and a retry.

## The gap this fixes

Two read-only audits listed about 90 concrete defects. The ones that mattered
most:

- The development seed ran as migration 9999, so production got
  `root@example.com` with the password `password` and the Root role.
- A call stored only its start. Its length was re-derived from today's rules,
  and only an identical start was refused, so overlapping rules or two
  requests at once could double-book.
- A stolen session survived a password reset; a banned user could log in.
- The email worker could send a row twice across two instances, stall behind
  an unknown template, and send a reminder after the call.
- Google calls had no timeout, and a failed booking left an event and an
  invite behind.
- The Settings form could overwrite newer values; a retry in Availability
  duplicated hours; "Move call" could be confirmed twice.
- Focus was lost after editing hours and after rescheduling or cancelling.

## What changed

**Seed.** `migrations/9999____seed_data.sql` is now `seeds/dev_seed.sql`,
re-runnable, applied by `reset_db.sh` and `scripts/seed-dev.sh` only. The
migrator ignores the recorded 9999. `0024` bans each seed login that still
has the seed password; nothing is deleted.

**Calls.** `0026` adds `bookings.slot_end`, backfilled from the rule that
covered each start. Length is stored at booking and reschedule and exposed as
`Booking.minutes`. One rule decides what is free, for both the list of times
and the booking itself: a new call may not come within the host's gap of
another live call. Create and reschedule take a per-host transaction lock and
re-check inside it.

**Sessions and login.** `0025` adds `users.session_version`, carried in the
token; a password change or reset bumps it. Banned users cannot log in.
Duplicate sign-up is 409. The login limit is per email and address together
(5 per 10 minutes) plus per address (10). Unknown emails cost the same
hashing time. Hashing runs off the async runtime. The cookie is `Secure`
unless `APP_ENV=local`.

**Google Calendar.** One HTTP client with a 10 second timeout. The event is
created after the booking is committed. Cadence's own events are not counted
as busy. The sign-in round trip carries a single-use `state`.

**Email worker.** Each row is claimed with `SKIP LOCKED`. Unknown templates
are marked failed. Reminders for started calls and emails for cancelled calls
are deleted, not sent. SMTP has a 20 second timeout. A sent reset email loses
its link. Old rows are purged. HTML emails greet the reader.

**Limits.** Real email checks and length caps on bookings, leads and visits;
rate limits on slots, visits, bug reports and reset-password; 64 KiB body
limit; booking lists capped at 500; security headers and an overwritten
`X-Forwarded-For` in nginx.

**Frontend bugs.**
- Settings follows newer saved values while unedited, keeps what is typed in
  number fields, clears the link error on Discard, and asks before leaving
  with unsaved changes.
- The dashboard says when the page is paused; the paused wording is correct.
- Availability retries only the days that failed; "From" cannot take the last
  time of the day.
- The manage page locks the picker while saving, has room for two columns,
  handles a paused host, and refreshes times after a cancel.
- Rulers use the viewer's clock style. "Tomorrow" is a calendar day. Upcoming
  re-sorts each minute. A failed settings request is an error, not UTC.
- Views draw each call at its own stored length.

**Accessibility.**
- Focus moves into the hours form and back to what opened it; after a call is
  rescheduled or cancelled the heading takes it.
- Past calls are outlined, not faded; today carries a "today" badge.
- The week view lists each day's calls under its track on a phone.
- Search inputs and segmented controls have names; the time list announces
  its day; agenda day headings are `h2` under a page title.
- `DataTable` renders one layout, so a row's panel mounts once.
- The "now" pill and the selected day in dark theme are darker for contrast.
  The ratios were estimated from the tokens, not measured.

**Cleanup.** `SegmentedControl`, `HourMark`, `NAME_LINK`, `formatLongDay` and
cached date formatters replace the copies. One word per thing: call,
Reschedule, Join call.

## Data model

- `0024_ban_seed_logins.sql`
- `0025_add_session_version.sql`: `users.session_version`
- `0026_add_booking_end.sql`: `bookings.slot_end`, not null, after start
- `0027_index_email_queue_booking.sql`

## API

- `Booking` and `CreatedBooking` gain `minutes`.
- `GET /api/google-calendar/oauth-config` returns a `state`;
  `POST /api/google-calendar/login` requires it.
- `POST /api/auth/password` re-issues the session cookie.
- Any overlap, or a time inside the gap, answers 409.
- A cancelled call's manage response has no `meeting_url`.

## Left alone

- Graceful shutdown and a request timeout layer: both need a Cargo feature
  that changes `Cargo.lock`.
- Guessing one email's password from many addresses is limited per address
  only. A per-email limit would let an attacker lock the owner out.
- Sign-up still says when an email is taken.
- Google tokens are stored in plain text.
- Unused tables and columns are not dropped (`audit_logs`, `accent_color`,
  `duration_minutes`, and others).
- Past and cancelled lists are not paginated beyond the 500 cap.
- No exclusion constraint on overlapping calls: existing rows could fail it.
- If a host cancels in the seconds between the insert and the calendar step,
  the event is still created.
- `ENVIRONMENT` and `APP_ENV` are both read, by different code.
- The public booking page still imports the full country and timezone tables.

## Tests

- Checks from `AGENTS.md`.
- Backend: seed logins are banned unless their password changed; overlapping
  rules cannot double-book; two bookings at once, one wins; a call keeps its
  length after its rule changes; a call can move next to itself but not next
  to another; old sessions are refused after a password change; a victim can
  log in while an attacker is limited; two worker ticks send each row once;
  late and cancelled emails are not sent; a wrong or reused OAuth state is
  refused.
- Frontend: `hourLabel`, `formatLongDay`, `dayCall`, `agendaDays` across a
  clock change, `daysLabel`, `callLengthLabel`.
- Browser: focus into and out of the hours form; a cleared number field stays
  empty and takes a typed value; paused status on the dashboard; week view at
  390px and 1440px; the reschedule picker in two columns with a paused and an
  active host.

## Definition of done

- A fresh production database has no seeded login, and an existing one has
  them switched off.
- No two live calls of one host can be closer than the host's gap.
- A password change ends every other session.
- No email is sent twice, and none after it has stopped being true.
