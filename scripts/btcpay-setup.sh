#!/usr/bin/env bash
# Sets up the local BTCPay Server from docker-compose.btcpay.yml for crypto
# checkout (Plan 48) and writes CRYPTO_PROVIDER and BTCPAY_* into .env.
# Regtest only: the coins are fake and no account is made anywhere. Safe to
# run again: the user, store and wallet are reused; the webhook and API key
# are made fresh, since BTCPay shows their secrets only once.
#
#   docker compose -f docker-compose.btcpay.yml up -d
#   scripts/btcpay-setup.sh                      set up, then restart the backend
#   scripts/btcpay-setup.sh pay <address> <btc>  pay an invoice and confirm it
#
# The BTCPay dashboard is http://localhost:49392, signed in with the
# ADMIN_EMAIL and ADMIN_PASSWORD below.
#
# BTCPay takes the password over its API only in an account's first 5
# minutes, so the first run saves an API key as BTCPAY_DEV_ADMIN_KEY and later
# runs use that. Lost it? Start over with
#   docker compose -f docker-compose.btcpay.yml down -v
set -euo pipefail

cd "$(dirname "$0")/.."

# Another file for trying the script out; .env otherwise.
ENV_FILE=${BTCPAY_SETUP_ENV_FILE:-.env}
COMPOSE=(docker compose -f docker-compose.btcpay.yml)
URL=http://localhost:49392
ADMIN_EMAIL=admin@cadence.local
ADMIN_PASSWORD=cadence-dev-only
STORE_NAME="Cadence (dev)"
# The backend on the host, as the BTCPay container reaches it.
WEBHOOK_URL=http://host.docker.internal:3001/api/billing/crypto/webhook
PAYMENT_METHOD=BTC-CHAIN
WALLET=dev
# A payment counts once its block is this deep; BTCPay asks for fewer.
CONFIRM_BLOCKS=6

bitcoin_cli() {
    "${COMPOSE[@]}" exec -T bitcoind bitcoin-cli -regtest -rpcport=43782 \
        -rpcuser=cadence -rpcpassword=cadence "$@"
}

# Mines `count` blocks to the dev wallet, which is where test coins come from.
mine() {
    local address
    address=$(bitcoin_cli -rpcwallet="$WALLET" getnewaddress)
    bitcoin_cli generatetoaddress "$1" "$address" >/dev/null
}

