#!/usr/bin/env bash
# Creates Cadence's products and prices in Stripe and writes their price ids
# into .env. Reads STRIPE_SECRET_KEY from .env. Safe to run again: prices are
# found by their lookup key and reused, never duplicated.
#
#   scripts/stripe-setup.sh          test mode (sk_test_ or rk_test_ key)
#   scripts/stripe-setup.sh --live   live mode, for production only
#
# The amounts match backend/src/domain/billing/plans.rs.
set -euo pipefail

cd "$(dirname "$0")/.."

# Another file for trying the script out; .env otherwise.
ENV_FILE=${STRIPE_SETUP_ENV_FILE:-.env}
API=https://api.stripe.com/v1
CURRENCY=usd

if [[ ! -f "$ENV_FILE" ]]; then
    echo "No $ENV_FILE here. Copy .env.example to .env first."
    exit 1
fi
KEY=$(grep -E '^STRIPE_SECRET_KEY=' "$ENV_FILE" | tail -1 | cut -d= -f2- | tr -d '"' || true)
if [[ -z "$KEY" ]]; then
    echo "Add STRIPE_SECRET_KEY=sk_test_... to $ENV_FILE first (Stripe dashboard → Developers → API keys)."
    exit 1
fi
if [[ "${1:-}" == "--live" ]]; then
    [[ "$KEY" == sk_live_* || "$KEY" == rk_live_* ]] || { echo "--live needs a live key."; exit 1; }
else
    [[ "$KEY" == sk_test_* || "$KEY" == rk_test_* ]] || {
        echo "That is not a test key. Use a sk_test_ key, or pass --live on purpose."
        exit 1
    }
fi

# Calls the API, prints the JSON, and stops on an error Stripe reports.
stripe_api() {
    local method=$1 path=$2
    shift 2
    local body
    body=$(curl -sS -X "$method" "$API$path" -u "$KEY:" "$@")
    if python3 -c 'import json,sys; sys.exit(0 if "error" in json.loads(sys.argv[1]) else 1)' "$body"; then
        echo "Stripe refused $method $path:" >&2
        python3 -c 'import json,sys; print(json.loads(sys.argv[1])["error"].get("message"))' "$body" >&2
        exit 1
    fi
    printf '%s' "$body"
}

# Reads one field of a JSON answer: `json_field "$json" data.0.id`.
json_field() {
    python3 -c '
import json, sys
value = json.loads(sys.argv[1])
for key in sys.argv[2].split("."):
    value = value[int(key)] if key.isdigit() else value[key]
print(value)
' "$1" "$2"
}

# How many results a list or search answer holds.
count_of() {
    python3 -c 'import json, sys; print(len(json.loads(sys.argv[1])["data"]))' "$1"
}

# The product a plan's prices hang on, found by its metadata or made.
product_for() {
    local plan=$1 name=$2
    local found
    found=$(stripe_api GET "/products/search" -G --data-urlencode "query=metadata['cadence_plan']:'$plan'")
    if [[ $(count_of "$found") != "0" ]]; then
        json_field "$found" "data.0.id"
        return
    fi
    local made
    made=$(stripe_api POST "/products" -d "name=$name" -d "metadata[cadence_plan]=$plan")
    json_field "$made" "id"
}

# A recurring price, found by its lookup key or made.
price_for() {
    local product=$1 lookup_key=$2 amount=$3 interval=$4
    local found
    found=$(stripe_api GET "/prices" -G -d "lookup_keys[]=$lookup_key")
    if [[ $(count_of "$found") != "0" ]]; then
        json_field "$found" "data.0.id"
        return
    fi
    local made
    made=$(stripe_api POST "/prices" \
        -d "product=$product" \
        -d "currency=$CURRENCY" \
        -d "unit_amount=$amount" \
        -d "recurring[interval]=$interval" \
        -d "lookup_key=$lookup_key")
    json_field "$made" "id"
}

# Sets NAME=value in .env, replacing an existing line or adding one.
set_env() {
    local name=$1 value=$2
    if grep -qE "^$name=" "$ENV_FILE"; then
        sed -i "s|^$name=.*|$name=$value|" "$ENV_FILE"
    else
        printf '%s=%s\n' "$name" "$value" >> "$ENV_FILE"
    fi
}

echo "Products..."
PRO=$(product_for pro "Cadence Pro")
BUSINESS=$(product_for business "Cadence Business")

echo "Prices..."
set_env STRIPE_PRICE_PRO_MONTH "$(price_for "$PRO" cadence_pro_month 1000 month)"
set_env STRIPE_PRICE_PRO_YEAR "$(price_for "$PRO" cadence_pro_year 9600 year)"
set_env STRIPE_PRICE_BUSINESS_MONTH "$(price_for "$BUSINESS" cadence_business_month 1600 month)"
set_env STRIPE_PRICE_BUSINESS_YEAR "$(price_for "$BUSINESS" cadence_business_year 15600 year)"

echo
echo "Done. $ENV_FILE now has:"
grep -E '^STRIPE_PRICE_' "$ENV_FILE"
echo
echo "Still to do:"
echo "  1. Customer portal: allow switching between the Pro and Business prices and"
echo "     cancelling at the end of the period, then Save:"
echo "     https://dashboard.stripe.com/test/settings/billing/portal"
echo "  2. Webhook secret: run 'stripe listen' (see the plan) and put its whsec_ in"
echo "     STRIPE_WEBHOOK_SECRET, then restart the backend."
