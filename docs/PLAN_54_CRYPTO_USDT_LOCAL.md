# Plan 54: USDT on TRON Nile in the local BTCPay

Status: implemented.
- Backend: fmt, clippy and every test pass.
- Checked live on the local BTCPay:
  - `scripts/btcpay-setup.sh usdt <address>` installed the plugin. This was
    tried after removing it with BTCPay's own `delete` command.
  - Running it again changed nothing.
  - A Cadence checkout then offered *USD₮ on TRON Testnet* (10 USDT for a
    $10 month) next to BTC and LTC.
  - An LTC refund was still claimed in LTC.
- Not tried live: a real USDT payment. That needs faucet USDT in your own
  TronLink. The refund fallback is covered by a gateway test against a local
  server answering as BTCPay does.

## Goal

- USDT can be tried locally, without editing BTCPay by hand.
- A refund of a USDT payment works.

## Gap

- Nothing installed the Tether USDt plugin, so the local BTCPay had no USDT.
- The plugin has no payouts. BTCPay answers `422` to a refund that names
  `USDT-TRON`. Since Plan 53 names the coin paid, such a refund failed.

## Facts

These come from the plugin's source (v0.7.9.0, needs BTCPay 2.3.7 or newer):
- On regtest the plugin uses TRON Nile: `https://nile.trongrid.io/jsonrpc`,
  test USDT contract `TXLAQ63Xg1NAzckPwKHvzw7CSEmLMEqcdj`, `USDt_USD = 1`.
- The payment method is `USDT-TRON`. The store sets it through the generic
  `PUT /api/v1/stores/{store}/payment-methods/USDT-TRON` with
  `{"enabled":true,"config":{"addresses":[...]}}`.
- Each open invoice holds one address. With all addresses held, checkout
  leaves USDT out.
- The plugin only watches a chain after a restart that follows the store
  setup.
- BTCPay installs a plugin from disk: `<id>.btcpay` plus an `install:<id>`
  line in `commands` in its plugin folder, read on the next start.

## Implementation

- **Compose:** `BTCPAY_PLUGINDIR=/datadir/Plugins`, on the data volume.
- **`scripts/btcpay-setup.sh usdt <nile address>...`:**
  - Installs the pinned plugin if it is missing.
  - Sets the addresses, restarts BTCPay, and waits for `USDT-TRON` to be
    available.
  - Writes `BTCPAY_COINS`. Plain setup keeps the label right too.
- **`pay-invoice <link> usdt`:** prints the address and amount to send from
  TronLink. It also takes the link as BTCPay shows it after a coin is
  picked, `.../i/<id>/USDT-TRON`.
- **`pay ... usdt`:** refused, because the script holds no TRON keys.
- **Gateway:** if a refund naming the paid coin gets `422`, it is tried once
  more without the coin, so the buyer claims the dollars in Bitcoin. Other
  errors are not tried again.

## Tests

- `a_coin_without_payouts_is_refunded_in_bitcoin`: a local server refuses
  the refund naming `USDT-TRON` and makes the one without. The test fails
  when the retry is removed.

## Definition of done

- A Cadence checkout offers USDT on TRON Nile after one script command.
- A USDT payment can be refunded, as a Bitcoin claim.
- BTC and LTC work as before.
