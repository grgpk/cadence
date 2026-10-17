# Plan 34: keep the promise, open the Calendly door, make the trial end personal

Status: done.
- Rust: fmt, clippy and every test pass.
- Frontend: build, typecheck, lint, knip and 212 tests pass.

Checked in the browser:
- an accent colour picked in Settings themes the booking page, in light and dark;
- the "Coming from Calendly?" card shows on a new host's Overview, and "Not now" hides it;
- a trial host 4 days from the end sees their own losses in the banner and on Billing.

## Goal

Three weak spots between what Cadence promises and how a new host comes to
pay:
- the pricing page promised an accent colour ("Soon") that did not exist;
- the Calendly import was hidden on the Event types page;
- the end of the trial said "you move to Free" without saying what that
  means for this host.

## Accent colour (Pro)

- `widget_settings.accent_color` (from 0007; no migration) is now part of
  `HostSettings`.
  - It must be `#rrggbb` and is saved in lowercase.
  - Changing it needs `custom_accent`, otherwise 403 `PlanRequired`. A saved
    colour stays saved on Free.
- `BookingPage.accent_color` and `HostPage.accent_color` carry the colour
  only when the plan has it and it is not the default (`DEFAULT_ACCENT`).
- `BookingShell` sets `--primary`, `--dusk` and `--ring` to the colour, and
  their foreground to white or ink by WCAG contrast (`lib/color.ts`
  `readableOn`).
- **Settings, Booking page:** six presets, a colour input, a hex box and a
  live button preview. Free shows the Pro notice.
- **Pricing:** the "Soon" badge is gone, and Pro lists "Your own accent
  colour".

## Calendly import for new hosts

- `SwitchingCard` on Overview: "Coming from Calendly?", shown while the host
  has no bookings. "Not now" is kept in `localStorage`.
- The pricing FAQ gains "Can I bring my setup from another scheduler?".

## Trial end

- **Single source:** `billing::service::trial_losses(&TrialUsage)`.
  - `TrialUsage` holds active kinds of call, questions asked, a saved accent
    colour and this month's bookings, all read by one query in
    `billing::db::trial_usage`.
  - Each line compares that usage with `Plan::Free.entitlements()`.
- **`BillingSummary.trial_losses`** is filled only while a trial runs.
- **Banner:** in the last 5 days it lists up to 3 lines. It is hidden on
  Billing, which lists them all under Current plan.
- **`trial_ending` email:** it is queued at sign-up, so the worker
  (`filled_in`) adds `losing` from `trial_losses_for(email)` when it sends
  it. Rows without `losing` keep the general text.

## Tests

- **HTTP:**
  - `an_accent_colour_needs_pro_and_stays_saved_without_it`;
  - `a_trial_host_sees_what_free_would_take_away`.
- **Unit:**
  - `the_trial_says_what_this_host_would_lose`;
  - `the_trial_email_lists_what_this_host_loses_when_it_knows`;
  - `readableOn` and `isHexColor`.
