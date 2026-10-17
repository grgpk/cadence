# Plan 31: Pro gets event types, promises on the home page, booking polish

Status: done.
- Rust: fmt, clippy and every test pass.
- Frontend: build, typecheck, lint, knip and tests pass.

Checked against the dev servers:
- a time taken through the API while the form was open sent the invitee back to the open times, with a warning;
- the confirmation shows the call and the Outlook link;
- the promises band is on the home page.

## Goal

Three changes:
- **Event types in Pro.** Several kinds of call cost $13 here, against $10 on Calendly and nothing on Cal.com, so they move into Pro ($8).
- **Promises before pricing.** The three promises reach visitors on the home page, before they ever see pricing.
- **Booking polish.** The invitee's side of a booking matches or beats the big schedulers.

## Changes

- **Plans** (`billing/plans.rs`): Pro has `max_event_types: None`.
  - Business keeps webhooks until Plan 33 gives it more.
  - `a_plan_decides_how_many_types_the_link_offers` now checks that Free gets 403, Pro gets 201, and Free afterwards offers one type.
  - The event types page says "Needs Pro".
  - On the pricing page:
    - Pro lists "Several kinds of call on one link";
    - the week sizer's "more than one kind of call" lands on Pro.
- **Home page:** a "What every plan promises" band sits between How it works and the closing call to action.
  - It uses `PromiseList`, the same words and icons as `/pricing`.
- **Time just taken:** a 409 sends the invitee back to the times, keeping the form.
  - The open times refresh.
  - A warning says "That time was just taken. Here are the times still open."
- **Confirmation page:**
  - shows the call's name;
  - adds "Add to Outlook" (`outlookCalendarUrl`) beside Google and .ics, through the shared `AddToCalendar`;
  - adds a "What happens next" list: confirmation email, a reminder (every plan sends at least the 24h one), and reschedule or cancel.
- **Manage page:**
  - add-to-calendar for a call that can still change, so a moved call can be added again;
  - rescheduling shows times in a timezone the invitee picks (`timezoneForIana`).

## Tests

- `calendar-links.test.ts`: the Outlook URL.
- `booking-session.test.ts`: the confirmation carries the call's title.
- `pricing.test.ts`: several kinds of call recommend Pro.
