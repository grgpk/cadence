# Plan 49: Crypto upgrades, refunds, NOWPayments, production BTCPay, pricing page

Status: in progress.
- Milestones 1 to 3 are done and checked live against the local BTCPay on
  regtest. A paid Pro month moved to Business for $6.20 with the same end;
  refunding the upgrade went back to Pro, and refunding the month left
  nothing. Both refunds gave real BTCPay claim links.
- Found while checking: BTCPay 2.x takes the admin password over its API only
  in the account's first 5 minutes. `scripts/btcpay-setup.sh` now saves an
  API key (`BTCPAY_DEV_ADMIN_KEY`) on the first run and uses it afterwards.
  It also waits for BTCPay to be ready and mines until the test coins can be
  spent.
- Known edge: a full refund during a trial does not bring back the
  trial-ending email that the crypto payment took away.

## Goal

- A host on a crypto Pro plan can move to Business without waiting for the
  period to end, and pays only the difference.
- An admin can refund a host's crypto payment from the admin Plans page. The
  time it bought is taken back.
- NOWPayments works as a second provider (Bitcoin, USDT, USDC and more), which
  shows that providers really do swap.
- There is a written way to run BTCPay Server in production without private
  keys on the server.
- The public pricing page says that paying with crypto is possible.

## The gap this fixes

Plan 48 left these out:
- Switching plan mid-period was refused.
- Refunds were done by hand.
- BTCPay was the only provider, and was set up locally only.
- Visitors could not learn about crypto before signing up.

## Data model

Folded into `migrations/0025_create_crypto_payments.sql`, which isn't on main
yet, following the no-ALTER rule:
- `kind TEXT NOT NULL DEFAULT 'period'` is `period` (buys an interval) or
  `upgrade` (moves the time already paid to a higher plan and adds no time).
- `status` gains `refunded`, and `refunded_at TIMESTAMPTZ` is added.
- The active period is the paid row with the latest `period_end`. When two
  rows end at the same moment, the higher plan wins, which is how an upgrade
  row takes effect.

## Upgrade mid-period

- Checkout for a **higher** plan while a crypto period runs is an upgrade:
  - **Price:** the difference between the two plans' prices for the running
    interval, for the time left, rounded up to the cent, $1 at least.
  - **Period:** the end doesn't move. When paid, an `upgrade` row ending at the
    current end gives the higher plan from then on.
- A **lower** plan is still refused mid-period. It runs out, then the host picks again.
- Renewing then renews the higher plan.
- `CryptoSummary.upgrade` is `{ plan, amount_cents }` when an upgrade is
  possible, so Billing shows the price before leaving.

## Refunds

- `POST /api/admin/billing/{user_unid}/crypto-refund` (`AdminUser`, owned by `crypto_billing`):
  - It refunds the host's most recently paid crypto payment in full.
  - It marks the row `refunded` and takes its time back:
    - For a `period` row, every other paid row of the host ending at or after
      it moves back by its interval.
    - For an `upgrade` row, nothing moves; the plan goes back to the lower one.
  - The renewal reminder moves to the new end, or is dropped if no period is left.
  - It answers `{ claim_url }`.
- How the money goes back depends on the provider:
  - **BTCPay** makes a refund link (a pull payment, `refundVariant: Fiat`, so
    the host gets the dollar amount). The host gets a `crypto_refund` email
    with that link to claim it to their wallet.
  - **NOWPayments** has no refund API for invoices. The payment is marked
    refunded in Cadence, and the admin is told to send it from the NOWPayments
    dashboard (`claim_url: null`).
- The admin Plans page gets a "Refund last crypto payment" item, behind a
  confirm dialog.

## NOWPayments

- `gateway/nowpayments.rs`:
  - **Invoice:** `POST /v1/invoice` with `price_amount`, `price_currency: usd`,
    `order_id`, `ipn_callback_url`, `success_url` and `cancel_url`. We keep its
    `id` and `invoice_url`.
  - **IPN signature:** `x-nowpayments-sig` is an HMAC-SHA512, with the IPN
    secret, of the body with its keys sorted. Their examples disagree on
    nested keys, so both forms are accepted.
  - **Status map:**
    - `waiting` → pending
    - `confirming`, `confirmed` and `sending` → processing
    - `partially_paid` → underpaid
    - `finished` → paid
    - `failed` and `refunded` → failed
    - `expired` → expired
  - **Status lookup:** NOWPayments has no simple invoice lookup with an API
    key, so reconcile can't recover a lost event for it (it answers pending).
    This is noted as a limit.
- Settings: `CRYPTO_PROVIDER=nowpayments`, `NOWPAYMENTS_API_KEY`,
  `NOWPAYMENTS_IPN_SECRET`, `NOWPAYMENTS_IPN_URL`, and optionally
  `NOWPAYMENTS_API_URL` for the sandbox.
- It is checked with recorded bodies only. A live sandbox check needs a
  NOWPayments account.

## Pricing page

- `GET /api/billing/crypto/options` (public) returns `{ available, coins }`.
  `coins` comes from the provider: "Bitcoin" for BTCPay, "Bitcoin, USDT, USDC
  and more" for NOWPayments.
- When it's available, the pricing page shows a line under the plans and a FAQ
  answer. Billing's crypto card names the coins the same way.

```
┌──────────────────────────────────────────────────────────┐
│ Plans                              [Monthly|Yearly]      │
│ ┌ Free ┐ ┌ Pro ┐ ┌ Business ┐                            │
│ ₿ Prefer crypto? Pay a month or a year up front in       │
│   Bitcoin from Billing once you have signed up.          │
└──────────────────────────────────────────────────────────┘
```

## Production BTCPay

- `DEPLOYMENT.md` gets a "Crypto payments" section:
  - **Where to run it:** BTCPay on its own small server through
    `btcpayserver-docker`, with a pruned Bitcoin node.
  - **Wallet:** a watch-only one, made from a hardware wallet's xpub, so no
    private keys sit on the server.
  - **Store and webhook:** the store's speed policy, a store-scoped API key
    (create and view invoices, create pull payments for refunds), and a
    webhook to `https://bookcadence.app/api/billing/crypto/webhook`.
  - **Settings:** the `.env` lines, and how to check it works with a small
    real payment.
- No app or compose change: BTCPay stays a separate service.

## Tests

- **Upgrade:**
  - the price for the time left
  - paying keeps the end and gives the higher plan
  - a lower plan is still refused
  - renewing after an upgrade renews the higher plan
- **Refund:**
  - a period row moves later rows back
  - refunding an upgrade row returns the lower plan
  - the reminder moves
  - non-admins are refused
  - the fake's claim link is emailed
- **NOWPayments:** the contract table of recorded IPNs, both signature forms, a bad signature.
- **Options endpoint and frontend:** the coins label, the upgrade quote, the pricing line.

## Milestones

1. Plan, the schema fold, the options endpoint, coins and the pricing page.
2. Upgrade mid-period.
3. Refunds.
4. The NOWPayments adapter.
5. The production BTCPay guide.

## Definition of done

The AGENTS.md checks pass, bindings are clean, the BTCPay flows (upgrade and
refund) are checked against the local regtest server, and Plan 0 lists this plan.
