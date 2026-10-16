# cadence, domain glossary

The words the code, plans and reviews use for the booking domain. One
meaning each; when a term changes, change it here first.

## Booking

- **Host**: the person whose time is booked. Has one booking page.
- **Booking page**: a host's public link, `/book/{slug}`. It can be paused;
  a paused page offers no open times.
- **Open time** (code: `TimeSlot`): a start and a length a visitor can book
  right now. Every open time comes from the host's hours, inside the booking
  window, after the notice period, and clear of calls and calendar events by
  their gaps.
- **Slot picker**: how a visitor picks an open time, on the booking page and
  when moving a call: a month calendar, a week timetable, or a list of the
  soonest times. One pick at a time; a pick that does not go through is
  released.
- **Clock preference**: 12h or 24h, chosen by the visitor and remembered on
  their device; the device's own style until they choose.
- **Hours**: the host's weekly hours, plus **date overrides** that give one
  date other hours or none.
- **Notice period**: how soon before a call it can still be booked.
- **Booking window**: how many days ahead a call can be booked.
- **Gap** (code: `buffer_minutes`): the free time a host keeps before and after
  every booked call.
- **Calendar event gap**: free time kept around events on the host's
  connected calendar: 15 minutes, or the host's gap when that is larger.
- **Moving call**: the call being rescheduled. Its own time does not close
  the open times next to it.
- **Slot policy**: the one module that decides open times, and reserves one
  for a booking or a move (`bookings/slots.rs`).
- **Reserve**: take an open time for a booking or a move: check it is open,
  then lock the host and check again inside the transaction that writes it.
- **Host calendar**: the calendar a host connected (Google today). Cadence
  reads its busy time and keeps an event there for each call. Not connected
  is a normal state, not an error.
- **Booking session**: one visitor's visit to a booking page, from landing to
  a booked call: the visit record, the lead and its autosaved answers, and
  the booking. The lead is submitted once per session.
- **Lead**: the visitor's details and answers, saved as they type and
  submitted when they first try to book.
- **Calling visit**: the record of one visit to a booking page (source, UTM,
  whether the form was submitted and a call booked). Never allowed to block
  a booking.
- **Manage token**: the private link an invitee uses to reschedule or cancel.

## Time

- **Civil day**: a date on someone's calendar with no time or zone attached
  (`lib/civil-day.ts`). Its key is the ISO date, `2026-10-05`, the same form
  the API uses for date overrides. Weeks start on Monday.
- **Wall clock**: what an instant reads in a given IANA timezone
  (`lib/zoned-time.ts`). Times are shown on the visitor's clock on the booking
  page and on the host's clock in the dashboard.

## Email

- **Email queue**: the `email_queue` table and the module that owns it. It
  alone decides which emails a call event sends, to whom and when, and it is
  the only writer of the table.
- **Call event**: what happened to a call, told to the email queue inside the
  transaction that made it happen: booked, rescheduled or cancelled (by the
  host or the invitee).
- **Reminders**: the invitee's emails 24 hours, 2 hours and 30 minutes before
  a call, queued only while still ahead. A move or a cancel drops the ones
  still waiting.
