#!/usr/bin/env bash
# Applies the development seed to the existing local database. Safe to run
# again: seeded logins are switched back on, other rows are left alone.
set -euo pipefail

cd "$(dirname "$0")/.."

refuse_production() {
    for value in "${APP_ENV:-}" "${ENVIRONMENT:-}"; do
        if [[ "${value,,}" == "production" ]]; then
            echo "Refuse to seed: APP_ENV/ENVIRONMENT is production."
            exit 1
        fi
    done
}

# Checked before and after .env, which would replace what the caller set.
refuse_production
if [[ -f .env ]]; then
    set -a
    # shellcheck disable=SC1091
    source .env
    set +a
fi
refuse_production

: "${POSTGRES_USER:?POSTGRES_USER must be set}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD must be set}"
: "${POSTGRES_DATABASE:?POSTGRES_DATABASE must be set}"

export PGPASSWORD="$POSTGRES_PASSWORD"
psql -v ON_ERROR_STOP=1 --single-transaction -h 127.0.0.1 -U "$POSTGRES_USER" -d "$POSTGRES_DATABASE" \
    -f seeds/dev_seed.sql
