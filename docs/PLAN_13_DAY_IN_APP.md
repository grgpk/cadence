# Plan 13: the day inside the app, and host control

Status: implemented. Google Calendar event removal on cancel and email
delivery are not yet tried against the real services.

## Goal

The day tape is what Cadence looks like everywhere, not only on the landing
page. A host opens the dashboard and sees today as a strip of hours with
their calls on it. A host can also read what an invitee answered, cancel a
call, and decide how soon and how far ahead people may book. An invitee can
put the call in their own calendar.

## The gap this fixes

- Plan 12 added motion, but standing still the app looked the same.
- The day-tape identity lived only in the landing hero. The dashboard, the
  sign-in pages and "How it works" were plain cards, tables and text.
- Headings jumped from a 7xl hero to 2xl sections, and times were set in clock
  type in some places and body type in others.
- Hosts collected answers to their questions and could never read them.
- A booking could not be cancelled. `cancelled_at` existed and was unused.
- Notice was a 4 hour constant and the booking window a 7 day constant.
- The confirmation page gave the invitee nothing to save.

## Approach

No new dependency and no new colors. Archivo and Hanken Grotesk stay; what
changes is the scale and where each is used.

**Type and spacing**

- `text-clock` utility: display face, narrow, tabular. Every time uses it.
- Three heading steps: public section `text-3xl sm:text-4xl font-wide`, app
  page title `text-3xl font-wide`, card heading `font-semibold` with
  `px-5 py-3.5`. Settings cards use the same heading as every other card.
- Landing sections share `py-16 sm:py-24`.
- Settings gains a sticky list of its sections beside the form.

**Screens**

```
/dashboard
┌──────────────────────────────────────────────────────────────┐
│ Today, Friday, October 9                         2 calls left │
│ 13:00   14:00   15:00   16:00   17:00   18:00 │ 19:00   20:00 │
│ ─┬───────┬───────┬───────┬───────┬───────┬────│──┬───────┬─── │
│  ░ open (afternoon)        ░ open (evening)  now      ▓ Mia   │
└──────────────────────────────────────────────────────────────┘
┌ Next up ─────────────────────────────────────── All bookings ┐
│ Today                                                         │
│  ▌8:00 PM   Mia Chen     mia@…                        [Join]  │
│ Tomorrow                                                      │
│  ▌10:00 AM  Leo Park     leo@…                        [Join]  │
└───────────────────────────────────────────────────────────────┘
┌ Your booking page ───────────────────────────────────────────┐

/login, /register
┌───────────────────────┬──────────────────────────────┐
│ Cadence               │ Set your hours once.         │
│ Log in                │ Share one link.              │
│ [Email            ]   │                              │
│ [Password         ]   │ 09:00   10:00   11:00        │
│ [Log in           ]   │  ▓ Ana   ░ open  ░ open      │
└───────────────────────┴──────────────────────────────┘

/  "How it works"
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Mon ░░▓▓▓▓░░░   │  │ cadence/book/ana│  │ ▌09:00  ▌09:30  │
│ Tue ░░▓▓▓▓▓▓░   │  │ ✓ Link copied   │  │ ▓13:00  ▌17:30  │
└─────────────────┘  └─────────────────┘  └─────────────────┘
1 Set your hours     2 Share your link    3 Get booked
```

- **Dashboard.** `TodayTape` draws today on the host's clock: open hours
  tinted by part of day, calls as ink blocks, a signal line at "now" that
  moves each minute. A long day scrolls and starts with "now" in view. "Next
  up" is an agenda grouped by day; a name opens the booking in full.
- **Landing.** Each step shows a small piece of the product above its text.
  They are pictures, not controls.
- **Booking page and confirmation.** The host's initial sits by their name.
  The confirmation names the host, prints the time under the block, and
  offers "Add to Google Calendar" and "Download .ics".
- **Sign-in.** Form on the left, the day tape on a night panel on the right.
  The panel is a nested `dark` scope, so it reuses the dark tokens. Hidden on
  phones.

**Features**

- **Add to calendar.** Frontend only, `lib/calendar-links.ts`.
- **Booking rules.** Minimum notice (0 to 168 hours), booking window (1 to 90
  days) and a "Take bookings" switch. A paused page answers as not found.
- **Booking details.** A sheet with status, email, phone, the invitee's own
  time and their answers.
- **Cancel a booking.** From the details sheet, after a confirmation. The time
  opens again, waiting reminders are dropped and the invitee is emailed.

## Data model

- `0007_create_widget_settings.sql`: `widget_settings.booking_window_days`,
  default 7, checked 1 to 90, next to `minimum_notice_hours` and `is_active`.
- `0009_create_bookings.sql`: the `(host_unid, slot_start)` uniqueness on
  `bookings` is a partial unique index that ignores cancelled rows.

## API

| Route | Access | Result |
| --- | --- | --- |
| `GET`, `PUT /api/host/settings` | host | `HostSettings` with `minimum_notice_hours`, `booking_window_days`, `is_active` |
| `GET /api/hosts/{slug}` | public | `BookingPage` with `booking_window_days`; 404 when paused |
| `GET /api/bookings/{unid}` | host, own | `BookingDetail { booking, lead }`; 404 otherwise |
| `POST /api/bookings/{unid}/cancel` | host, own | the cancelled `Booking`; 409 when already cancelled or already started |

## Flow

Cancel, in one transaction: mark the booking cancelled, delete its unsent
rows in `email_queue`, queue `booking_cancelled`. After the commit the Google
Calendar event is deleted on a best-effort basis; a failure is logged and does
not undo the cancellation.

## Implementation sketch

- Backend: `bookings/{service,db,handlers,routes,models}.rs`,
  `host_settings/{models,service,db}.rs`, `emails/service.rs` (template),
  `google_calendar/{client,service}.rs` (`delete_event`).
- Frontend: `domains/dashboard/TodayTape.tsx`,
  `domains/bookings/BookingDetailSheet.tsx`, `domains/bookings/bookings.ts`
  (`agendaDays`, detail query, cancel), `domains/landing/StepPreviews.tsx`,
  `domains/shared/AuthCard.tsx`, `routes/dashboard/{index,settings}.tsx`,
  `routes/booking-confirmation.tsx`, `lib/calendar-links.ts`.

## Out of scope

- A cancel or reschedule link for the invitee. It needs a signed token.
- A buffer between calls. Slots are fixed steps, so it needs another model.
- A message on a paused page instead of not found.
- Stacked mobile layouts for the admin tables.

## Tests

- Checks from `AGENTS.md`.
- `backend/tests/host_booking_http.rs`: notice and window on `/slots`, range
  validation, pause, detail ownership and answers, the cancel flow.
- Frontend: `calendar-links.test.ts`, `agendaDays` in `bookings.test.ts`.
- Browser walk-through at 390px and 1440px, light and dark: landing, sign-in,
  dashboard, open a booking, cancel it, settings, booking page, confirmation.

## Definition of done

- Every time on screen is set in `text-clock`.
- A cancelled call can be booked again and sends no reminders.
- One host cannot read or cancel another host's booking.
- Bindings are generated, never edited by hand.
