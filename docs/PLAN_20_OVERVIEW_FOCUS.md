# Plan 20: an overview that leads with now

Status: implemented. Typecheck, lint, knip, tests and build pass. Not yet
clicked through in a browser.

## Goal

A host opens `/dashboard` and sees what matters first: anything that stops
people from booking, then the call under way or the next one today, with
its Join button.

## The gap this fixes

- Three cards had the same weight (the Today strip, Next up, and Your booking
  page), so nothing said what to look at first.
- Today's calls were drawn on the strip and listed again under "Next up",
  and the two weren't linked.
- The strip's tinted bands had no legend, and its call blocks couldn't be
  clicked.
- A paused page or missing hours was the last row on the page.
- The strip read only weekly rules, so a date taken off (Plan 19) still
  showed as open.

## Approach

No new dependency, token or migration. Existing shadcn `Alert`, `AgendaRow`,
`hoursOn`, `callPlace`, `useNow`.

```
Welcome back, Ana                 cadence.app/book/ana  [Copy link] [Open]

┌ ! Your booking page needs one fix ───────────────────────────────────┐   only when broken
│   Your booking page is paused, so nobody can book you. [Take bookings again] │
└──────────────────────────────────────────────────────────────────────┘
┌ Today, Friday, October 16 ──────────────────────────── 2 calls left ┐
│ (Next)  2:30 PM   Mia Chen / in 25 min                 [Join call]  │
│ 9 AM    10     11     12 PM   1   ▓Mia ┃now   3    ▓Leo    5       │
│ ░ Open for booking   ▓ Booked   ▢ Done                              │
│ ▌2:30 PM  Mia Chen   mia@…                         [Join call]     │
│ ▌4:00 PM  Leo Park   leo@…                         [Join call]     │
└─────────────────────────────────────────────────────────────────────┘
┌ Later ───────────────────────────────────────────── All bookings ───┐
│ Tomorrow  ▌10:00 AM  Sara Kim                                       │
└─────────────────────────────────────────────────────────────────────┘
Mon to Fri, 30 min calls · Tbilisi · Google Meet links   [Edit hours] [Settings]
```

## Implementation sketch

- `domains/bookings/bookings.ts`: `dayPlan()` gives today's calls, the call
  under way, the next call today, and calls on later days.
- `domains/dashboard/booking-page-status.tsx`: the pure function
  `bookingPageProblems()`, plus `SetupProblems` (an alert, only when something
  is wrong) and `SetupSummary` (one line at the bottom). The old
  `BookingPageStatus` card is gone.
- `domains/dashboard/today-card.tsx` (was `today-tape.tsx`): a spotlight
  line, then the strip with clickable blocks and a legend, then today's list.
  Open hours come from `hoursOn`, so overrides count.
- `routes/dashboard/index.tsx`: the booking link moves into the header, and
  "Next up" becomes "Later" (days after today).

## Tests

- `bookings.test.ts`: `dayPlan` ordering, current and next, later days, and
  today on the host's clock.
- `booking-page-status.test.ts`: `bookingPageProblems`.

## Definition of done

- Setup problems show above the calls, and only when there are any.
- Today's calls show in one place, and the next call can be joined from the
  top of the card.
- A date taken off shows as off on the strip.
