# Plan 19: Availability you draw, and dates that differ

Status: implemented. Drawing, stretching, the arrow keys and taking a date off
were clicked through in Chrome at 1440px and 390px. Dragging on a real touch
screen was not tried.

## Goal

A host shapes their week by drawing on it, sees how many calls each day opens
while they draw, and can take a single date off or give it other hours.

## The gap this fixes

- The week ruler was a picture. Every change went through a form: "Add hours",
  From, To, Call length, "Also add to".
- A range could not be moved or stretched, only retyped.
- There was no way to say "off on the 24th" or "only 10 to 1 on the 28th".
  Hosts had to delete their weekly hours and remember to put them back.
- Nothing said how many calls the hours actually opened.

## Approach

No new dependency. Existing tokens only: day phases (`dawn`, `noon`, `dusk`),
`text-clock`, `HourMark`, shadcn `Popover`, `RadioGroup`, `DropdownMenu`.

```
Availability                                             [Change timezone]
Open 40 hours a week, Mon to Fri: up to 80 calls of 30 min. Times are in Tbilisi.

┌ Weekly hours ────────────────────────────────────────────────────────────┐
│             7 AM    9       11      1 PM    3       5       7            │
│ ⋯ Monday     ┃░░ 9:00 to 12:00 ░┃  ┃▒▒ 1:00 to 5:00 ▒▒┃          14 calls │
│ ⋯ Saturday     ·  ·  drag across to add hours  ·  ·                  Off │
│                     ┌ 3:00 PM to 5:00 PM, 4 calls ┐   (while dragging)   │
└──────────────────────────────────────────────────────────────────────────┘
┌ Dates ──────────────────────────────── October 2026   [‹] [›] ──────────┐
│  Mon     Tue     Wed     Thu     Fri     Sat     Sun                     │
│  19      20      21      22      23      24 Off  25                      │
│  ▬▬▬▬    ▬▬▬▬    ▬▬▬▬    ▬▬▬▬    ▬▬▬▬    ┊    ┊  ▬▬      (dusk outline)  │
│ Dates set apart: [Saturday, October 24 off]                              │
└──────────────────────────────────────────────────────────────────────────┘
 a date → Popover: (•) Weekly hours  ( ) Off all day  ( ) Other hours
```

- **Draw.** Press on an empty part of a day's track and drag. The range snaps
  to 15 minutes. A bubble shows the range and its calls, and the day's count
  on the right follows. Release saves. A tap opens the form at that hour.
- **Move and stretch.** Drag a range to move it, its ends to stretch it.
  Every move is measured from where the press began (`dragSpan`), so the
  range stays under the pointer, and a pointer drag stops at the ends of the
  ruler. A dashed outline marks where the range was picked up; Escape lets go
  and leaves it there. A range that lands on or touches another is folded
  into it (`foldInto`), so a day never holds two ranges over the same time.
  The range about to be folded in is outlined, and the bubble shows the
  range, length and calls that letting go will save.
- **Guide.** A resting mouse over an empty part of a track shows a line and
  the time under it.
- **Keyboard.** Ranges are buttons. ← → move by 15 minutes, Shift changes the
  end, Alt the start. The change is saved once the keys go quiet. Enter opens
  the form, Delete removes the range with Undo.
- **Day menu.** Add hours, copy to Mon to Fri, copy to every day, clear.
- **Dates.** A month grid in the host's timezone. Every date draws the hours
  it will really have as a thin tape. A date set apart is outlined in dusk.
- The ruler shows an hour either side of the week's hours, so a range can
  grow past them by dragging. Anything further goes through the form.
- Every weekly change shows at once and is put back if the server refuses it.

## Data model

`migrations/0016_create_availability_overrides.sql`: `availability_overrides`
(`host_unid`, `date`, `start_time`, `end_time`, `slot_minutes`). A day off is
one row with no times; other hours are one row per range. A check constraint
allows only those two shapes. `date` is a wall-clock date in the host's
timezone.

## API

| Method | Path | Result |
| --- | --- | --- |
| `GET /api/availability/overrides` | host | `DateOverride[]` from yesterday (UTC) on |
| `PUT /api/availability/overrides/{date}` | host | `DateOverride`; body `DateOverrideInput { ranges }`, none for a day off |
| `DELETE /api/availability/overrides/{date}` | host, own | 204; the date follows the weekly hours again |

A date is replaced as a whole in one transaction. Ranges on one date must not
overlap, and the date must be within a year.

## Flow

`slots_between(rules, overrides, timezone, from, to)` walks local dates. A date
with an override uses its ranges (none: no calls); every other date uses its
weekday's rules. `bookings::service::open_slots` loads both, so the public
page and the invitee's reschedule both follow dates set apart.

## Files

- Backend: `domain/availability/{models,db,service,handlers,routes}.rs`,
  `domain/bookings/service.rs`.
- Frontend: `domains/availability/` holds the page's parts, which outgrew the
  route file: `hours-math.ts`, `use-hours.ts`, `hours-form.tsx`,
  `week-canvas.tsx`, `dates-calendar.tsx`, `date-editor.tsx`.
  `routes/dashboard/availability.tsx` composes them.

## Tests

- `availability/service.rs`: a day off closes only that date; a date's own
  hours replace the weekly ones; a date is read in the host's timezone; rows
  group into dates.
- `host_booking_http.rs`: put, read and remove a date; overlapping ranges
  refused; public slots follow; another host can neither see nor remove it.
- `hours-math.test.ts`: snapping, pointer to time, call counts, folding ranges,
  a date's hours, and drags measured from the press and kept on the ruler.

## Not in scope

- The bookings calendar and the booking page status still read weekly hours
  only.
- Several named schedules (Calendly's "schedules"); one week per host.

## Definition of done

- [x] Migration, API, slot generation and tests.
- [x] Draw, move, stretch, keyboard, day menu, call counts.
- [x] Dates calendar and editor.
- [x] Seed, `request.http`, bindings.
- [ ] Tried on a touch screen.
