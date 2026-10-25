#!/usr/bin/env bash
# Sets up the local BTCPay Server from docker-compose.btcpay.yml for crypto
# checkout (Plan 48) and writes CRYPTO_PROVIDER and BTCPAY_* into .env.
# The store takes Bitcoin and Litecoin (Plan 53); a `coin` is btc or ltc.
# Regtest only: the coins are fake and no account is made anywhere. Safe to
# run again: the user, store and wallets are reused; the webhook and API key
# are made fresh, since BTCPay shows their secrets only once.
#
#   docker compose -f docker-compose.btcpay.yml up -d
#   scripts/btcpay-setup.sh                                  set up, then restart the backend
#   scripts/btcpay-setup.sh pay <address> <amount> [coin]    pay an address and confirm it
#   scripts/btcpay-setup.sh pay-invoice <link> [coin]        pay a checkout page's invoice in full
#   scripts/btcpay-setup.sh pay-invoice <link> short [coin]  pay half of it, to test a short payment
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
COINS=(btc ltc)
WALLET=dev
# A payment counts once its block is this deep; BTCPay asks for fewer.
CONFIRM_BLOCKS=6

# Runs a coin's node cli: `node_cli ltc getbalance`.
node_cli() {
    local service=bitcoind cli=bitcoin-cli
    if [[ "$1" == ltc ]]; then
        service=litecoind
        cli=litecoin-cli
    fi
    shift
    "${COMPOSE[@]}" exec -T "$service" "$cli" -regtest -rpcport=43782 \
        -rpcuser=cadence -rpcpassword=cadence "$@"
}

# BTCPay's payment method for a coin: BTC-CHAIN or LTC-CHAIN.
payment_method() {
    echo "${1^^}-CHAIN"
}

# The coin an optional argument names, btc when there is none.
coin_arg() {
    case "${1:-btc}" in
        btc | ltc) echo "${1:-btc}" ;;
        *) echo "Unknown coin $1: use btc or ltc." >&2; exit 1 ;;
    esac
}

# Mines `count` blocks of a coin to the dev wallet, which is where test coins
# come from: `mine ltc 6`.
mine() {
    local address
    address=$(node_cli "$1" -rpcwallet="$WALLET" getnewaddress)
    node_cli "$1" generatetoaddress "$2" "$address" >/dev/null
}

if [[ "${1:-}" == "pay-invoice" ]]; then
    [[ $# -ge 2 ]] || { echo "Usage: scripts/btcpay-setup.sh pay-invoice <checkout link or invoice id> [short] [btc|ltc]"; exit 1; }
    amount=full
    if [[ "${3:-}" == short || "${3:-}" == full ]]; then
        amount=$3
        coin=$(coin_arg "${4:-}")
    else
        coin=$(coin_arg "${3:-}")
    fi
    key=$(grep -E '^BTCPAY_DEV_ADMIN_KEY=' "${BTCPAY_SETUP_ENV_FILE:-.env}" | tail -1 | cut -d= -f2-)
    store=$(grep -E '^BTCPAY_STORE_ID=' "${BTCPAY_SETUP_ENV_FILE:-.env}" | tail -1 | cut -d= -f2-)
    [[ -n "$key" && -n "$store" ]] || { echo "Run scripts/btcpay-setup.sh first."; exit 1; }
    invoice=${2##*/}
    methods=$(curl -sf -H "Authorization: token $key" \
        "$URL/api/v1/stores/$store/invoices/$invoice/payment-methods") ||
        { echo "No invoice $invoice in the local BTCPay."; exit 1; }
    read -r address due < <(python3 -c '
import json, sys
method = next(m for m in json.loads(sys.argv[1]) if m["paymentMethodId"] == sys.argv[3])
due = float(method["due"]) / (2 if sys.argv[2] == "short" else 1)
print(method["destination"], f"{due:.8f}")' "$methods" "$amount" "$(payment_method "$coin")")
    node_cli "$coin" -rpcwallet="$WALLET" sendtoaddress "$address" "$due" >/dev/null
    mine "$coin" "$CONFIRM_BLOCKS"
    echo "Paid $due ${coin^^} on invoice $invoice and mined $CONFIRM_BLOCKS blocks."
    exit 0
fi

if [[ "${1:-}" == "pay" ]]; then
    [[ $# -ge 3 ]] || { echo "Usage: scripts/btcpay-setup.sh pay <address> <amount> [btc|ltc]"; exit 1; }
    coin=$(coin_arg "${4:-}")
    node_cli "$coin" -rpcwallet="$WALLET" sendtoaddress "$2" "$3"
    mine "$coin" "$CONFIRM_BLOCKS"
    echo "Paid $3 ${coin^^} to $2 and mined $CONFIRM_BLOCKS blocks."
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

# A wallet in each node with mined coins to pay invoices from. Coinbase
# coins can be spent after 100 blocks.
for coin in "${COINS[@]}"; do
    node_cli "$coin" createwallet "$WALLET" >/dev/null 2>&1 ||
        node_cli "$coin" loadwallet "$WALLET" >/dev/null 2>&1 || true
    # The image may start with blocks of its own, so mine until coins are spendable.
    for _ in 1 2 3; do
        [[ $(node_cli "$coin" -rpcwallet="$WALLET" getbalance | cut -d. -f1) -ge 1 ]] && break
        mine "$coin" 101
        echo "Mined 101 ${coin^^} blocks for test coins."
    done
done

# BTCPay takes each coin once its explorer has caught up with those blocks.
for _ in $(seq 1 40); do
    [[ $(btcpay_api GET /api/v1/server/info | grep -o '"available":true' | wc -l) -ge ${#COINS[@]} ]] && break
    read -rt 3 < <(tail -f /dev/null) || true
done

# A hot wallet per coin whose keys BTCPay keeps: fine for fake coins only.
for coin in "${COINS[@]}"; do
    method=$(payment_method "$coin")
    if ! btcpay_api --allow-fail GET "/api/v1/stores/$STORE_ID/payment-methods/$method" |
        grep -q '"enabled":true'; then
        btcpay_api POST "/api/v1/stores/$STORE_ID/payment-methods/$method/wallet/generate" \
            '{"savePrivateKeys":true,"scriptPubKeyType":"Segwit"}' >/dev/null
        echo "Made the store's ${coin^^} wallet."
    fi
done

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
# Quoted: dotenv refuses a bare value with spaces.
set_env BTCPAY_COINS "\"Bitcoin or Litecoin\""

echo "Done: wrote CRYPTO_PROVIDER and BTCPAY_* to $ENV_FILE. Restart the backend."
echo "To pay a checkout page's invoice in BTC or LTC:"
echo "  scripts/btcpay-setup.sh pay-invoice <link> [short] [btc|ltc]"
