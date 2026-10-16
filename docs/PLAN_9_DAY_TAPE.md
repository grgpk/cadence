# Plan 9: the day tape

Status: done.

## Goal

Cadence looks like what it does. A day is a strip of hours and a booking is a
block on it; that strip is the one bold thing a visitor sees, and it moves.

## The gap this fixes

Plan 8 left a quiet monochrome system that read as a standard template, with
no motion and nothing that said "scheduling". The home page was also the
booking page of one host, which Plan 7 moved to `/book/{slug}`.

## Approach

This restyles the existing shadcn system through its tokens. It is not a
second design system, and it adds no animation library: motion is CSS
keyframes in `frontend/src/styles/globals.css`.

**Color**

| Token | Role |
| --- | --- |
| `--background` paper | page ground |
| `--foreground`, `--primary` ink | text, primary buttons, booked blocks |
| `--dawn` | morning times |
| `--noon` | afternoon times, open hours, days that have times |
| `--dusk` | evening times, the selected day, focus rings |
| `--signal` | the "now" playhead and the logo's clock hand, nothing else |

The three day colors carry information: a time's rail tells the visitor
which part of their own day it falls in.

**Type.** Archivo Variable replaces Funnel Display as `--font-display`. Its
width axis is the point: `font-wide` for headlines, `font-narrow` with
tabular figures for clock times. Hanken Grotesk stays for body text. New
dependency `@fontsource-variable/archivo`; `funnel-display` is removed.

**Motion.** All of it is off under `prefers-reduced-motion`.

1. The landing tape drifts under a fixed "now" line while open slots get
   booked one at a time. It pauses on hover and off screen. This is the only
   ambient motion in the app.
2. Picking a time fills the button from its rail, then the card slides to the
   details step, and back on "Change time".
3. A month change slides in from the side of travel.
4. On the confirmation page the booked block drops into its strip once.

Nothing else animates.

**Pointer cursor.** One base rule gives every clickable control a pointer;
the shadcn menu, select and command items use `cursor-pointer` to match.

## Flow schema

```
/  landing
┌──────────────────────────────────────────────┐
│ Cadence                     Log in  Sign up  │
├──────────────────────────────────────────────┤
│ Your week, open                              │
│ for booking.                                 │
│ Set the hours ...      [Create your account] │
│                                              │
│  09:00   10:00 │ 11:00   12:00   13:00    ←  │
│ ─┬───────┬─────│─┬───────┬───────┬──── drift │
│  ▓ Ana   ░ open│ ░ open          ▓ Mia       │
│               now                            │
├──────────────────────────────────────────────┤
│ 1 Set your hours  2 Share your link  3 Get booked
└──────────────────────────────────────────────┘

/book/{slug}  the card is the page
┌──────────┬──────────────┬────────────────┐
│ Host     │  October     │ Fri, Oct 9     │
│ Title    │ Mo Tu We ..  │ Morning        │
│ 30 min   │  •  •  ●     │ ▌09:00  ▌09:30 │
│ timezone │              │ Afternoon      │
│          │              │ ▌13:00         │
└──────────┴──────────────┴────────────────┘
```

## Implementation sketch

- `styles/globals.css`: tokens, `--animate-*` keys with their keyframes,
  `font-wide` and `font-narrow` utilities, the pointer rule.
- `domains/landing/`: `LandingPage`, `DayTape`.
- `domains/widget/`: `SlotStep` (times grouped by part of day, rail fill),
  `BookingCalendar` (month slide), `BookingCard` (step slide),
  `zoned-time.ts` (`dayPhase`).
- `routes/booking-confirmation.tsx`: `BookedStrip`.
- `routes/dashboard/availability.tsx`: `DayStrip`, each weekday's hours on a
  midnight-to-midnight bar.

## Out of scope

- An Open Graph image and a refreshed favicon.
- Motion in the dashboard and admin tables.

## Tests

- Checks from `AGENTS.md`.
- Browser walk-through at 390px and 1440px, light and dark: landing, booking
  page, pick a time, form errors, confirmation, unknown link, dashboard,
  availability, settings.
- With reduced motion on, the tape is still and steps change without sliding.

## Definition of done

- No hardcoded colors in `frontend/src` outside `components/ui` and the Meet
  icon.
- Every animation is behind `motion-safe:` or a `motion-reduce:` override.
- Buttons, links, switches, checkboxes, radios, tabs and menu items show a
  pointer; disabled controls do not.
