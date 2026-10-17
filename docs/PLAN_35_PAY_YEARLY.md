# Plan 35: pay yearly, one confirm screen away

Status: done.
- Rust: fmt, clippy and every test pass.
- Frontend: build, typecheck, lint, knip and tests pass.

Checked in the browser: a local host on monthly Pro sees "Pay yearly and
save $24" on Billing, at desktop and phone width.

## Goal

A host who pays monthly pays 20% more than they need to, and the only way
to change it was finding the interval switch inside the billing portal.
Billing now says what yearly saves and opens Stripe's confirm screen with
the yearly price already picked.

## Gap

- Billing showed "Yearly, save 20%" on the plan cards, but a paying host's
  card only said "Your plan".
- The portal can change the price, but the host has to know to look.

## Data model

No migration. `db::stripe_account` also reads `plan` and
`billing_interval`.

## API

`POST /api/billing/yearly` → `RedirectUrl`.
- 409 unless the host has a Stripe customer and subscription, is `active`
  and pays monthly.
- 503 without Stripe keys.
- The URL is a portal session with
  `flow_data[type]=subscription_update_confirm`: the subscription's one item
  (read with `GET /v1/subscriptions/{id}`) moved to the plan's yearly price.
  After confirming, Stripe redirects to Billing; the usual
  `customer.subscription.updated` event sets the interval.

## Flow

```
Billing (monthly Pro)
┌ Current plan ─────────────────────────────────────┐
│ Pro  Active          Bookings this month        0 │
└───────────────────────────────────────────────────┘
┌ Pay yearly and save $24 ──────────────────────────┐
│ Pro yearly is $96 a year, $24 less than 12 months │
│ at today's price.                                 │
│ Stripe shows the exact amount …  [Switch to yearly]│
└───────────────────────────────────────────────────┘
```

- `billing.ts` `yearlySwitch(summary, offers)` decides: active, monthly, not
  cancelling, and yearly saves something.
- The saving is worked out from today's prices. A host on a locked older
  price sees the real amount on Stripe's screen before agreeing.

## Tests

- HTTP: `a_monthly_payer_switches_to_yearly_on_stripes_confirm_screen`
  (trial 409, monthly gets the yearly price URL, yearly 409).
- Vitest: `yearlySwitch` offers monthly payers and stays quiet for yearly,
  trials and cancelling plans.

## Setup note

Stripe's portal configuration must allow subscription updates with the
yearly prices listed, as `scripts/stripe-setup.sh` already asks.

## Definition of done

A monthly payer goes from Billing to Stripe's confirm screen in one click,
and nobody else sees the offer.
