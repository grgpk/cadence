#!/usr/bin/env bash
# Dumps the production Postgres container to $BACKUP_DIR and prunes old dumps.
# Runs on the server from cron; see DEPLOYMENT.md "Backups".
set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/root/cadence-backups}"
BACKUP_KEEP_DAYS="${BACKUP_KEEP_DAYS:-14}"
COMPOSE_PROJECT="${COMPOSE_PROJECT:-cadence}"

# Found by compose labels: `docker compose` itself would need the deploy-time
# image variables that are not kept in the server .env.
container=$(docker ps -q \
    --filter "label=com.docker.compose.project=$COMPOSE_PROJECT" \
    --filter "label=com.docker.compose.service=db")
if [[ -z "$container" ]]; then
    echo "No running db container for compose project '$COMPOSE_PROJECT'" >&2
    exit 1
fi

umask 077
mkdir -p "$BACKUP_DIR"
target="$BACKUP_DIR/cadence_$(date -u +'%Y-%m-%dT%H%M%SZ').dump"
partial="$target.partial"
trap 'rm -f "$partial"' EXIT

# Credentials come from the container's own POSTGRES_* environment.
docker exec "$container" sh -c \
    'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --format=custom --no-owner' \
    > "$partial"
docker exec -i "$container" pg_restore --list < "$partial" > /dev/null
mv "$partial" "$target"

find "$BACKUP_DIR" -name 'cadence_*.dump' -mtime +"$BACKUP_KEEP_DAYS" -delete

echo "Backup written: $target ($(du -h "$target" | cut -f1))"
