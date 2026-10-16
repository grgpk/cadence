# Plan 22: one slot policy, one host calendar seam

Status: implemented. Backend fmt and tests pass (95 unit, 15 auth, 39 booking
HTTP); bindings unchanged. `cargo clippy -D warnings` already failed before
this change (274 pedantic findings on a clean checkout); this change adds
only doc and `must_use` findings of the same kind.

From the architecture review (candidates 1 and 2). Terms are in `CONTEXT.md`.

## Goal

A visitor sees exactly the times the host can take, and those rules can be
tested without HTTP and Postgres. A host with a connected calendar is
tested at all.

## The gap this fixes

- "Which times are open" was split over three domains. Bookings undid Google's
  15 minute event buffer before applying its own gap.
- The clock was read in eight places; the rules had no unit tests.
- Create and reschedule repeated "find the slot, then lock, then check again".
- Google Calendar had no seam, so nothing about a connected calendar (busy
  time, events, Meet links, moving and deleting) was tested.

## Approach

No new dependency, migration or wire type.

```
handlers ──▶ bookings::service ──▶ bookings::slots ──┬─▶ availability (hours, overrides)
                                                    ├─▶ bookings::db (booked calls, host lock)
                                                    └─▶ AppState.calendar ─┬─ Google adapter
                                                                           └─ in-memory adapter
```

- **Slot policy** (`bookings/slots.rs`). Two functions:
  - `open_slots(pool, calendar, page, now, moving)`: the times on offer. A
    paused page offers nothing.
  - `reserve(pool, calendar, page, start, now, moving)`: the open time at
    `start`, returned with the transaction that holds the host lock, or
    "this time is no longer available". The calendar is read before the
    transaction starts, as before.
  - Inside: one gap rule for booked calls, and `max(15 minutes, gap)` around
    calendar events (unchanged from Plan 15). A broken calendar is logged and
    counts as no busy time. The rules are a pure core with unit tests.
- **Host calendar** (`google_calendar/calendar.rs`). `enum HostCalendar
  { Google, InMemory }` in `AppState`, with `busy`, `create_event`,
  `move_event`, `delete_event`. Not connected is a normal answer: no busy
  time, no event, nothing to change. The Google adapter owns tokens, HTTP and
  Meet links; bookings no longer reads `page.calendar_connected` (kept for
  display).
- `availability::overrides_around(pool, host, today)` takes the date instead
  of reading the clock.

## Tests

- Unit, slot policy: notice and window, date overrides, gap around calls, a
  moving call, a call just outside the window, calendar event gap below and
  above 15 minutes, a call's own event, a paused page.
- Unit, calendar: all-day and timed event instants, the in-memory adapter,
  event end times.
- HTTP, with the in-memory calendar: busy time closes nearby times and a
  broken calendar still books; a connected host's call gets an event and a
  Meet link that the manage page and emails carry, and the event follows a
  move and a cancel.
- Removed as covered by the unit tests: the gap and window HTTP tests (their
  validation checks live on in `booking_rules_are_checked_and_stay_off_the_public_page`)
  and the gap half of the move test.

## Definition of done

- [x] One module decides open times; create and reschedule reserve through it.
- [x] The calendar is behind a seam with a Google and an in-memory adapter.
- [x] The connected-calendar paths have tests.
- [ ] Booked against a real Google Calendar after deploy.
