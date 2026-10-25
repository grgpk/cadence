# Plan 51: Crypto keeps the price promise, and goes quiet behind a card

Status: implemented.
- Backend: fmt, clippy, every test passes (crypto 24 HTTP, including the
  email worker with a recording mailer). Frontend: build, typecheck, lint,
  knip and tests pass.

## Goal

- The pricing page's "Will my price go up? Not while you stay subscribed"
  holds for crypto too: a host who renews before their period ends pays what
  they paid last time, even after a price rise.
- A host who has moved to a card subscription gets no more "renew with
  crypto" or "your crypto plan has ended" emails.

Stripe's pricing, checkout and flows stay exactly as they are.

## The gap this fixes

- A crypto renewal was charged the price of the day.
- The crypto renewal reminder and end notice were queued at payment time,
  and still went out after Stripe started billing the host.

## Data model

No change. The locked price is the `amount_cents` of the host's last paid
`period` payment for the same plan and interval.

## Behaviour

- **Price lock.** A crypto checkout for the plan of the running period (a
  renewal) costs the lower of today's price and that last price. A host whose
  period has ended pays today's price, which is what "stay subscribed" means
  for crypto. Upgrades still use today's prices for the difference.
- `CryptoSummary.renewal` lists the renewal price for each interval of the
  running plan, so Billing's crypto card shows what renewing really costs.
- **Behind a card.** The email worker drops a queued `crypto_renewal` or
  `crypto_ended` email when Stripe bills the host at send time.
- **Wording.** The renewal email and the crypto FAQ answer say that renewing
  before the end keeps the price.

## Tests

- **HTTP:**
  - a renewal pays the last price after a price rise
  - after a lapse the host pays today's price
  - the summary lists the renewal prices
  - a renewal reminder is dropped when Stripe bills at send time, and sent
    when it does not
- **Frontend:** the crypto card prefers the renewal price.

## Definition of done

The AGENTS.md checks pass, bindings are clean, and Plan 0 lists this plan.
