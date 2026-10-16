#!/usr/bin/env bash
# Copies the newest production dump from the server into ./backups/.
set -euo pipefail

cd "$(dirname "$0")/.."

SERVER="${SERVER:-root@37.27.157.169}"
BACKUP_DIR="${BACKUP_DIR:-/root/cadence-backups}"

# shellcheck disable=SC2029 # BACKUP_DIR is meant to expand locally.
latest=$(ssh "$SERVER" "ls -1t '$BACKUP_DIR'/cadence_*.dump | head -n 1")
if [[ -z "$latest" ]]; then
    echo "No backups found in $SERVER:$BACKUP_DIR" >&2
    exit 1
fi

mkdir -p backups
chmod 700 backups
scp "$SERVER:$latest" backups/
echo "Pulled: backups/$(basename "$latest")"
