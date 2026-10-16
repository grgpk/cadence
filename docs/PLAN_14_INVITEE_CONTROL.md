# Plan 14: the invitee's side, and the last plain screens

Status: implemented. Moving a call and cancelling it from the invitee's link
are covered by API tests but were not clicked through in a browser. Email
delivery, the Google Calendar event move and how mail clients draw the HTML
are not yet tried against the real services.

## Goal

The person who booked a call can move or cancel it from a private link,
without an account, and the host hears about every booking. Availability and
Bookings look like the rest of the app, and the emails people receive look
like Cadence.

## The gap this fixes

- Only the host could cancel. An invitee had no way to change their call.
- A host got nothing from Cadence about a new booking unless Google Calendar
  was connected.
- Emails were one line of plain text with no host name.
- Availability drew each day as a 6px line and a chip. Hours could be added
  and removed but not changed.
- Bookings was a bare table next to a dashboard that is an agenda.
- A paused booking page said "This page does not exist".

## Approach

No new dependency. Existing tokens only.

```
/dashboard/availability
              08:00   10:00   12:00   14:00   16:00   18:00
              ─┬───────┬───────┬───────┬───────┬───────┬──
Monday         ░ 9:00 to 12:00 ░        ▒ 14:00 to 18:30 ▒▓       + Add hours
               [9:00 AM to 12:00 PM] 30 min calls ×
Tuesday        ░░░░░░░░ 9:00 to 17:00 ▒▒▒▒▒▒▒▒▒▒▒▒                + Add hours

/booking/{token}
Your call with Giorgi
┌────────────────────────────────────────────┐
│ 11:00   12:00   13:00   14:00              │
│ ─┬───────┬───────┬───────┬───              │
│          ▓▓▓  12:00 to 12:30               │
└────────────────────────────────────────────┘
 Date / Time / Timezone
[Reschedule] [Cancel booking]
┌ calendar ───────────────────┬ times ───────┐   (after "Reschedule")
```

- **Availability.** One hour ruler for the whole week, sized by `weekHours`.
  Each range is a block tinted by part of day (`phaseSpans`). A block or its
  chip opens `HoursForm`, which also adds hours and copies them to other days.
- **Bookings.** The dashboard's agenda, moved to `domains/bookings/Agenda.tsx`
  and used on both pages. Search by name or email. Cancelled calls are struck
  through with their badge.
- **Manage link.** `/booking/{token}` shows the call and lets the invitee move
  it (the booking page's calendar and time list, through `SlotStep` without
  its summary column) or cancel it. The confirmation page and every invitee
  email link to it.
- **Paused page.** "{host} is not taking bookings right now."
- **Error pages** keep the public header and footer.
- **Emails.** Text and HTML, one layout: the logo (a hosted PNG), a headline, the call
  as a block with the part-of-day color, then buttons. Every value from a
  person is escaped.

## Data model

- `0009_create_bookings.sql`: `bookings.manage_token`, unique, with a random
  default.
- `APP_URL`: the public site address used in email links. Falls back to the
  first `CORS_ORIGINS` entry.

## API

| Route | Access | Result |
| --- | --- | --- |
| `PUT /api/availability/{id}` | host, own | the changed `AvailabilityRule` |
| `GET /api/hosts/{slug}` | public | `BookingPage` with `accepting_bookings`; a paused page is readable, not bookable |
| `POST /api/hosts/{slug}/bookings` | public | `CreatedBooking`: the booking and its `manage_token` |
| `GET /api/manage/{token}` | public | `ManagedBooking`; 404 for an unknown link |
| `POST /api/manage/{token}/cancel` | public | the cancelled call; 409 when cancelled or started |
| `POST /api/manage/{token}/reschedule` | public | the moved call; 409 when the time is not open |

The token never appears in `Booking` as hosts and admins read it.

## Flow

- **New booking:** confirmation and reminders to the invitee,
  `host_new_booking` to the host.
- **Invitee cancels:** the time opens, unsent emails are dropped, the host gets
  `host_booking_cancelled`. The invitee is not emailed about their own action.
- **Invitee moves the call:** `slot_start` changes in place so the link keeps
  working, unsent emails are replaced, the invitee gets `booking_rescheduled`
  and fresh reminders, the host gets `host_booking_rescheduled`, and the
  Google Calendar event is moved on a best-effort basis.
- **Host cancels:** as in Plan 13; nothing is sent to the host.

## Out of scope

- Days off and date overrides.
- A buffer between calls.
- An `.ics` attachment on emails; they link to Google Calendar instead.
- Letting the host move a call.

## Tests

- Checks from `AGENTS.md`.
- `backend/tests/host_booking_http.rs`: editing hours, the paused page, the
  manage link, invitee cancel, reschedule and the emails each one queues.
- Unit tests for `render`: every template gives text and HTML, input is
  escaped, and a payload queued before this plan still renders.
- Frontend: `weekHours`, `phaseSpans`, `matchesSearch`.
- Email previews: `CADENCE_EMAIL_PREVIEW_DIR=<dir> cargo test -p
  cadence-backend --lib email_previews -- --ignored`.
- Browser walk-through at 390px and 1440px: availability, edit hours,
  bookings and its Past tab, the manage link up to the "Move your call?"
  question, an unknown manage link, a paused page.

## Definition of done

- An invitee can move and cancel their call from the link alone.
- The link of one call never reads or changes another.
- A host is emailed when someone books, moves or cancels.
- Every email has a plain-text part.
