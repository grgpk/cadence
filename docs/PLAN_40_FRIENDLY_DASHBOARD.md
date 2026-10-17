# Plan 40: a friendlier host dashboard

Status: implemented.

## Goal

The dashboard feels warmer and calmer without a second design system. Cards
read as raised, loading looks like what is coming, and the overview greets the
host for the part of their day.

## The gap this fixes

- Light mode put white cards on a white page; dark mode was pure gray.
- No shadow scale: `shadow-xs`, `shadow` and `shadow-lg` were mixed on cards.
- Loading showed generic blocks, so content jumped when it arrived.
- `text-[0.9375rem]` was an unnamed step in the type scale (18 uses).
- Card heading bars were hand-built in each place and drifted.
- The overview always said "Welcome back".

## Tokens

- `--canvas`: the dashboard page behind its cards. Light `oklch(0.982 0.004 265)`,
  dark `oklch(0.155 0.006 280)`. Only `SidebarInset` uses it; public pages stay
  on `--background`.
- Dark surfaces carry a trace of dusk (hue 280, chroma under 0.01). This revises
  Plan 12's "only the navy tint is gone".
- `--shadow-card` (from `--card-shadow`): the one card shadow, set on `Card`.
  Popovers and sheets keep `shadow-lg`.
- `--radius` 0.75rem. Cards `rounded-xl`, rows `rounded-lg`, chips `rounded-md`.
- `--text-md`: 0.9375rem / 1.45.

## Flow

```
┌───────────┬──────────────────────────────────────────────┐
│ ● Cadence │  Dashboard / Overview                 ◐      │  top bar on canvas
│▐Overview  ├──────────────────────────────────────────────┤
│  Bookings │  Good afternoon, Ana        link  [Copy]     │
│  ...      │  ▬▬ noon rail                                │
│           │ ┌ Today ───────────────────────────────────┐ │  cards lift off
│           │ └──────────────────────────────────────────┘ │  the canvas
│           │ ┌ Later ───────────────────── All bookings ┐ │
└───────────┴──────────────────────────────────────────────┘
 ▐ active item: card surface, card shadow, dusk icon
```

## Implementation

- `styles/globals.css`: tokens above. `lib/color.ts` and the `theme-color` meta
  follow the new dark page.
- `PanelHeader` and `PanelCardSkeleton` in `domains/shared/panel-card.tsx`;
  used by the overview, insights, today, availability, settings and apps pages.
- Event types load as `TypeRowSkeleton`, the shape of a `TypeRow`.
- `EmptyState` icon tile in dusk.
- `domains/dashboard/greeting.ts`: salute from `dayPhase` on the host's
  timezone; `PageHeader` takes an `accent` for the phase rail.
- Button press scale is now `motion-safe:` only.

## Tests

- `greeting.test.ts`: host timezone wins over the browser's; changes at 12:00
  and 17:00.
- Screenshots of Overview, Event types, Settings and Bookings, light and dark,
  desktop and 375px.

## Done

All frontend checks pass, and both themes read as raised cards on a canvas.
