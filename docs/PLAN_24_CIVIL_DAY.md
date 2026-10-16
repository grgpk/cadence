# Plan 24: one civil day module

Status: implemented. Frontend typecheck, lint, knip, tests (155) and build
pass. Checked in the browser: booking page month and week, dashboard
calendar day, week and month, availability dates calendar.

From the architecture review (candidate 4). Terms are in `CONTEXT.md`.

## Goal

A date on a calendar means one thing in the frontend: one type, one key, one
way to step through days, weeks and months, and one way to write them.

## The gap this fixes

- Five representations of a day (`CivilDay`, `WeekDay`, `ZonedTime`, the
  month view's `CalendarView`, ISO strings) and six day keys, some padded and
  some not, with padded keys compared by `<`.
- Two `weekDays` with different meanings, two month grids (one on the local
  `Date`), three copies of the Monday-first weekday, and the `Date.UTC` trick
  repeated to format a day.
- Two week label styles: "Oct 19 – 25" on the booking page, "12 to 18 October"
  (en-GB, day first) on the dashboard.
- `zoned-time.ts` lived in the booking widget but was imported everywhere.

## Approach

No new dependency.

- `lib/civil-day.ts`: `CivilDay` with the ISO key (`dayKey`, `fromKey`, the
  API's date form), `dayOfInstant`, `todayIn`, `addDays`, `daysBetween`,
  `sameDay`, `weekdayIndex`, `mondayOf`, `weekFrom`, `daysInMonth`,
  `shiftMonth`, `monthWeeks(year, month, { rows: "fit" | "six" })`, and the
  labels (`monthLabel`, `weekLabel`, `shortDayLabel`, `longDayLabel`,
  `shortWeekday`, `relativeDayLabel`) on `lib/format`'s locale.
- `lib/zoned-time.ts`, moved from `domains/widget/`.
- Removed: `widget/calendar-grid.ts`, `WeekDay`, `weekDays`, `dayOf`,
  `monthGrid` and the private keys in `bookings.ts`, `isoDate` and
  `weekdayOf` in `hours-math.ts`, the private helpers and en-GB formatters in
  `calendar-view.tsx`, the day helpers in `widget/slot-days.ts`.
- The week and list views read today from `useNow`, so it moves on at midnight.

Visible change, as agreed: week labels read "Oct 19 to 25" everywhere; the
dashboard calendar's day reads "Friday, October 16" (was "Friday 16 October").
The booking page's month view is unchanged.

## Tests

- `lib/civil-day.test.ts`: keys both ways, today either side of midnight in
  three zones, days across month, year and clock change, weekdays and Mondays,
  `monthWeeks` fit and six (a five-week month, a six-week month, a month
  starting on Monday), month steps across a year, every label.
- Moved, not copied: the day cases from `bookings.test.ts`,
  `slot-days.test.ts`, `calendar-grid.test.ts` (deleted with its module) and
  `hours-math.test.ts`. `zoned-time.test.ts` moved with its module.

## Definition of done

- [x] One day type and one key across widget, bookings and availability.
- [x] One month grid, one week label.
- [x] No day helpers left outside `lib/civil-day.ts`.
