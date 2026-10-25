# Plan 53: Litecoin next to Bitcoin in the local BTCPay, refunds in the coin paid

Status: implemented.
- Backend: fmt, clippy and the BTCPay gateway tests pass.
- Checked live on the local BTCPay: a host paid a Pro month in LTC and was
  credited by the backend's 5-minute check. An admin refunded $3 from
  `/admin/plans`, and the claim page asked for an LTC address.
- Found while testing: before this plan, the same refund asked for a **BTC**
  address. BTCPay refunds in Bitcoin unless told otherwise, so a USDT payment
  would have been refunded in Bitcoin too.

## Goal

- Paying with a coin other than Bitcoin can be tried locally from start to
  finish: checkout, credit, short payment, refund.
- A refund goes back in the coin the buyer paid in.

## Gap

- `docker-compose.btcpay.yml` ran Bitcoin regtest only.
- `scripts/btcpay-setup.sh` only set up and paid `BTC-CHAIN`.
- The refund sent no `payoutMethodId`, so BTCPay refunded in Bitcoin.

## Data model and API

No changes. Invoices stay priced in USD, and BTCPay's checkout offers every
coin the store takes.

## Implementation

- **Compose:** a `litecoind` regtest service. NBXplorer and BTCPay run
  `btc,ltc`.
- **Setup script:**
  - Mines test coins and makes a hot wallet for each coin.
  - Writes `BTCPAY_COINS="Bitcoin or Litecoin"`. The quotes are needed,
    because dotenv refuses a bare value with spaces.
  - `pay` and `pay-invoice` take an optional `btc|ltc` as the last argument.
    The default is `btc`.
- **Gateway:** `refund` reads the invoice's payment methods first, then sends
  the one that has payments as `payoutMethodId`. If no method has payments,
  nothing is sent and BTCPay keeps its default.

## Flow

```text
docker compose -f docker-compose.btcpay.yml up -d
scripts/btcpay-setup.sh                       # restart the backend afterwards
Billing → Crypto → Pay      (BTCPay offers BTC and LTC)
scripts/btcpay-setup.sh pay-invoice <link> ltc
scripts/btcpay-setup.sh pay-invoice <link> short ltc
/admin/plans → Refund       (the claim page asks for an LTC address)
```

## Tests

- `a_refund_goes_back_in_the_coin_that_was_paid`: the paid method is found
  and sent as `payoutMethodId`.

## Definition of done

- A host can pay in LTC locally and gets credited.
- An admin refund of an LTC payment is claimed in LTC.
- The Bitcoin path works as before.
