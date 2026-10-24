# Deployment

Cadence runs at `https://bookcadence.app` on `37.27.157.169`.

## One-time setup

GitHub Actions configuration:

| Type | Name |
| --- | --- |
| Secret | `SSH_PRIVATE_KEY` |
| Secret | `PROD_DOCKER_USERNAME` |
| Secret | `PROD_DOCKER_PASSWORD` |
| Variable | `SERVER_IP=37.27.157.169` |
| Variable | `PROD_DOCKER_REPOSITORY=cadence` |
| Variable | `LETSENCRYPT_EMAIL=contact@rustify.rs` |

Current Cadence GitHub setup already has `SSH_PRIVATE_KEY`, `SERVER_IP`, and
`LETSENCRYPT_EMAIL`. Add the two Docker secrets and repository variable before
first image deploy.

Server prerequisites, needed on any new server before the first deploy:

- The public half of `SSH_PRIVATE_KEY` is in `/root/.ssh/authorized_keys`
  (`ssh-keygen -y -f <private-key>` prints it). Without it the workflow fails
  with `Permission denied (publickey,password)`.
- Docker with the compose plugin, Nginx, and certbot are installed.
- Port 80 is reachable on `bookcadence.app` so certbot can issue the
  certificate.

Application runtime secrets are separate and stay on the server:

