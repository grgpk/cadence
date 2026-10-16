# Plan 21: week and list views on the booking page

Status: implemented. Typecheck, lint, knip, tests and build pass. Screenshots
checked at 1440, 900 and 390px, dark and light. Clicked through in headless
Chrome from each view to the details step and back; no booking submitted.

## Goal

A visitor on `/book/$slug` can see open times the way that suits them: the
month calendar (unchanged), a week of days side by side, or a list of the
soonest times. In the list, the earliest time is one click away.

## The gap this fixes

- Every pick took two clicks (a day, then a time), even for "as soon as
  possible".
- Only one day's times were visible at once, so comparing days meant
  clicking through the calendar.

## Approach

Frontend only: `GET /slots` already returns every open slot in the booking
window. No dependency, token, migration or binding change. The month view's
`SlotStep` is not changed. The only addition on its screen is the view switch
in the summary column.

```
┌────────────┬──────────────────────────────────────────────┐
│ host       │ Oct 17 – 23                ‹ ›   12h | 24h   │
│ title      │ Sat   Sun   Mon   Tue   Wed   Thu   Fri      │
│ ◷ 30 min   │ 17    18    19    20    21    22    23       │
│ 🌐 tz      │ ● Morning                                    │
│            │   ─     ─   ▌11:00▌11:00▌11:00▌11:00▌11:00     │
│            │ ● Afternoon                                  │
│            │ ▌12:00▌12:00▌12:00▌12:00 …                    │
│[Month|Week|List]                                          │
└────────────┴──────────────────────────────────────────────┘
List: "Soonest open times", then Tomorrow / Sunday, October 18 / …, each
day with all its times, 5 days at a time with "Show more days".
Below 760px the week is day columns to swipe through (the next one peeks in);
the list stacks with 3 chips a row.
```

- The week is a timetable: one row per time of day, so the same time lines up
  across days ("Tuesday at 2" is a glance across), a dash where a day is not
  open then, and Morning / Afternoon / Evening rows in the phase colors.
- The week is seven days in a row, starting on the first open day, not on a
  Monday, so it never opens on days that are already gone. ‹ › skip stretches
  with nothing open. A day picked on the month calendar opens the week that
  holds it, so Fri Oct 23 shows Oct 17 – 23, not a week starting on Friday.
- `?view=week` and `?view=list` open the page on that view (month needs no
  value), so a host can share a link that opens on a given view, and a
  refresh keeps the view.
- Chips keep the month view's dawn/noon/dusk rail and fill on pick.
- The switch (`view-switch.tsx`) is three equal parts filling the column, with
  one highlight that slides between them. A switched view slides in like a step.
- On phones the page starts at the top instead of centering the card.
- Beside the summary, week and list both scroll inside the card (day headings
  stay pinned, the last times fade out), so all three views keep the card about
  the month calendar's height and the page does not move. On a phone the page
  scrolls as usual.
- The list ends with "Those are all the open times in the next N days."

## Implementation

- `frontend/src/domains/widget/slot-days.ts`: `SLOT_LAYOUTS`, grouping by local day,
  week and day labels, shared call length.
- `view-switch.tsx`, `time-chip.tsx`, `slots-frame.tsx` (heading, clock choice, loading/error/empty),
  `week-slots.tsx`, `list-slots.tsx`.
- `booking-card.tsx` picks the view; `routes/book/$slug.tsx` validates `?view=`.

## Tests

- `slot-days.test.ts`: grouping across zones and a DST change, week span and
  labels, Today/Tomorrow, layout guard, call length, timetable rows.

## Definition of done

- [x] Month view looks as before, apart from the switch.
- [x] Week and list views at desktop and phone widths.
- [x] Pick a time in week and list, reach the details step, "Change time"
      returns to the same view.
- [ ] Submit a booking from week and list against a real host.
