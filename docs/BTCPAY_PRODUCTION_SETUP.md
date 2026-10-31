# BTCPay production setup

A step-by-step guide to turning crypto payments on in production with a
self-hosted BTCPay Server. The reference is the *Crypto payments* section of
`DEPLOYMENT.md`; this guide adds the order of steps and what was learned
testing locally (Plans 48 to 55).

Crypto stays off until phase 6, so the code can be merged and deployed first
without changing anything for anyone.

## Phase 0: decide first

- **Coins.**
  - Bitcoin is the simplest.
  - Litecoin is cheap to add at install time.
  - USDT needs the plugin, a TRON node and an address pool (phase 8).
    Add it later.
- **Wallet.** Use a hardware wallet whose **xpub** (extended public key)
  BTCPay can watch. Then no private keys sit on the server, and you sign
  refunds yourself.

## Phase 1: a separate server and DNS

- Run BTCPay on a **separate server**, not the app's (37.27.157.169),
  because it runs a full Bitcoin node. A pruned node keeps the disk small.
  Check BTCPay's current hardware guide before choosing a size, and add room
  if you include Litecoin.
- Point `A pay.bookcadence.app` at the new server before installing, so its
  HTTPS certificate can be issued.

## Phase 2: install BTCPay

On the new server:

```sh
git clone https://github.com/btcpayserver/btcpayserver-docker
cd btcpayserver-docker
export BTCPAY_HOST="pay.bookcadence.app"
export NBITCOIN_NETWORK="mainnet"
export BTCPAYGEN_CRYPTO1="btc"
# export BTCPAYGEN_CRYPTO2="ltc"     # only if you want Litecoin
export BTCPAYGEN_REVERSEPROXY="nginx"
export BTCPAYGEN_ADDITIONAL_FRAGMENTS="opt-save-storage-s"
. ./btcpay-setup.sh -i
```

The node then has to sync, which can take a day or more. The dashboard shows
progress. Checkout won't offer a coin until its node is synced.

## Phase 3: admin, store and wallet

1. Open `https://pay.bookcadence.app` right away and register. **The first
   account becomes the admin**, so do it before anyone else can. Turn on
   two-factor sign-in.
2. Create a store named `Cadence` with **USD** as the default currency.
3. Go to Store → Wallets → Bitcoin → **Connect an existing wallet**, and
   paste the hardware wallet's xpub. **Don't** generate a hot wallet. Do the
   same for Litecoin if you added it.
4. Checkout settings:
   - invoice expiry 15 minutes;
   - payment tolerance 0%;
   - speed policy *Medium* (one confirmation).

## Phase 4: webhook and API key

1. Go to Store → Settings → Webhooks → *Create*:
   - URL: `https://bookcadence.app/api/billing/crypto/webhook`
   - every event, with automatic redelivery on
   - **copy the secret now**, because BTCPay shows it only once.
2. Go to Account → API keys → *Generate*, limited to the Cadence store, with
   these permissions:
   - create invoices
   - view invoices
   - create pull payments (for refunds)

   Grant nothing more. **Copy the key.**
3. Note the Store ID from Store settings → General.

## Phase 5: confirm the code is live (still off)

After the merge and deploy:

```sh
curl -fsS https://bookcadence.app/api/billing/crypto/options
```

It should answer `"available":false`: the code is live and crypto is still
off.

## Phase 6: turn it on

Add these to `/root/cadence/.env` on the app server, then redeploy:

```sh
CRYPTO_PROVIDER=btcpay
BTCPAY_URL=https://pay.bookcadence.app
BTCPAY_STORE_ID=<store id>
BTCPAY_API_KEY=<key from phase 4>
BTCPAY_WEBHOOK_SECRET=<secret from phase 4>
BTCPAY_COINS="Bitcoin"          # or "Bitcoin or Litecoin"
```

`BTCPAY_COINS` is only the label on the pricing and Billing pages. Make sure
it matches what the store actually takes.

## Phase 7: test with a small real payment before announcing

1. `curl …/api/billing/crypto/options` should now answer
   `"available":true`.
2. With a test host account, pay for a Pro month ($10) from Billing.
3. In BTCPay, open the webhook's *Deliveries*: each one should show **200**.
4. `/admin/plans` should show the host with a *Crypto* badge, and the
   payment's coin should appear in the payments sheet.
5. Refund it from the row's menu, then open the emailed claim link and enter
   a wallet address.
6. **The refund isn't sent automatically.** With an xpub wallet, the claim
   waits in Store → Payouts until you approve it and sign with the hardware
   wallet. Cadence marks the payment refunded as soon as the admin clicks, so
   check Payouts regularly.

If the webhook misses an event, the backend asks BTCPay about unpaid invoices
every 5 minutes and credits a paid one anyway.

## Phase 8 (later, optional): USDT

Follow *USDT through BTCPay* in `DEPLOYMENT.md`. What was learned locally:

- **Give it many receiving addresses.** An unpaid invoice holds its address
  until paid, or for **24 hours after expiry** while BTCPay keeps watching it
  (the store's monitoring window). With all addresses held, checkout silently
  drops USDT, while the pricing page still says USDT.
- The plugin's default public TRON node is for testing only. Use a private
  node, or TronGrid with an API key.
- **USDT can't be refunded in USDT.** Cadence turns it into a Bitcoin claim
  of the same dollars, and the refund email says so.
- Don't open the plugin's TRON settings page on BTCPay 2.4.5. It crashes and
  BTCPay disables the plugin.
- Make one small real USDT payment and refund it before relying on it. That
  path has not been tried live.
- Then set `BTCPAY_COINS="Bitcoin or USDT"` (or with Litecoin) and redeploy.

## Ongoing

- Admins get an email when a host **overpays** by more than 1%. Refund the
  extra as a partial refund so the host keeps their time.
- **Keep the hardware wallet's seed safe.** All received coins are there;
  BTCPay only watches.
- Update BTCPay with `btcpay-update.sh` on its server from time to time.
- To turn crypto off, remove `CRYPTO_PROVIDER` and redeploy. Hosts keep the
  time they paid for; refunds need the provider back on.
