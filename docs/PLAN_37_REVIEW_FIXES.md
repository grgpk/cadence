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
a row the 14-day Pro trial a new sign-up gets, from the deploy. It
queues two emails:
- `trial_started` at once: "Your page is on Pro, free for 14 days",
  saying nothing they use changes today;
- the usual trial-ending email 3 days before the end. The row holds only
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

## The findings first left for later (part 2)

All seven were then fixed as well.

### Weekly insights email

- One host failing is logged and the loop goes on, instead of costing every
  later host their week.
- Each host is queued under `pg_advisory_xact_lock` and checked again under
  it, so two workers in one minute queue one email. Test:
  `two_workers_in_one_minute_queue_one_weekly_email`.
- `weekly_due` skips timezones Postgres does not know, which used to fail
  the query for everyone.

### Webhook deliveries

The worker gives a delivery up, without sending it, when its endpoint is
paused or the host's plan no longer has webhooks. The panel shows why.
Before, retries went on for a day.

### Admin comp

An admin can comp hosts only; anyone else gets 400.

### Accent colour on dark pages

`forDarkPage` lightens a colour until it reaches 3:1 against the dark page,
and `.host-accent` in globals.css picks the light or dark version. Checked:
`#1e1b4b` shows as `#787693` in dark mode, with a visible focus ring.

### Webhooks read the subscription as Stripe has it now

`apply_event` fetches `GET /v1/subscriptions/{id}` before its transaction
and applies that. Events from the same second, or out of order, can no
longer leave an older state. Test:
`events_from_one_second_in_any_order_leave_stripes_latest_state`.

### One Checkout at a time

`subscriptions.stripe_checkout_session_id` (in `0017`, which exists only on
this branch) keeps the newest session. Checkout now:
1. makes the Stripe customer first and stores it, so every tab shares one;
2. asks Stripe whether it already bills that customer (409 if so, even
   before the webhook lands);
3. expires the previous session, then stores the new one.

Checked in Stripe test mode: the first of two tabs shows "this checkout
session has timed out"; the second paid; a further checkout got 409.

### Calendly import, all or nothing

`calendly::apply` runs in one transaction. The writes it uses have `_in`
versions taking the connection:
- `host_settings::update_in`;
- `availability::replace_weekly_in` and `save_override_in`;
- `event_types::list_in`, `create_in` and `update_in`.

Their single-query `db` functions take any `PgExecutor`, and the
multi-query ones take `Acquire`, so other callers did not change. Only the
plan is read from the pool. An import brings at most 366 dates. Test:
`a_calendly_import_that_fails_halfway_changes_nothing`.

## Still open

- **Weekly email after an email change:** it is matched by address, so a
  host who changes their email that week can get a second one.
- **Refunds:** a refund does not take a referral credit back.
