# Plan 48: Opt-in crypto plan payments, BTCPay Server first

Status: implemented.
- Backend: fmt, clippy, every test passes (crypto 11 HTTP, billing 24 HTTP unchanged).
- Frontend: build, typecheck, lint, knip and tests pass.
- Checked end to end against a local BTCPay Server on regtest: checkout, a
  real payment, signed events, the plan and the renewal email.

## Goal

- A host can choose to pay for Pro or Business in crypto from Billing.
- Stripe stays exactly as it is: prices, checkout, webhooks, the
  `subscriptions` table and the cancel, pause, portal and referral flows.
- The crypto processor can be swapped by changing config and adding one
  adapter file.

## The gap this fixes

- Card through Stripe was the only way to pay.

## How a crypto plan works

- A wallet can't be charged again automatically, so a crypto plan is a
  **prepaid period**. The host pays the plan's normal price (`plans.rs`) for
  1 or 12 months.
- Paying again adds to the period. A crypto renewal email goes out 3 days
  before it ends. If the host doesn't pay, the crypto plan simply ends.
- Providers are used for one thing only: "make an invoice for $X, and tell me
  how it went". Periods and reminders stay in Cadence, so no provider's own
  subscription feature is relied on.

## Data model

- `migrations/0025_create_crypto_payments.sql`: one `crypto_payments` row per
  invoice. The row's `unid` is the order id the provider echoes back.
  - Statuses are `pending`, `processing`, `paid`, `underpaid`, `expired` and
    `failed`, and they only move forward. `paid` is reached once, and a late
    payment still counts.
  - `provider` has no CHECK: the Rust enum owns its values, so adding a
    provider needs no schema change.
- The active crypto plan is the latest `paid` row with `period_end > now`.
  It's worked out when read, so no sweep is needed.

## API

- `GET /api/billing/crypto` returns `CryptoSummary { available, active, confirming }`.
- `POST /api/billing/crypto/checkout` takes `{plan, interval}` and returns `RedirectUrl` to the provider's checkout.
  - 409 while Stripe bills the host, or for a different plan while a crypto period runs.
  - 503 without a provider.
- `POST /api/billing/crypto/webhook`: the provider's signed events.
- `/api/admin/billing` rows gain `crypto_until`, and their `plan` counts the
  crypto plan. The admin's Plans page shows a Crypto badge and "Crypto paid
  until …".
- `billing::entitlements_for` and `/api/billing/me` give the host the
  **higher** of the Stripe plan and the crypto plan (`Plan` derives `Ord`).
  No Stripe path changed. Trial losses are skipped for a host paying in crypto.

## The provider seam

`backend/src/domain/crypto_billing/gateway/`:
- `mod.rs` holds `CryptoPayments`, the enum `Gateway { BtcPay, Fake }`
  (matched like `billing/stripe.rs`) and the provider-neutral `InvoiceRequest`,
  `Invoice`, `PaymentEvent` and `PaymentStatus`.
- `btcpay.rs` is the Greenfield API.
  - It creates an invoice in USD with `metadata.orderId`.
  - It checks `BTCPay-Sig: sha256=<HMAC of the body>`.
  - Events map as: `InvoiceSettled` → paid, `InvoiceProcessing` →
    processing, `InvoiceExpired` → expired (underpaid if `partiallyPaid`),
    `InvoiceInvalid` → failed.
- **Adding a provider** means one file, one `Gateway` variant, one arm in
  `from_env`, and a contract test that turns the provider's recorded events
  into the same statuses.

## Reconciliation

- `crypto_billing/worker.rs` runs every 5 minutes.
  - It asks the provider (`CryptoPayments::invoice_status`) about this
    provider's payments from the last 3 days that are pending, processing,
    expired or underpaid and have been quiet for 2 minutes.
  - When the provider says something different, the worker applies it through
    `service::apply_event`, as if the event had arrived.
- So an event that was lost, refused or blocked never leaves a paid invoice
  uncredited. A payment that lands after expiry still counts, and a paid one is
  never asked about again.
- Checked live: an invoice paid while BTCPay could not reach the webhook was
  credited on the worker's first pass.

## Frontend

- `frontend/src/domains/billing/crypto-card.tsx`: a "Pay with crypto" card
  under the current plan. You pick a plan and 1 month or 1 year, and the
  price is shown.
  - The card is hidden while Stripe bills the host or when crypto is off.
  - A running period offers renewal of the same plan.
  - While a payment is confirming, the card says so and polls every 15 seconds.
- When crypto pays for the plan, the Current plan card shows the crypto
  period instead of the trial or Stripe status.

```
┌──────────────┬──────────────────────────────────────────────┐
│ sidebar      │ Billing                                      │
│              │ ┌ Current plan ────────────────────────────┐ │
│              │ │ Business · paid with crypto until …      │ │
│              │ └──────────────────────────────────────────┘ │
│              │ ┌ Pay with crypto ─────────────────────────┐ │
│              │ │ [Pro|Business] [1 month|1 year]  $16     │ │
│              │ │                    [₿ Renew with crypto] │ │
│              │ └──────────────────────────────────────────┘ │
│              │ Plans (Stripe, unchanged)                    │
└──────────────┴──────────────────────────────────────────────┘
```

## Local BTCPay

- `docker compose -f docker-compose.btcpay.yml up -d` runs bitcoind on
  regtest, NBXplorer, BTCPay Server 2.4.5 and its Postgres.
  - `BTCPAY_SSRFEXCEPTIONS` lets webhooks reach the backend at
    `host.docker.internal:3001`. BTCPay refuses private addresses otherwise.
- `scripts/btcpay-setup.sh` sets everything up and is safe to run again:
  - it creates the admin, the store and a hot wallet,
  - it makes a fresh webhook and a store-scoped API key,
  - it mines test coins and writes `CRYPTO_PROVIDER` and `BTCPAY_*` into `.env`.
- `scripts/btcpay-setup.sh pay <address> <btc>` pays an invoice and mines 6 blocks.

## Tests

- `backend/tests/crypto_billing_http.rs`, against the fake provider:
  - without a provider, crypto is off
  - a paid invoice gives the plan for its interval and queues the reminder
  - a repeated event counts once, and renewing adds to the period
  - unpaid invoices give nothing, and a seen payment shows as confirming
  - a wrong secret or another invoice changes nothing
  - checkout is refused while Stripe bills or for another plan
  - an ended period leaves the Stripe plan or Free
  - a paid invoice whose events were lost is credited on the next pass, once
  - a payment that just changed is left to its events
  - a payment landing after expiry still counts
  - the admin sees a plan paid in crypto and until when
- `gateway/btcpay.rs`: the event contract table, the invoice status read back,
  bad signatures, and invoices Cadence didn't make.
- `billing.test.ts`: `cryptoStatusLine`. `emails/service.rs`: the renewal email.

## Known limits and next steps

- Switching plan mid-period and refunds are out of scope. A refund is made in
  BTCPay by hand, and the period then runs out on its own.
- Production needs a BTCPay Server (self-hosted, or a third-party host) and a
  wallet whose keys stay off the server, such as an xpub or a hardware wallet.
