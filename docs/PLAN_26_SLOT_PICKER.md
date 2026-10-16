# Plan 26: one slot picker for booking and rescheduling

Status: implemented. Frontend typecheck, lint, knip, tests (173) and build
pass. Checked in the browser: the booking page's month view is unchanged; the
reschedule page offers month, week and list; a 24h choice on the booking page
carries to the reschedule page; a pick there can be released with "Keep
looking". No call was moved.

From the architecture review (candidates 6 and the one useful part of 7).
Terms are in `CONTEXT.md`; candidate 7 itself is declined in ADR 0001.

## Goal

An invitee moving a call picks a time the same ways as when booking it, and
how a time is picked is decided in one place.

## The gap this fixes

- The month view and the week and list views each had their own pick timer
  and their own loading, error and empty states; they had drifted: only the
  month view let go of a pick that failed, or said the page was paused.
- The reschedule page offered the month view only.
- The booking card and the reschedule page each kept the month view's
  position and passed it back in; the booking card also cleared it on a
  timezone change.
- The 12h/24h choice lived in the booking card's localStorage; the reschedule
  page ignored it and started from the device's style.
- Phase colours were exported from a widget UI file and imported by bookings.
- "Refresh this page's open times" was written four times.

## Approach

No new dependency.

```
booking card ──┐                    ┌─ MonthSlots ─┐
               ├─▶ SlotPicker ──────┼─ WeekSlots  ─┼─▶ usePick, slotsStatus
manage page  ──┘   (where you are)  └─ ListSlots  ─┘   slot-picker-rules
```

- `SlotPicker` (`domains/widget/slot-picker.tsx`) keeps where the visitor is:
  the month and day, which week opens, the list's pages, and the pick. Pages
  give it the view, the clock, the timezone, an optional summary column and
  the wording. A new timezone starts over from the first open month.
  `ViewSwitch` stays separate: in the summary on the booking page, above the
  picker on the reschedule page.
- `slot-picker-rules.ts`: the opening month, the shown day, month and week
  steps, list pages and the one-pick rule, as plain functions.
- `slotsStatus` and `usePick(onPick, releaseKey)` serve all three views.
- `slot-step.tsx` becomes `month-slots.tsx`; its month opens from the rules
  instead of an effect that jumped once the times arrived.
- `useClockPreference()` (`hooks/`): one remembered choice for every page,
  24h on the server until the page is in the browser. The confirmation page
  keeps reading `h12` from its address.
- `PHASE_STYLES` moves to `domains/shared/day-phase-styles.ts`.
- `refreshOpenTimes(queryClient, slug)` next to `publicSlotsQuery`.

## Tests

- `slot-picker-rules.test.ts`: opening month, shown day and its fallback,
  month steps, week opening and steps (moved from `slot-days.test.ts`), list
  pages, the one-pick rule.

## Definition of done

- [x] One picker on both pages, with month, week and list on both.
- [x] One pick, one set of states; a failed pick is released in every view.
- [x] One clock preference across pages.
- [ ] Move a real call from the week and list views after deploy.
