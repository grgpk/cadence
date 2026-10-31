# Plan 55: the coin paid, overpayments, and a local check of all three coins

Status: implemented.

## Goal

- Admins and hosts can see which coin paid for a plan: BTC, LTC or USDT.
- A refund email says which coin the host claims it in. For a USDT payment
  that is Bitcoin.
- Admins hear when a host paid more than the invoice, so the extra goes back.
- The local BTCPay offers all three coins after a break, not just USDT.

## Gap

Found by running BTC, LTC and USDT checkouts against the local BTCPay:

- **A settled event after a full refund failed.** `mark_paid` matched
  `refunded` rows, so the update broke the `crypto_payments_refunded` CHECK.
  The webhook answered 500, and BTCPay kept retrying (a redelivery from its
  dashboard showed this). Fixed: a refunded payment stays refunded.
- **BTC and LTC were missing from checkout.** Regtest chains left for a day
  count as still syncing (initial block download), and BTCPay offers only
  the coins that are available. The setup script mined only into an empty
  wallet. Its wait counted any two available coins, so USDT plus LTC passed
  while BTC was down.
- **USDT was missing from the second checkout.** With one Nile address,
  the first invoice holds it until paid. If it expires unpaid, BTCPay keeps
  watching it for the store's monitoring window (24 hours by default), and
  the plugin keeps the address held for that time. `BTCPAY_COINS` still
  names USDT. Seen live: the invoice expired at 14:25:27, and checkouts after
  that still offered no USDT.
- **A short USDT payment likely could not be refunded** (not tried live). The
  coinless retry from Plan 54 asks for a `RateThen` refund, which reads what
  was paid in Bitcoin: nothing. A last retry now refunds the dollars that
  arrived.
- **Nothing recorded the coin paid**, and an overpayment went unnoticed.

## Data model

`crypto_payments.coin TEXT`, folded into `0025_create_crypto_payments.sql`
(databases are reset, not altered). It is set when the payment is paid, and
stays NULL when the provider doesn't say (NOWPayments).

## API

`AdminCryptoPayment.coin: string | null`.

## Flow

```text
InvoiceSettled ─▶ apply_event
                  ├─ gateway.settlement(invoice)   coin + dollars arrived,
                  │                                 asked before the transaction;
                  │                                 a failure still credits
                  └─ paid: mark_paid(coin), receipt "Paid $10.00 in LTC",
                     arrived > invoiced + 1%  ─▶  admin_crypto_overpaid email
refund ─▶ gateway.refund ─▶ Refund { claim_url, coin }
          USDT: 422 ─▶ coinless ─▶ 422 on RateThen ─▶ Custom of the dollars arrived
          email: "You claim it in BTC, to a wallet address of yours."
```

## Implementation

- **Gateway:**
  - `Settlement { coin, arrived_cents }` and `Refund { claim_url, coin }`.
  - In BTCPay, `ticker` turns `LTC-CHAIN` into `LTC`, and a fallback refund
    names `BTC`.
  - The fake gateway gains `set_fake_coin`.
- **Emails:** `queue_admin_crypto(AdminCryptoAlert::{Underpaid, Overpaid}, …)`
  replaces `queue_admin_crypto_underpaid`, and there is a new
  `admin_crypto_overpaid` template. The extra is refunded as a part refund,
  so the host keeps the time.
- **Admin sheet:** a *Coin* row when known.
- **`scripts/btcpay-setup.sh`:**
  - Every run mines one block per coin.
  - The wait names BTC and LTC, and the script stops if they never come up.
  - `wake` only mines, so the webhook secret and key stay as they are.
- **Docs:** `DEPLOYMENT.md` covers the USDT address pool and `wake`.

## Tests

- `a_settled_event_after_a_full_refund_changes_nothing`.
- `a_payment_keeps_its_coin_and_admins_hear_when_more_arrived`: the coin is
  in the receipt, the admin list and the refund email. 10% over alerts the
  admins; 0.5% over does not.
- `a_short_payment_in_a_coin_without_payouts_is_refunded_as_its_dollars`.

## Definition of done

- The local check passes: BTC and LTC pay, short-pay, and refund in their own
  coin, and USDT is offered.
- A redelivered settle on a refunded payment answers 200 and changes nothing.
- A real USDT transfer was not tried. It needs faucet USDT in TronLink, and
  the gateway tests cover the refund paths.
