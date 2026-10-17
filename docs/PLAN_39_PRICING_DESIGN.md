# Plan 39: pricing and billing design

Status: done.
- Frontend: build, typecheck, lint, knip and tests pass.
- Checked in headless Chrome:
  - `/pricing` at 1440, 1280, 768 and 390 wide, in light and dark;
  - `/dashboard/billing` at 1280 and 390, signed in as the seeded host on trial.

## Goal

Pricing and Billing should look like Cadence, not a template, and read well on a phone.
The page spends its boldness in one place, the week sizer. Everything else gets quieter.

## Gap

- The week sizer was a plain form.
- The plans were three identical cards, and on a phone they made one long scroll.
- The FAQ used half the page width.
- "14 days of Pro" was said three times.
- The current plan on Billing was a list of rows.

## Changes (frontend only)

- **Week grid** (`pricing/week-grid.tsx`):
  - Monday to Friday, 6 slots a day: 30 slots, which is `MAX_CALLS_PER_WEEK`.
  - Slots are booked in a fixed order, so each step of the slider adds exactly one block.
  - Open slots use the day tape's `OPEN_STYLES`.
  - It is `aria-hidden`, because the slider already announces the count.

```
     Mon  Tue  Wed  Thu  Fri
 9   ▇▇   ░░   ▇▇   ░░   ░░
10   ░░   ▇▇   ░░   ░░   ▇▇
 …
```

- **`PlanRow`** (`pricing/plan-card.tsx`):
  - One bordered box holds the plans.
  - It has 3 columns when the box is at least 42rem wide (a container query), and stacks below that.
  - The plan that fits, or your plan on Billing, gets a dusk tint and a dusk bar on top. No ring, no shadow.
- **Pricing page:**
  - The period and compare toggles are full width on phones.
  - The FAQ has its heading in a sticky left column.
  - The repeated trial line under the plans is gone.
- **Billing:**
  - The current plan has two zones: on the left, the plan name and status; on the right, the bookings this month as a large number with a meter.
  - The plans use `PlanRow`.
- **`SegmentedControl`** takes a `className`, so a caller can stretch it to full width.

## Definition of done

- At 390px nothing scrolls sideways, and the grid fills one block per slider step.
- With reduced motion on, no block drops in and no bar grows.

## Second pass

- **The grid shows where Free runs out.** Calls past Free's allowance turn
  dusk, the colour of "Fits your week". A short key under the grid explains
  the two colours, and it shows only once the week goes past Free. The
  allowance comes from `freeCallsPerWeek(offers)`: with Free's 25 a month,
  that is 5 calls a week.
- **The pricing hero has two columns.** On wide screens the headline and the
  promises sit on the left, with the sizer beside them. On phones the order
  is headline, sizer, promises.
- **The trial is a strip of days.** During a trial, Billing shows 14 day cells,
  with the days left in dusk. The bookings this month read against Free's
  limit, for example "2 of 25 on Free". `trialFreeLimit` replaces
  `freeAllowanceLine`.
