# Plan 8: UI refresh

Status: done.

## Goal

Every page looks like one product: quiet monochrome surfaces, hairline borders,
tight display type, and a booking card as the centerpiece. A visitor picks a
time before filling in a form. A host can edit weekly hours.

## The gap this fixes

Three color systems coexisted (shadcn tokens in the app, hardcoded `slate-*`
and `#f52326` in the widget, cream on legal pages). Inter was named but never
loaded, dark mode was defined but unreachable and broken, dashboard pages had
no padding, lists were stacked divs with raw dates and status strings, and
availability was read-only.

## Approach

This restyles the existing shadcn system through its tokens. It is not a second
design system.

- **Color.** Ink primary on white cards over a mist background. One `--signal`
  red token, used only for the selected day, the availability dots, and the
  logo mark. `--success` and `--warning` for status. No hardcoded colors
  outside the Google Meet icon.
- **Type.** Funnel Display for titles, Hanken Grotesk for everything else,
  self-hosted through `@fontsource-variable/*` so the embeddable widget makes
  no third-party request. `tabular-nums` on times, dates, and counts.
- **Shape.** Controls 8px, cards 12px, the booking card 16px. Shadow only on
  the booking card and popovers.
- **Theme.** Follows the system setting, with a toggle in the public header and
  the app's top bar (moved there from the user menu in Plan 10). `lib/theme.ts` sets the class before paint; no dependency.
- **Signature.** Days that can be booked carry a small red dot, so the month
  reads as a rhythm. The logo is three uneven bars for the same reason.

## Screens

```
Landing
┌──────────────────────────┬────────────────────────────────────┐
│ headline, one CTA        │ booking card (live)                │
├──────────────────────────┴────────────────────────────────────┤
│ How it works: 1 Set hours  2 Share link  3 Get booked         │
└───────────────────────────────────────────────────────────────┘

Booking card, step 1            Booking card, step 2
┌─────────┬──────────┬───────┐  ┌─────────┬──────────────────┐
│ summary │ calendar │ times │  │ summary │ details form     │
│         │  ·  ·  · │       │  │ + time  │ per-field errors │
└─────────┴──────────┴───────┘  └─────────┴──────────────────┘

App shell (dashboard and admin)
┌──────────────┬────────────────────────────────────────────┐
│ logo         │ ☰  Area / Page                             │
│ nav          │  page header, then cards and tables        │
│ user menu    │  (max-w-5xl)                               │
└──────────────┴────────────────────────────────────────────┘
```

## Implementation sketch

- `frontend/src/styles/globals.css`: tokens, fonts, repaired `.dark`.
- `frontend/src/lib/`: `routes.ts` (route map), `format.ts` (dates, times,
  weekday names), `theme.ts`.
- `frontend/src/domains/shared/`: `AppShell`, `PageHeader`, `EmptyState`,
  `ErrorState`, `DataTable`, `TableSkeleton`, `StatusBadge`, `PublicLayout`,
  `AuthCard`, `LegalPage`, `OAuthCallback`, `Logo`, `ThemeToggle`, `NotFound`.
- `frontend/src/domains/widget/`: `BookingCard`, `SlotStep`, `DetailsStep`,
  `BookingCalendar`, `BookingSummary`, `booking-form.ts`. The card renders
  after mount because timezone and today's date only exist in the browser.
- `frontend/src/routes/dashboard/availability.tsx`: weekly editor.
- New shadcn primitives: `checkbox`, `radio-group`, `tabs`, `textarea`,
  `sonner` (new dependency, for toasts).
- Backend: `DELETE /api/availability/{id}`, documented in Plan 3.

## Not done here

- The Google Calendar connection belonged to the single public host, so its
  card stayed on `/admin`. Plan 7 made it per host and Plan 10 placed it in
  `/dashboard/settings`, next to the meeting link.
- Event types, per-host booking URLs, cancel and reschedule, settings,
  password reset, table sorting and pagination.
- Availability times are stored and shown in UTC, as the slot service reads
  them. Per-host timezones belong to Plan 7.

## Tests

- Checks from `AGENTS.md`.
- Browser walk-through at 375px and 1440px, light and dark: pick a day and
  time, trigger form errors, confirmation page, 404, login, dashboard,
  bookings tabs, add and remove hours, each admin list.

## Definition of done

- No hardcoded colors in `frontend/src` outside `components/ui` and the Meet
  icon.
- Every list has loading, empty, and error states; errors offer a retry.
- Dark mode is readable on every page.
