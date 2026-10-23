# Plan 33: Business that turns bookings into clients

Status: done.
- Rust: fmt, clippy and every test pass, including three new HTTP tests.
- Frontend: build, typecheck, lint, knip and 210 tests pass.

Checked in the browser as the seeded Business host:
- "I'm coming" from a reminder link records the confirmation on the manage page;
- the Bookings list shows Confirmed / Not confirmed yet;
- the Unfinished panel sends a follow-up, queued in `email_queue`;
- the Overview shows the 7-day card;
- `/pricing?tools=yes` recommends Business.

## Goal

Plan 31 moved several kinds of call into Pro, which left Business ($16, or
$13 yearly) with only webhooks. Business now gives a solo professional tools
that Calendly and Cal.com do not offer at this price. Each tool is built on
data Cadence already keeps.

1. **Win back people who left.** The form saves each visitor's draft
   (`leads`), so the host sees who gave an email and left without a call. The
   host sends each one their times, at most once.
2. **Know who will show up.** The day-before reminder asks "I'm coming" or "I
   need another time". The host hears 2 hours ahead about anyone unconfirmed,
   and marks no-shows after a call.
3. **Your week in numbers.** An Overview card and a Monday 08:00 email show:
   - booked, held, confirmed ahead and no-shows;
   - people who left without booking;
   - the busiest weekday and hour;
   - the top campaign source.
4. **Webhooks**, as before.

## Data model: `migrations/0020_add_business_tools.sql`

The migration is new rather than an edit to the CREATE migrations, because
production has already run them.
- `bookings.attendance_confirmed_at`: cleared when a call moves.
- `bookings.no_show_at`.
- `leads.followed_up_at`.

Plan 42 later folded this migration into the CREATE migrations; see
`PLAN_42_SCHEMA_NORMALIZATION.md`.

`Entitlements` gains `win_back`, `attendance` and `insights`, true for
Business only.

## API

| Route | Who | What |
|---|---|---|
| `GET /api/host/leads/unfinished` | Business | email given, gone an hour, no booking since, not followed up, last 30 days |
| `POST /api/host/leads/{unid}/follow-up` | Business | queues `lead_follow_up`; a second one is 409 |
| `POST /api/manage/{token}/attend` | invitee | sets `attendance_confirmed_at`; rate-limited like the other manage calls |
| `POST /api/bookings/{unid}/no-show` | Business | `{ no_show }`, only once the call is over |
| `GET /api/host/insights` | Business | the last 7 days, and the busiest time and sources over 30 |

`BookingPage.follow_up_notice` tells the form to add "If you leave without
picking a time, {host} may email you once with their open times." to the
consent line.

## Emails (all through `email_queue`)

- **`reminder_24h`:** gets "I'm coming" and "I need another time" when the
  payload has `attend_url`, which only Business hosts' calls carry.
- **`host_unconfirmed`:** queued with the reminders for 2 hours before the
  call. The worker drops it once the invitee confirms
  (`PendingEmail.call_confirmed`) or the call starts.
- **`lead_follow_up`:** "Still want to talk with {host}?" with "Pick a time",
  and a note that it is the only email.
- **`weekly_insights`:** the email worker's sweep queues one for each
  Business host whose clock reads Monday 08:xx and who had none in the last 6
  days.

## UI

- **Bookings:**
  - "Started booking and left (n)" panel with "Send my times", shown only
    when there is someone;
  - attendance badges on rows;
  - "Coming" row and "Mark no-show" / "They did come" in the detail sheet.
- **Overview:** "Your last 7 days" card (Booked, Showed up, Confirmed ahead,
  Left without booking), plus the busiest time and top source.
- **Manage page:**
  - `?attend=yes` confirms once;
  - "{host} knows you are coming" shows whenever the call is confirmed.
- **Pricing:**
  - Business tagline "For turning bookings into clients.";
  - highlights for the three tools;
  - a "Business tools" group in the compare table;
  - the week sizer's "I want to follow up with people and see who shows up"
    (`?tools=yes`) recommends Business.

## Tests

- **HTTP:**
  - `the_business_tools_are_closed_to_pro`;
  - `a_business_host_sends_their_times_once_to_someone_who_left`;
  - `a_business_host_knows_who_comes_and_who_did_not` (attend, the
    unconfirmed heads-up queued, no-show only after the call, insights
    counts, undo).
- **Email units:**
  - the day-before buttons;
  - the heads-up dropped once confirmed;
  - follow-up and weekly rendering.
- **Frontend:** `attendanceOf`, `showUpRate`, the client-tools recommendation
  and its URL round trip.
