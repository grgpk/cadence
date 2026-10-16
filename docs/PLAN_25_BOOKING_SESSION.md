# Plan 25: the booking session, out of React

Status: implemented. Frontend typecheck, lint, knip, tests (167) and build
pass. Checked in the browser: the visit is recorded with its UTM tags and the
lead autosaves through the session. No booking was submitted.

From the architecture review (candidate 5). Terms are in `CONTEXT.md`.

## Goal

What happens when a visitor books (the visit, the lead, the booking and the
confirmation) can be tested without React, a router or a browser.

## The gap this fixes

- The booking card's `Booking` component ran the whole sequence in a
  mutation: save the lead, submit it once (a ref), tell the visit, create the
  booking, tell the visit again, then build a nine-field confirmation address
  inline. Only rendering could exercise it.
- The confirmation route checked that address with its own, separate shape.
- The "time was taken" check was written twice (booking card, manage page).
- `leadUpdate` and `queryValue` read `window.location` themselves.

## Approach

No new dependency.

- `domains/widget/booking-session.ts`:
  `createBookingSession(api, { slug, leadUnid, arrival })` gives `start()`
  (records the visit), `saveDraft(values)` (never throws) and
  `book(choice)` (returns the `ConfirmationSearch`). Visit updates never stop
  a booking; the lead is submitted once per session.
  `CADENCE_API` is the production API; tests pass a recorder.
- `ConfirmationSearch` is the one type for the confirmation address; the
  route's `validateSearch` returns it.
- `bookingFailure(error)`: `"taken"` or `"failed"`, used by the booking card
  and the manage page with their own wording.
- `booking-form.ts`: `arrivalHere()` reads the address once; `leadUpdate`
  and `queryValue` take it as an argument.
- The booking card keeps rendering, the remembered form and navigation.

## Tests

- `booking-session.test.ts`, against a recorder: the visit with its source;
  the order of calls; one submit across a retry after a taken time; booking
  without a visit record; booking when visit updates fail; autosave that
  never throws; every confirmation field, and a call with no place to meet;
  `bookingFailure`.
- `booking-form.test.ts`: `leadUpdate` trims, joins and reads the arrival;
  `queryValue`.

## Definition of done

- [x] The booking sequence lives in a plain module with injected API calls.
- [x] One type for the confirmation address.
- [x] One check for a taken time.
