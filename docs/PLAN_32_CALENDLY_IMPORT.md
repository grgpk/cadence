# Plan 32: bring your Calendly setup

Status: done.
- Rust: fmt, clippy and every test pass, including an end-to-end import against a fake Calendly.
- Frontend: build, typecheck, lint, knip and tests pass.
- Checked against the real Calendly API: a made-up token is refused, and the page says why.
- Not run live: a full import with a real token, which needs a Calendly account.

## Goal

Someone leaving Calendly brings over their event types, weekly hours,
dates with their own hours and their timezone in a minute. Cal.com offers
this, and Calendly users are the biggest group to win over.

## Flow

The page is Event types → "Import from Calendly" → `/dashboard/event-types/import`.

1. **Paste a token.** The host pastes a Calendly personal access token. API
   access works on every Calendly plan, Free included. The token needs read
   access to users, event types and availability. It is used for one preview
   and is never stored or logged.
2. **Review.** The page lists:
   - the kinds of call, as checkboxes;
   - the calls left out, with a reason;
   - the weekly hours and the count of dates with their own hours;
   - the timezone.

   On Free it says Pro brings all the calls.
3. **Bring them in.** Then a toast, and the page goes back to Event types.

## Backend: `domain/calendly`

The domain is new because Calendly is a new outside service, like
`google_calendar`.

- **`client.rs`:** `Calendly::{Live, Fake}`, in the same style as the Stripe
  gateway.
  - Live calls `GET /users/me`, `/event_types?user=&active=true&count=100`
    (at most 3 pages) and `/user_availability_schedules?user=` (the default
    schedule). Each call has a 10-second timeout.
  - A 401 or 403 from Calendly becomes `Unauthorized`.
  - The fake answers `FAKE_TOKEN` with a fixed account.
- **`service::preview_of`** is pure and unit-tested. It maps:
  - slugs to link-safe, unique values;
  - titles to at most 80 characters and descriptions to at most 300;
  - lengths to the 5–240 range;
  - `wday` to weekday 0 = Monday, with "24:00" read as 23:59;
  - `date` rules from today to a year ahead into dates with their own hours.

  Team calls (group, round robin) are skipped, with a reason.
- **`service::apply`** saves through the existing services, so validation
  and plan limits stay in one place:
  - the timezone, through `host_settings::update`;
  - the weekly hours, through the new `availability::replace_weekly`, which
    checks every rule before deleting and inserting in one transaction;
  - the dates, through `save_override`;
  - the kinds of call. The first fills the host's first call and keeps its
    link. The rest go through `event_types::create`. A `PlanRequired` counts
    the call as left out rather than failing the import.

  These steps are not one transaction across services. Each step validates
  before it writes.
- **Routes:**
  - `POST /api/host/import/calendly/preview`, rate-limited to 10 an hour per
    host;
  - `POST /api/host/import/calendly`.

## Tests

- `calendly::service::tests` covers type mapping, the schedule to hours and
  dates, and link-safe slugs.
- `host_booking_http::a_calendly_setup_comes_in_and_the_first_link_keeps_working`
  covers:
  - a refused token;
  - the preview;
  - an import on the Pro trial, with both calls, the hours replaced and the
    first link kept;
  - an import on Free, which brings one call and leaves one out.