| Group | Variables in `/root/cadence/.env` |
| --- | --- |
| Database | `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DATABASE`, `DATABASE_URL` |
| Auth | `JWT_SECRET`, `APP_ENV`, `ENVIRONMENT`, `CORS_ORIGINS` |
| Google Calendar | `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_REDIRECT_URI` |
| Resend SMTP | `EMAIL_FROM`, `RESEND_API_KEY`, `RESEND_SMTP_HOST`, `RESEND_SMTP_PORT`, `RESEND_SMTP_USERNAME` |
| Crypto (optional) | `CRYPTO_PROVIDER`, then `BTCPAY_*` or `NOWPAYMENTS_*`: see [Crypto payments](#crypto-payments) |

Create server runtime env. Never commit it:

```sh
cp .env.prod.example .env.prod
# Fill values, then:
./__setup_prod_server_env.sh
```

Runtime secrets stay in `/root/cadence/.env`: Postgres credentials, `JWT_SECRET`,
Google OAuth client (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, redirect URI),
and Resend SMTP (`RESEND_API_KEY`, `RESEND_SMTP_*`, `EMAIL_FROM`). Docker
registry credentials stay in GitHub.

DNS: `A bookcadence.app -> 37.27.157.169`.

## Deploy

```sh
git checkout main
git pull --ff-only origin main
./deploy_prod.sh
```

Script checks quality, tags the commit, and starts `.github/workflows/deploy.yml`.
Workflow builds and pushes Docker images, SSHes to server, pulls tagged images,
starts Postgres/backend/frontend, configures Nginx/TLS, then smoke-tests.

Nginx sends `/api/` and `/.well-known/oauth-*` to the backend. The second is
MCP OAuth discovery (Plan 45): claude.ai and ChatGPT look for it at the site
root, so the backend answers there and not under `/api/`.

Plans 44 and 45 changed the access tokens CREATE migration in place and
renumbered `0022`–`0024`. A database migrated before them needs the reset
below before this deploy.

Skip local checks only when intentional:

```sh
./deploy_prod.sh --skip-quality
```

## Inspect

```sh
gh run list --workflow deploy.yml --limit 5
ssh root@37.27.157.169 'cd /root/cadence && docker compose --env-file .env -f docker-compose.prod.yml ps'
curl -fsS https://bookcadence.app/health
```

Production migrations run when backend starts. Schema changes edit the table's
CREATE migration, so a deploy that changes an applied migration needs a
production reset (below); otherwise sqlx reports a checksum mismatch and the
backend restarts in a loop. Revert bad deploy by reverting commit on `main`.

## Reset production database

This deletes all production data. Never use `reset_db.sh` here: it targets
`127.0.0.1` and loads the dev seed. Check the backend logs first:

```sh
ssh root@37.27.157.169 'docker logs --tail 50 "$(docker ps -aq --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=backend)"'
```

Then on the server, back up, stop the backend, recreate the database, and start
the backend so it runs the migrations. No seed is applied.

```sh
ssh root@37.27.157.169
cd /root/cadence
./scripts/backup-db.sh
DB=$(docker ps -q  --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=db)
BE=$(docker ps -aq --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=backend)
docker stop "$BE"
docker exec "$DB" sh -c 'psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d postgres \
  -c "DROP DATABASE \"$POSTGRES_DB\" WITH (FORCE);" \
  -c "CREATE DATABASE \"$POSTGRES_DB\";"'
docker start "$BE"
docker logs -f "$BE"
```

Verify with `curl -fsS https://bookcadence.app/health`.

## Crypto payments

Hosts can pay for a plan in crypto, opting in from Billing (Plans 48 and 49).
It stays off until `CRYPTO_PROVIDER` is set in `/root/cadence/.env`. Pick one
provider; switching later is a change to `.env` and a redeploy.

### BTCPay Server (self-hosted, no account anywhere)

Run BTCPay on its own small server, not on the app's. It needs a Bitcoin node,
and a pruned one keeps the disk small. Check BTCPay's docs for current
hardware needs before you size it.

1. **Server and DNS.** Point `A pay.bookcadence.app` at the new server.
2. **Install** with the official
   [btcpayserver-docker](https://github.com/btcpayserver/btcpayserver-docker):

   ```sh
   git clone https://github.com/btcpayserver/btcpayserver-docker
   cd btcpayserver-docker
   export BTCPAY_HOST="pay.bookcadence.app"
   export NBITCOIN_NETWORK="mainnet"
   export BTCPAYGEN_CRYPTO1="btc"
   export BTCPAYGEN_REVERSEPROXY="nginx"
   # A pruned node; see the repo's README for the storage fragments.
   export BTCPAYGEN_ADDITIONAL_FRAGMENTS="opt-save-storage-s"
   . ./btcpay-setup.sh -i
   ```

   Wait for the node to sync. The dashboard shows when it is done.
3. **Admin and store.** The first account you register is the admin. Turn on
   two-factor sign-in. Create a store named `Cadence` with USD as its default
   currency.
4. **Wallet, watch-only.**
   - Use Store → Wallets → Bitcoin → *Connect an existing wallet*, and enter
     the extended public key (xpub) of a hardware wallet.
   - Never generate a hot wallet here: with an xpub, no private key is on the
     server.
   - Refunds then wait in Store → Payouts until you approve and sign them with
     the hardware wallet.
5. **Checkout settings.**
   - Invoices expire after 15 minutes (the default).
   - Payment tolerance is 0%.
   - The speed policy is *Medium* (one confirmation), which is plenty for
     $10–$156 plans.
6. **Webhook.**
   - Go to Store → Settings → Webhooks → *Create*.
   - URL `https://bookcadence.app/api/billing/crypto/webhook`, every event,
     automatic redelivery on.
   - Copy the secret.
7. **API key.**
   - Go to Account → API keys → *Generate*, for the Cadence store only.
   - Give it these permissions: create invoices, view invoices, create pull
     payments (for refunds).
8. **App settings,** then redeploy:

   ```sh
   CRYPTO_PROVIDER=btcpay
   BTCPAY_URL=https://pay.bookcadence.app
   BTCPAY_STORE_ID=<Store settings → General → Store ID>
   BTCPAY_API_KEY=<the key from step 7>
   BTCPAY_WEBHOOK_SECRET=<the secret from step 6>
   ```

9. **Check it** with a small real payment:
   - `curl -fsS https://bookcadence.app/api/billing/crypto/options` answers
     `"available":true`.
   - Pay for a Pro month from Billing. In BTCPay, the webhook's deliveries
     show `200`.
   - `/admin/plans` shows the host with a *Crypto* badge.
   - Refund it from the row's menu, and claim the refund with the emailed link.

If a webhook is missed, the backend asks BTCPay about unsettled invoices every
5 minutes and credits a paid one anyway.

### NOWPayments (hosted, many coins and stablecoins)

1. **Account.** Make an account at nowpayments.io and set its payout wallet.
   Follow their KYC rules for your country and volume.
2. **Keys.** In the dashboard, create an API key and an IPN secret.
3. **App settings,** then redeploy:

   ```sh
   CRYPTO_PROVIDER=nowpayments
   NOWPAYMENTS_API_KEY=<API key>
   NOWPAYMENTS_IPN_SECRET=<IPN secret>
   NOWPAYMENTS_IPN_URL=https://bookcadence.app/api/billing/crypto/webhook
   ```

4. **Try the sandbox first.** It needs its own sandbox account. Set
   `NOWPAYMENTS_API_URL=https://api-sandbox.nowpayments.io`, because the
   adapter has only been checked against recorded IPNs.

Limits next to BTCPay:
- **Missed IPNs:** NOWPayments has no invoice lookup with an API key, so a
  missed IPN is not caught by the 5-minute check.
- **Refunds:** refunding from `/admin/plans` takes the time back but sends no
  money. Send the refund from the NOWPayments dashboard.

## Backups

`scripts/backup-db.sh` runs on the server. It dumps the `db` container with
`pg_dump --format=custom` into `/root/cadence-backups/` (root-only, UTC
timestamped) and deletes dumps older than 14 days. Deploy copies it to
`/root/cadence/scripts/`. Override with `BACKUP_DIR` and `BACKUP_KEEP_DAYS`.

Schedule it once per server, nightly at 03:15 server time:

```sh
ssh root@37.27.157.169 "echo '15 3 * * * root /root/cadence/scripts/backup-db.sh >> /var/log/cadence-backup.log 2>&1' > /etc/cron.d/cadence-backup"
```

Run one now, or pull the newest dump to `./backups/` (gitignored):

```sh
ssh root@37.27.157.169 /root/cadence/scripts/backup-db.sh
./scripts/pull-backup.sh
```

Check a dump locally by restoring it into a scratch database:

```sh
createdb -h 127.0.0.1 -U postgres cadence_restore_test
pg_restore -h 127.0.0.1 -U postgres -d cadence_restore_test --no-owner backups/cadence_<timestamp>.dump
```

Restore production only on purpose. This replaces existing objects. Containers
are found by compose labels because `docker compose` on the server needs the
deploy-time image variables:

```sh
ssh root@37.27.157.169 'docker stop "$(docker ps -aq --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=backend)"'
ssh root@37.27.157.169 'docker exec -i "$(docker ps -q --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=db)" sh -c '\''pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner'\''' < backups/cadence_<timestamp>.dump
ssh root@37.27.157.169 'docker start "$(docker ps -aq --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=backend)"'
```
