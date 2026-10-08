# Plan 11: frontend hardening

Status: implemented. The browser walk-through below is still to do.

## Goal

The app behaves well when something goes wrong: a session that ended sends the
person to log in and back, an error reads as a sentence, and the booking form
works by keyboard. The same time, status and query is written once, and the
pure logic has tests.

## The gap this fixes

- A session that expired showed an error panel whose "Try again" could not
  work. Every failed request was retried three times before saying so.
- Backend error strings such as "full_name is invalid" reached the screen.
- Login forgot where the person was going. An unknown URL sent an anonymous
  visitor to login instead of the 404.
- The booking form moved no focus between its steps, and its field errors were
  not tied to their inputs.
- Times were shown in six different styles, and a host saw each booking in the
  invitee's timezone.
- Statuses were plain strings in the wire types, so nothing checked them.
- The frontend had a test runner and no tests.

## Approach

No new dependency and no new look. Each milestone is one change that passes the
checks in `AGENTS.md`.

Decisions:

- Times follow the device's 12 or 24 hour style. The booking card's toggle
  overrides it, and the confirmation page keeps that choice.
- A host's bookings list is in the host's timezone, with the invitee's time on
  the second line when the zones differ.
- The old coaching wording goes from the legal pages and the France defaults
  become one constant. The optional questions and lead scoring stay.

## Data model

No migration. The status enums added in M3 mirror existing `CHECK` constraints
and only change the generated TypeScript; the Rust fields stay `String`.

## API

No new endpoint. M7 starts using the existing `POST /api/bug-reports`.

## Milestones

- [x] M1: test harness and CI. `vitest.config.ts`, unit tests for the pure
  modules, the search helpers moved to `lib/search.ts`; CI runs the tests and
  fails when generated bindings are not committed.
- [x] M2: robustness core. Query client in the router with one retry, redirect
  to login when a session ends, return path after login, 404 for unknown URLs,
  friendly error messages, an error page inside the shell.
- [x] M3: typed wire values. Status unions from Rust, `number` ids, cancelled
  bookings never upcoming.
- [x] M4: query options beside their fetchers; host calls out of the widget
  folder.
- [x] M5: one time format policy; host timezone in the bookings list.
- [x] M6: booking flow. Step focus, shared `Field`, shared slot states, form
  kept across a refresh.
- [x] M7: legal copy and defaults, head tags and `robots.txt`, client error
  reporting, `noUncheckedIndexedAccess`.

## What changed

- `router.tsx` owns the query client: one retry, none for an answer below 500,
  and a redirect to login when a protected page gets a 401. Unhandled route
  errors render `PageError` inside the shell; public routes keep `RouteError`.
- `lib/routes.ts`: `isProtectedPath` replaces the list of public paths, and
  `safeReturnPath` checks the `redirect` login takes.
- `lib/error-message.ts` turns a failed request into a sentence. Forms pass
  their own sentence for a status they understand.
- Rust: `BookingStatus`, `UserStatus` and `LeadQualification` exist for the
  bindings (`#[ts(as = ...)]`); `i64` ids export as `number`.
- `domains/dashboard/host-api.ts` holds the host's settings, hours and Google
  Calendar calls with their query options; `bookingsQuery`, `adminDataQuery`,
  `bookingPageQuery` and `publicSlotsQuery` sit beside their fetchers.
- `lib/format.ts`: `prefersHour12` is the default for every time; `zoneName`,
  `clockMinutes` and `clockTime` replace four copies each.
- `bookingColumns(hostTimezone)` for the host's lists; the admin list passes
  nothing and keeps the invitee's zone.
- Booking card: one mutation for the submit, focus moves to the step that
  opens, the form is kept in `sessionStorage` until the booking is made.
- `lib/bug-report.ts` posts uncaught errors, each message once and at most
  five per page load.
- The calendar events query keeps `retry: false`: its failure is how the page
  learns no calendar is connected.
- No `theme-color` tag: the tokens are `oklch` and a second copy of the
  background colour would drift.

## Not done here

- Cancel and reschedule, answers shown to the host, password reset, table
  pagination, admin write actions. Each needs a new endpoint.
- OAuth `state` (Plan 5) and response headers in nginx (Plan 6).
- A week strip in place of the month calendar, prerendering the static pages,
  and loading the country tables lazily: looked at and left alone.

## Tests

- `pnpm --dir frontend test`: `lib/search`, `lib/routes` (the guard and the
  return path, with crafted addresses), `lib/error-message`, `lib/format`,
  `widget/calendar-grid`, `widget/zoned-time`, `widget/booking-form`,
  `shared/phone/phone-format`, `admin/host-setup`, `bookings/bookings`.
- Checks from `AGENTS.md`.
- Browser walk-through at 390px and 1440px, light and dark, as admin and host.

## Definition of done

- Losing a session never leaves a page that cannot recover.
- No backend error string is shown to a person.
- The booking form can be completed by keyboard, with errors announced.
- A status, a time format or a query is defined in one place.
