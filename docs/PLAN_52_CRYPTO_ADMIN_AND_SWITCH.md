# Plan 52: Crypto payments for admins, USDT through BTCPay, and one switch in Billing

Status: implemented.
- Backend: fmt, clippy, every test passes (crypto 26 HTTP). Frontend: build,
  typecheck, lint, knip and tests pass.
- Checked live on the local BTCPay: a $16 Business month refunded $5, then
  the remaining $11, which ended the period. BTCPay shows two USD pull
  payments of 5.00 and 11.00.
- Billing loads its data in the browser, so the switch is checked by unit
  tests and the build, not in a browser.

## Goal

- An admin sees every crypto payment a host has made and refunds any one of
  them, in full or in part.
- Admins hear about a payment that came in short, not only the host.
- BTCPay can take USDT through its plugin, and Cadence names the coins right.
- Billing pays in crypto the way the pricing page does: one Card | Crypto
  switch over the same plan cards, instead of a separate crypto card.

Stripe's pricing, checkout and flows stay exactly as they are. Card is
Billing's default.

## Data model

New `migrations/0026_create_crypto_refunds.sql`: `crypto_refunds` holds one
row per refund.
- Columns: `unid`, `payment_unid` (FK to `crypto_payments`), `amount_cents`,
  `claim_url` (nullable) and `created_at`.
- The refunds on a payment never add up to more than it paid.
- A payment is marked `refunded`, and its time taken back, only when its
  refunds cover all of it. A partial refund gives money back and leaves the
  time.

## API (crypto_billing, `AdminUser`)

- `GET /api/admin/billing/{user_unid}/crypto-payments` returns
  `AdminCryptoPayment[]`, newest first: plan, interval, kind, amount, refunded
  so far, status, paid at, period end.
- `POST /api/admin/billing/crypto-payments/{payment_unid}/refund` takes
  `{ amount_cents?: number }` and answers with the `CryptoRefund`.
  - Without an amount, it refunds whatever of the payment is left.
  - A payment that isn't paid answers 409. An amount of 0, or more than is
    left, answers 400.
- The Plan 49 route that refunded the latest payment is replaced by these two.
- BTCPay refunds the dollar amount (`refundVariant: Custom`, in USD) as a
  pull payment. NOWPayments still refunds from its own dashboard.

## Short payments

When an invoice expires part-paid, every admin (Root and Admin) gets an
`admin_crypto_underpaid` email. It names the host and the plan, and links to
`/admin/plans`, where the host's payments can be refunded.

## USDT on BTCPay

- `BTCPAY_COINS` (default `Bitcoin`) names what the store takes, such as
  `Bitcoin or USDT`. It feeds the pricing switch, the FAQ and Billing.
- `DEPLOYMENT.md` gets a section on installing the `Tether USDt` plugin:
  - Settings → Plugins, then restart.
  - Choose the chains (TRON, Ethereum or Polygon) and the receiving
    addresses.
  - For production, use a private TRON node or a TronGrid API key.
  - Refunds in USDT need the plugin's payout support. Otherwise, refund from
    BTCPay by hand.

## Billing switch

```
┌──────────────┬────────────────────────────────────────────────────────┐
│ sidebar      │ Plans      [▭ Card | ₿ Crypto]   [Monthly | Yearly]    │
│              │ ┌ Free ──┐ ┌ Pro ──────────────┐ ┌ Business ────────┐  │
│              │ │        │ │ $96 up front/yr   │ │ $156 up front/yr │  │
│              │ │        │ │ [Renew Pro · $96] │ │ [Move up · $8.80]│  │
│              │ └────────┘ └───────────────────┘ └──────────────────┘  │
│              │ ₿ Paid in Bitcoin, up front; nothing renews by itself.  │
└──────────────┴────────────────────────────────────────────────────────┘
```

- **When it shows:** while crypto is on and Stripe does not bill the host.
- **Which side it starts on:** crypto when the host has a crypto period or
  picked crypto on the pricing page, card otherwise.
- **Crypto buttons on each card:**
  - "Pay with crypto", with no period running
  - "Renew · $X" at the locked price, for the plan being paid
  - "Move up · $X" for a higher plan
  - "After {date}" (disabled) for a lower plan
- **Notes:** a confirming payment and the coins appear under the cards.
- **Card side:** unchanged, with Plan 50's notice while crypto time runs.
- **What goes:** the separate "Pay with crypto" card is removed.

## Tests

- **HTTP:**
  - payments are listed for admins only
  - a partial refund keeps the time, and the rest then refunds in full and
    takes the time back
  - an over-refund is refused
  - an older payment can be refunded
  - the admin alert on short payments
- **Unit:** the BTCPay refund body, the admin email.
- **Frontend:** the crypto plan button for each case.

## Definition of done

The AGENTS.md checks pass, bindings are clean, and Plan 0 lists this plan.
