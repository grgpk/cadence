# Plan 37: fixes from the review of the pricing branch

Status: done.
- Rust: fmt, clippy and every test pass.
- Frontend: build, typecheck, lint, knip and tests pass.

## Goal

Before `feat/pricing` (Plans 27–36) is merged, three read-only reviews
covered billing and accounts, the other backend domains, and the
frontend. This plan fixes what they found that would hurt a host, an
invitee, or the business. What was left on purpose is listed at the end.

## Existing hosts start a Pro trial

`subscriptions` (0017) has no row for anyone who signed up before plans,
which reads as Free. On deploy those hosts would lose the 2h and 30 min
reminders and their questions, and get the logo back, without a word.

`0022_start_trials_for_existing_hosts.sql` gives every active host without
a row the 14-day Pro trial a new sign-up gets, from the deploy. It also
queues the trial-ending email 3 days before the end. The row holds only
the name; the worker fills in the billing link and what this host would
lose when it sends (`emails::with_trial_losses`).

This was checked on a local host without a row, inside a transaction
that was rolled back.

## Referral credits (Plan 36)

- Credits run after the webhook commits. Before, a Stripe failure rolled
  back the plan sync and blocked a paying host for up to three days of
  Stripe retries.
- Each credit needs the latest invoice to have `amount_paid > 0`.
- Credits are claimed in SQL before calling Stripe and released if Stripe
  refuses.
- `metadata[referral]` on the balance transaction is checked first, so a
  credit is made once even after Stripe's 24h idempotency window.
- Checked in Stripe test mode: both credits landed with their keys. A fresh
  event after the local mark was erased made no second credit.

## Subscriptions with an unknown price

A `customer.subscription.deleted` whose price no catalog entry or metadata
names was logged and dropped, so the host kept the plan for free.
`db::end_subscription` now cancels the stored plan when it is that
subscription.

## Attendance (Plan 33)

- A host was told "has not confirmed" for calls booked less than a day
  ahead, whose invitee never got the reminder that asks.
  `asked_to_confirm` now requires the call to be at least 24h away when it
  is booked or moved.
- Opening the reminder's "I'm coming" link confirmed on page load, so a
  mail scanner could answer for the invitee. The page now shows an
  "I'm coming" button, and confirming takes a click.

## Follow-ups (Plan 33)

- Once per person, not per lead row. A visitor who started three times got
  three "only email" messages.
- `mark_followed_up` applies the list's own filters: valid email, 30-day
  window, left over an hour ago, not booked since. A stale list or a direct
  POST can't email someone who has booked.
- The mark and the queued email share one transaction.
- The list shows one row per person.

## The Free limit email (Plan 30)

It was sent when the month's count was exactly 25. Two bookings at once
could step over it, and a cancellation could bring it back for a second
email. Now it is sent at 25 or more, unless one already went out this
month in the host's timezone, as the FAQ says.

## Frontend

- **Billing buttons:** they are turned on again when the browser restores
  the page from its back/forward cache after Stripe (`pageshow`).
- **409 messages:** checkout, portal and the yearly switch each have their
  own.
- **Waiting for Stripe:** after 30 seconds, "Waiting for Stripe…" becomes
  a note with a "Check again" button.
- **Yearly toggle:** it says "save up to 20%" when the plans save different
  amounts (Pro 20%, Business 18.75%), via `yearlyLabel`.
- **Pricing page:** its loader keeps the plans (`staleTime: Infinity`), so
  moving the slider does not fetch them again. Checked: 0 requests over
  six steps.
- **Insights card:** it shows an error with Retry instead of loading
  forever.
- **Ended trial:** no "Trial" badge (`trialEnded`).
- **Invite note:** "Yours arrives once you are on a paid plan."
- **Unfinished panel:** a refused follow-up refreshes the list and says why.

## Left on purpose

- **Two Checkout tabs:** they can create two subscriptions. That needs a
  customer made before Checkout; it is rare and visible in Stripe.
- **Same-second events:** Stripe events from the same second can apply in
  the wrong order. Fetching the subscription from Stripe on each event
  would fix it.
- **Weekly insights:** the email assumes one worker. Its loop stops at the
  first failing host.
- **Calendly import:** it is not atomic (from Plan 32's notes).
- **Admin comp:** an admin can comp any user, not only hosts.
- **Webhook retries:** queued retries continue after an endpoint is paused.
- **Dark-mode accent:** a dark accent has low contrast as a focus ring on
  a dark page.
