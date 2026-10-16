# Plan 12: motion and polish

Status: implemented. The browser walk-through below is still to do.

## Goal

Moving through Cadence feels continuous. A picked time visibly becomes the
booked block, pages change without a blank frame, and every action that takes
time says so the same way.

## The gap this fixes

- A route change showed nothing until the next page was ready.
- The booking card changed shape when it mounted, and the confirmation page
  had no visual link to the time that was just picked.
- A host whose first open time was next month got a calendar with nothing to
  pick and a "Pick a day" hint.
- Pending buttons were a mix of spinners and bare label changes. Connecting
  Google Calendar gave no feedback at all.
- Invitees on the confirmation page were offered "Log in" and "Sign up".
- The landing page ended without a second way to sign up.

## Approach

No new dependency. Motion is CSS keyframes in
`frontend/src/styles/globals.css` plus the browser View Transitions API, which
TanStack Router drives through `defaultViewTransition`. Browsers without it
get the same pages with no transition.

This replaces Plan 9's "Nothing else animates". Its other rules stay: every
effect is off under `prefers-reduced-motion`, no hardcoded colors, and
`--signal` belongs to the playhead and the logo.

**Three moments**

1. **A time becomes the block.** The rail on the picked-time chip and the block
   on the confirmation strip share the `vt-booked-block` utility
   (`view-transition-name: booked-block`), so the rail travels and widens into
   its place on the ruler. On a direct load the block still drops in.
2. **The tape spools in.** On the landing page the headline and call to action
   rise in, then the tape arrives fast from the right and slows into its
   drift. Once per load.
3. **Day to night.** The theme toggle opens the new theme as a circle from the
   button, using `--reveal-x`, `--reveal-y` and `--reveal-r` set at click time
   and scoped by `data-theme-switch` on `<html>`.
4. **Theme card.** Settings has a Theme card, like Cal.com's: System, Light or
   Dark, saved per device in `localStorage`. A device that never chose gets
   Dark. The card and the top bar toggle read one store, so they never
   disagree, and System follows the OS live.
5. **Cal.com neutrals.** Paper, ink, surfaces, borders and focus rings use
   Cal.com's grays: white and gray-50 under gray-900 ink in light; #101010,
   #1c1c1c and #2b2b2b under white ink in dark. The day colors and the
   signal stay; only the navy tint is gone.

```
details step                         confirmation
┌──────────┬───────────────┐         Your call is booked
│ Host     │ Your details  │
│ ▌Fri 9   │ [First][Last] │  ───►   09:00  10:00  11:00  12:00
│ ▌10:00   │ [Email]       │  morph  ─┬──────┬──────┬──────┬───
│ Change   │ [Confirm]     │               ▓▓▓▓  ◄ same element
└──────────┴───────────────┘
```

**Navigation.** Pages cross-fade in 150ms. A 2px dusk line runs across the top
while the router is pending, starting after 150ms so quick changes stay clean.

**Feedback on actions**

- Buttons press in slightly (`active:scale-[0.98]`).
- Segmented controls and calendar days ease their color changes.
- Times step in with a short stagger when a day is picked.
- The settings "Unsaved changes" bar rises in.
- Admin funnel bars grow from zero.
- Every pending button is a spinner plus its label: log in, create account,
  save changes, add hours, connect, reconnect and disconnect Google Calendar.

**Fixes**

- The booking card's loading state is the same three columns as the card.
- The calendar opens on the month of the first open time.
- `PublicLayout` takes `accountLinks`; the confirmation page turns them off.
- The bookings tab bar renders while bookings load.
- Not found, failed page, legal and "No booking to show" headings use the
  public `font-wide` treatment.
- The landing page closes with a second call to action.

## Implementation sketch

- `styles/globals.css`: `tape-spool`, `rise`, `bar-grow`, `nav-progress`,
  `theme-reveal`; the `vt-booked-block` utility; `::view-transition-*` rules,
  including the reduced-motion ones.
- `router.tsx`: `defaultViewTransition`.
- `routes/__root.tsx`: `NavigationProgress`.
- `hooks/theme.ts`, `domains/shared/theme-toggle.tsx`: the reveal; the
  preference store (`useSyncExternalStore`) and the dark default.
- `domains/dashboard/appearance-section.tsx`: the Theme card. Its previews use
  the real tokens through `.light` and `.dark` classes in `globals.css`.
- `domains/landing/`: spool wrapper, entrance, closing band.
- `domains/widget/`: `SlotStep` (first-month jump, stagger), `BookingCard`
  (skeleton), `DetailsStep` and `routes/booking-confirmation.tsx` (the morph).
- Pending states in `routes/login.tsx`, `routes/register.tsx`,
  `routes/dashboard/settings.tsx`, `routes/dashboard/availability.tsx`,
  `domains/dashboard/GoogleCalendarConnection.tsx`.

## Out of scope

- Add to calendar on the confirmation page.
- Stacked mobile layouts for the admin tables.
- Per-field validation on the login and register forms.
- A JavaScript animation library.

## Tests

- Checks from `AGENTS.md`.
- Browser walk-through at 390px and 1440px, light and dark:
  - Landing: the spool-in plays once, then the tape drifts and pauses on hover.
  - Book a call: the rail becomes the block. Reload the confirmation page and
    the block drops in instead.
  - Toggle the theme from the landing header and the dashboard top bar, by
    mouse and by keyboard.
  - Clear storage: the app loads dark with no flash. Pick each option in the
    Theme card; the top bar icon follows, and a second tab follows too.
  - Move between dashboard pages; throttle the network to see the top line.
  - A host whose first open time is next month opens on that month.
- With reduced motion on: no spool, morph, reveal or cross-fade, and popovers
  and sheets still close.
- In a browser without View Transitions everything works without them.

## Definition of done

- Every animation is behind `motion-safe:`, the reduced-motion block, or a
  JavaScript check of the same preference.
- No page shows two elements with `vt-booked-block`.
- No pending button changes its label without a spinner.
