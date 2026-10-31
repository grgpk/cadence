# Plan 56: Public pages redesign

Status: done.

## Goal

A visitor understands Cadence within seconds of opening the home page: the
hero shows a booking happening from both sides. Pricing, the public header and
footer, and the legal pages share the same quiet look.

## Gap

The hero was an abstract tape of hours drifting past "now". It was a nice
signature, but it did not show what a guest or a host actually does.

## Approach

Same tokens and type as Plan 8. No new dependencies, no backend change.

- **Hero (`landing/booking-demo.tsx`).** A guest picks a day and a time on
  Ana's link (Lisbon time), confirms, and the call flies into Ana's Tuesday
  (Berlin time), lands in dusk next to the "now" line, and gets a Meet link.
  A step timer drives it. It pauses off screen and under the pointer, and
  with reduced motion it shows the last frame still. It is `aria-hidden`.
- **Landing sections.** How it works (kept), after someone books (a timeline
  of features that exist: emails, calendar + Meet, reminder, reschedule
  link), works with, promises, and an ink closing band.
- **Pricing.** Logic untouched. The "fits your week" bar grows in when the
  recommendation moves, the price rolls when the period changes, and the
  closing band matches the landing page.
- **Chrome.** The header is transparent until the page scrolls and gains a
  "How it works" link. The footer has product and legal columns.
- **Legal.** A 68ch reading column with a sticky contents list built from
  section anchors.

```
┌─ guest: cadence/book/ana ─────┐ ┌─ Your Tuesday ── Berlin ┐
│ Nov 2026   [10:00] ↖ Confirm  │ │ 10 open  ── now ──      │
│ ● 10                          │→│ 11 Leo (Meet)           │
└───────────────────────────────┘ └─────────────────────────┘
```

## Tests

Visual change only: lint, typecheck, knip, vitest, build. Checked with
screenshots at 1440 and 390, in light and dark.
