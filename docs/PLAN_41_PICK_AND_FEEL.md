# Plan 41: a pick-a-call page that leads into booking, and one feel for touch

Status: implemented.

## Goal

A visitor who lands on a host's link with several calls sees the same card,
avatar and accent as the booking flow, and the host's header carries over when
they pick a call. Across the app, anything clickable gives a little under the
pointer and anything that opens, opens smoothly.

## The gap this fixes

- `/book/<slug>` with several event types floated bare cards under a small
  grey host name. It looked nothing like the booking card it led to.
- Only `Button` had press feedback. App cards and calendar days had none.
- The sidebar "Apps" group and webhook deliveries snapped open while the
  Accordion animated.
- Some hover colours changed with no transition (calendar view, today strip,
  public footer links).

## Data model and API

None. `HostPage` already has the host name, accent and event types.

## Flow

```
┌──────────────────────────────┐        ┌───────────┬────────────┬───────┐
│ (C) Cadence Host             │  pick  │ (C) Host  │ October    │ times │
│ Pick a call                  │ ─────▶ │ Deep dive │ calendar   │       │
├──────────────────────────────┤ header │ 60 min    │            │       │
│▌ Book a coaching call      › │ morphs └───────────┴────────────┴───────┘
├──────────────────────────────┤
│▌ Career deep dive          › │  ▌ host accent bar on hover/focus
│  [⏱ 1 h]                     │
└──────────────────────────────┘
```

## Implementation

- `domains/widget/host-badge.tsx`: `HostBadge`, the avatar and name, with
  `vt-host`. `BookingSummary` and `HostTypesPage` both use it, so the view
  transition the router already runs morphs one into the other.
- `BOOKING_FRAME` in `booking-card.tsx`: the booking card's frame, shared with
  the pick-a-call card.
- `HostTypesPage`: one framed card, rows divided, a 3px `bg-primary` bar on
  hover and focus (the host accent through `.host-accent`), a duration chip
  when the type has a fixed length.
- `globals.css`: `@utility vt-host`, a 0.3s `host` group timing, and
  `@utility press`, which scales clickable surfaces to 0.98 on `:active` only
  when motion is allowed. Used on `AppCard` and booking calendar days. Rows
  inside a card darken on press instead of scaling.
- Collapsibles in `app-shell.tsx` and `webhooks-panel.tsx` use
  `animate-collapsible-down/up` from tw-animate-css. The shadcn primitive is
  unchanged.

## Tests

No logic changed. Frontend checks, plus a browser pass: hover and tab through
the pick-a-call rows, pick one and watch the header morph, open and close the
sidebar Apps group, and repeat with reduced motion on.

## Definition of done

- The pick-a-call page matches the booking card in both themes and at 390px.
- Picking a call carries the host header across with no duplicate
  `view-transition-name` on either page.
- Press, hover and open/close feedback is consistent, and nothing moves under
  reduced motion.