if [[ "${1:-}" == "pay" ]]; then
    [[ $# -eq 3 ]] || { echo "Usage: scripts/btcpay-setup.sh pay <address> <btc>"; exit 1; }
    bitcoin_cli -rpcwallet="$WALLET" sendtoaddress "$2" "$3"
    mine "$CONFIRM_BLOCKS"
    echo "Paid $3 BTC to $2 and mined $CONFIRM_BLOCKS blocks."
    exit 0
fi

if [[ ! -f "$ENV_FILE" ]]; then
    echo "No $ENV_FILE here. Copy .env.example to .env first."
    exit 1
fi

# How the admin signs in: the password until the setup key exists. BTCPay
# rate-limits password sign-ins, so the rest goes through the key.
AUTH=(-u "$ADMIN_EMAIL:$ADMIN_PASSWORD")

# Calls the Greenfield API as the admin and prints the JSON. Stops on an
# error BTCPay reports, unless `--allow-fail` comes first.
btcpay_api() {
    local allow_fail=false
    if [[ "$1" == "--allow-fail" ]]; then
        allow_fail=true
        shift
    fi
    local method=$1 path=$2 body=${3:-}
    local args=(-sS -X "$method" "$URL$path" "${AUTH[@]}"
        -H "Content-Type: application/json" -w '\n%{http_code}')
    [[ -n "$body" ]] && args+=(-d "$body")
    local answer status
    answer=$(curl "${args[@]}")
    status=${answer##*$'\n'}
    answer=${answer%$'\n'*}
    if [[ "$status" -ge 400 && "$allow_fail" == false ]]; then
        echo "BTCPay refused $method $path ($status): $answer" >&2
        exit 1
    fi
    printf '%s' "$answer"
}

# Reads one field of a JSON answer: `json_field "$json" apiKey`.
json_field() {
    python3 -c '
import json, sys
value = json.loads(sys.argv[1])
for key in sys.argv[2].split("."):
    value = value[int(key)] if isinstance(value, list) else value.get(key)
    if value is None:
        break
print("" if value is None else value)' "$1" "$2"
}

# Sets KEY=value in the env file, replacing the line if there is one.
set_env() {
    local key=$1 value=$2
    if grep -qE "^$key=" "$ENV_FILE"; then
        python3 -c '
import re, sys
path, key, value = sys.argv[1:]
text = open(path).read()
open(path, "w").write(re.sub(rf"^{key}=.*$", f"{key}={value}", text, flags=re.M))' "$ENV_FILE" "$key" "$value"
    else
        # A file that does not end in a newline would glue the line onto its last.
        [[ -z "$(tail -c1 "$ENV_FILE")" ]] || printf '\n' >>"$ENV_FILE"
        printf '%s=%s\n' "$key" "$value" >>"$ENV_FILE"
    fi
}

echo "Waiting for BTCPay on $URL..."
curl -sf --retry 40 --retry-delay 3 --retry-all-errors --max-time 5 "$URL/api/v1/health" >/dev/null

# The first user may sign up without credentials and becomes the admin.
btcpay_api --allow-fail POST /api/v1/users \
    "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\",\"isAdministrator\":true}" >/dev/null

# An unrestricted key for this script only, kept in the env file between
# runs. The backend never reads it.
SETUP_KEY=$(grep -E '^BTCPAY_DEV_ADMIN_KEY=' "$ENV_FILE" | tail -1 | cut -d= -f2- || true)
if [[ -z "$SETUP_KEY" ]] || ! curl -sf -o /dev/null -H "Authorization: token $SETUP_KEY" \
    "$URL/api/v1/api-keys/current"; then
    SETUP_KEY=$(json_field "$(btcpay_api POST /api/v1/api-keys \
        '{"label":"cadence dev setup","permissions":["unrestricted"]}')" apiKey)
    # A failure inside $(...) does not stop the script by itself.
    [[ -n "$SETUP_KEY" ]] || { echo "Could not make the setup key. Try again in a few minutes."; exit 1; }
    set_env BTCPAY_DEV_ADMIN_KEY "$SETUP_KEY"
fi
AUTH=(-H "Authorization: token $SETUP_KEY")

STORES=$(btcpay_api GET /api/v1/stores)
STORE_ID=$(python3 -c '
import json, sys
print(next((s["id"] for s in json.loads(sys.argv[1]) if s["name"] == sys.argv[2]), ""))' \
    "$STORES" "$STORE_NAME")
if [[ -z "$STORE_ID" ]]; then
    STORE_ID=$(json_field "$(btcpay_api POST /api/v1/stores \
        "{\"name\":\"$STORE_NAME\",\"defaultCurrency\":\"USD\"}")" id)
    echo "Made store $STORE_ID."
fi

# A wallet in bitcoind with mined coins to pay invoices from. Coinbase coins
# can be spent after 100 blocks.
bitcoin_cli createwallet "$WALLET" >/dev/null 2>&1 || bitcoin_cli loadwallet "$WALLET" >/dev/null 2>&1 || true
# The image may start with blocks of its own, so mine until coins are spendable.
for _ in 1 2 3; do
    [[ $(bitcoin_cli -rpcwallet="$WALLET" getbalance | cut -d. -f1) -ge 1 ]] && break
    mine 101
    echo "Mined 101 blocks for test coins."
done

# BTCPay takes Bitcoin once its explorer has caught up with those blocks.
for _ in $(seq 1 40); do
    btcpay_api GET /api/v1/server/info | grep -q '"available":true' && break
    read -rt 3 < <(tail -f /dev/null) || true
done

# A hot wallet whose keys BTCPay keeps: fine for fake coins only.
if ! btcpay_api --allow-fail GET "/api/v1/stores/$STORE_ID/payment-methods/$PAYMENT_METHOD" |
    grep -q '"enabled":true'; then
    btcpay_api POST "/api/v1/stores/$STORE_ID/payment-methods/$PAYMENT_METHOD/wallet/generate" \
        '{"savePrivateKeys":true,"scriptPubKeyType":"Segwit"}' >/dev/null
    echo "Made the store's wallet."
fi

# One webhook to the backend, made again so its secret is known.
WEBHOOKS=$(btcpay_api GET "/api/v1/stores/$STORE_ID/webhooks")
for id in $(python3 -c '
import json, sys
print(" ".join(w["id"] for w in json.loads(sys.argv[1]) if w["url"] == sys.argv[2]))' \
    "$WEBHOOKS" "$WEBHOOK_URL"); do
    btcpay_api DELETE "/api/v1/stores/$STORE_ID/webhooks/$id" >/dev/null
done
WEBHOOK_SECRET=$(openssl rand -hex 24)
btcpay_api POST "/api/v1/stores/$STORE_ID/webhooks" \
    "{\"url\":\"$WEBHOOK_URL\",\"secret\":\"$WEBHOOK_SECRET\",\"authorizedEvents\":{\"everything\":true}}" >/dev/null

# The backend's key can make and read this store's invoices and refund them
# (a refund is a pull payment), nothing else.
API_KEY=$(json_field "$(btcpay_api POST /api/v1/api-keys \
    "{\"label\":\"cadence backend\",\"permissions\":[\"btcpay.store.cancreateinvoice:$STORE_ID\",\"btcpay.store.canviewinvoices:$STORE_ID\",\"btcpay.store.cancreatepullpayments:$STORE_ID\"]}")" apiKey)


set_env CRYPTO_PROVIDER btcpay
set_env BTCPAY_URL "$URL"
set_env BTCPAY_STORE_ID "$STORE_ID"
set_env BTCPAY_API_KEY "$API_KEY"
set_env BTCPAY_WEBHOOK_SECRET "$WEBHOOK_SECRET"

echo "Done: wrote CRYPTO_PROVIDER and BTCPAY_* to $ENV_FILE. Restart the backend."
echo "To pay an invoice, copy its address and amount from the checkout page:"
echo "  scripts/btcpay-setup.sh pay <address> <btc>"
