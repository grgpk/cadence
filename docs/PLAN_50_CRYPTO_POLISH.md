# Plan 50: Crypto payments, fair and clear

Status: implemented.
- Backend: fmt, clippy, every test passes (crypto 22 HTTP). Frontend: build,
  typecheck, lint, knip and tests pass.
- Checked live on the local BTCPay. A host whose trial ends 8 Nov paid for a
  Pro month, and it runs to 8 Dec. The receipt reads "Pro until 8 December
  2026, Paid $10.00", with the reminder on 5 Dec and the end notice on 8 Dec.
- An upgrade during the trial costs the difference for the trial days too:
  Business replaces the free Pro then.

## Goal

- A host paying in crypto is never charged twice without knowing it.
- Paying during the trial does not cost trial days.
- The host hears about every crypto payment: when it arrives, when it is
  short, and when its period ends.
- Invoices can't be made at the provider in a loop.

Stripe's pricing, checkout and flows stay exactly as they are.

## The gap this fixes

- Plan 48 promised a Billing notice for a host with crypto time who
  subscribes by card. It was never built, and the crypto status disappeared
  as soon as Stripe billed.
- A crypto period started when the payment landed, even mid-trial, so the
  trial days were lost.
- No receipt was sent. An underpaid invoice told the host nothing, and
  nothing was sent when a period ended.
- Every crypto checkout makes a real invoice, and nothing limited it.

## Data model

No change.

## Behaviour

- **Double payment notice.** While a crypto period runs, Billing shows a
  notice over the plan cards:
  - "Pro is paid with crypto until {date}. Subscribing by card starts billing
    today, and the crypto time is not refunded."
  - The same line stays in the Current plan card after Stripe starts billing,
    so the crypto time stays visible.
  - Stripe's buttons and checkout are unchanged.
- **Trial first.** A crypto *period* payment made while the host is on a
  running trial, for the trial's plan or a lower one, starts when the trial
  ends. A higher plan still starts at once, because the trial does not give
  it.
- **Emails** (account emails, through `email_queue`):
  - `crypto_receipt` goes out when a payment is credited. It lists the plan,
    the amount and the date the plan runs to.
  - `crypto_underpaid` goes out when an invoice expires part-paid. Nothing was
    added to the plan, and the host is asked to reply so an admin can refund
    what arrived.
  - `crypto_ended` is queued for the moment the period ends. It moves or is
    dropped together with the renewal reminder.
- **Rate limit.** `CRYPTO_CHECKOUT_PER_HOST` allows 10 checkouts an hour per
  host. Past it, the checkout answers 429.

## Tests

- **HTTP:**
  - a payment during the trial starts at the trial's end
  - a Business payment during a Pro trial starts at once
  - a receipt with the plan and the end date
  - the underpaid email
  - the ended email follows the reminder
  - the 11th checkout in an hour is refused
- **Unit:** the new emails render with their links.
- **Frontend:** the notice text helper.

## Definition of done

The AGENTS.md checks pass, bindings are clean, and Plan 0 lists this plan.
