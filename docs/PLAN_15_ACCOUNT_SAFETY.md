# Plan 15: prove what exists, then account safety and the remaining screens

Status: implemented. The reset email has not been sent for real, and no Google
Calendar path has been tried against Google.

## Goal

What earlier plans built is shown to work with real email. A host who forgets
their password can get back in, public forms cannot be hammered, and calls can
have a gap between them. Bookings can be read as a week, admin works on a
phone, and a booking link shows a picture when it is shared.

## The gap this fixes

- Email delivery and the invitee's "Move call" and "Cancel booking" had never
  been run with real sending.
- No way to recover a forgotten password or change a name or password.
- No limit on login, sign-up, booking or manage-link attempts. The
  `rate_limits` table existed and was unused.
- `buffer_minutes` existed with a default of 15 and was never read.
- An email the provider refused for good was retried every minute forever.
- Cancelled calls sat under "Past". Admin tables scrolled sideways on phones.
- A pasted booking link showed no image.

## Verified with real sending

On the local server, one call was booked, moved and cancelled through the
pages, as an invitee. Five emails were sent: booked, host notice, moved (both
sides) and cancelled (host side). Both freed times became bookable again.
Resend's test sender only accepts the account owner's address, so the host
notices were pointed at that inbox to send.

## Approach

No new dependency. Existing tokens only.

```
/dashboard/bookings   List | Week              ‹  5 to 11 October  ›
              08:00   10:00   12:00   14:00   16:00   18:00
              ─┬───────┬───────┬───────┬───────┬───────┬──
Mon 5          ░░░░░░░░░░░░░░        ▒▒▒▒▒▒▒▒▒▒▒▒▓▓
Tue 6          ░░░░▓Ana░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
Sat 10               ░▓Ivo▒▒▒▒▒▒▒▒
```

- **Bookings.** Tabs are Upcoming, Past and Cancelled. A List / Week switch;
  Week draws seven days on one ruler with open hours tinted, calls as blocks
  that open the detail sheet, a "now" line, and previous and next week. The
  ruler is shared with Availability through `domains/dashboard/WeekGrid.tsx`.
- **Booking details step.** A field says what is wrong once it has been left,
  not only after a failed submit.
- **Admin.** Dates in `text-clock`. `DataTable` stacks each row as a short
  list under `sm`. The funnel shows each step as a share of visits. Failed
  emails appear under "Needs attention".
- **Link previews.** `frontend/public/og.png` (1200x630, the day tape),
  `og:image` with the full address taken from the request, a large Twitter
  card, an app icon and a web manifest. The favicon uses the ink color.

## Data model

- `0015_create_password_resets.sql`: token hash, user, expiry, used_at.
- `0007_create_widget_settings.sql`: `buffer_minutes` defaults to 0, so open
  times only change when a host chooses a gap.

## API

| Route | Access | Result |
| --- | --- | --- |
| `POST /api/auth/forgot-password` | public | always 200; queues `password_reset` for a known address |
| `POST /api/auth/reset-password` | public | 200; 400 when the link is unknown, used or older than an hour |
| `PUT /api/auth/me` | signed in | the `SessionUser` with the new name |
| `POST /api/auth/password` | signed in | 200; 400 when the current password is wrong |
| `PUT /api/host/settings` | host | now with `buffer_minutes`, 0 to 120 |

**Limits.** Fixed windows in `rate_limits`, answered with 429: login 10 per
10 minutes per address and per email; sign-up 5 per hour; forgot-password 5
per hour per address and 3 per email; booking 10 per hour; lead saves and
manage-link requests 60 per 10 minutes each.

**Gap between calls.** With a gap set, a time is offered only when it leaves
that many minutes free before and after every booked call. Calendar events
keep their existing 15 minutes, or the gap when it is larger. A call being
moved does not block its own neighbours.

**Emails.** After 20 failed tries a row is left alone and listed as Failed.
A row that sends clears its earlier error.

## Known limits

Plan 16 closed four limits first listed here: a password change now signs out
other devices, the reset link is cleared from the queue once sent, the proxy
overwrites `X-Forwarded-For`, and a call's own calendar event no longer blocks
moving it. Plan 16 also changed the login limit to 5 per 10 minutes per email
and address together.

- The email address cannot be changed.
- One preview image for every page. An image per host needs an image
  rendering dependency.

## Tests

- Checks from `AGENTS.md`.
- `backend/tests/auth_http.rs`: reset works once and expires, an unknown
  address queues nothing, profile and password changes, login and
  forgot-password limits.
- `backend/tests/host_booking_http.rs`: the gap removes and returns
  neighbouring times and allows a move to the next slot; a row past the retry
  cap is not picked up.
- Frontend: `weekDays`, the cancelled list in `splitBookings`.
- Browser walk-through: Bookings week view and Cancelled tab, admin emails on
  a phone, Settings rules and Account (name saved, password changed and used
  to log in), forgot-password page, reset page with a dead link.

## Definition of done

- A forgotten password can be replaced from an emailed link that works once.
- Repeated attempts on any public form are refused with a clear message.
- With a gap set, no two calls can be booked closer than the gap.
- No email is retried forever.
