#!/usr/bin/env bash
set -euo pipefail

source_file="${1:-.env.prod}"
server_ip="${SERVER_IP:-23.88.45.210}"
ssh_key="${PROD_SSH_KEY:-$HOME/.ssh/hetzner-mac}"
deploy_user="${DEPLOY_USER:-root}"
deploy_dir="${DEPLOY_DIR:-/root/cadence}"

if [ ! -s "$source_file" ]; then
    echo "Missing non-empty env file: $source_file" >&2
    exit 1
fi

if [ ! -s "$ssh_key" ]; then
    echo "Missing SSH key: $ssh_key" >&2
    exit 1
fi

ssh_options=(-i "$ssh_key" -o BatchMode=yes -o StrictHostKeyChecking=yes)
scp "${ssh_options[@]}" "$source_file" "$deploy_user@$server_ip:/tmp/cadence.env"
ssh "${ssh_options[@]}" "$deploy_user@$server_ip" \
    "install -d -m 700 '$deploy_dir' && install -m 600 /tmp/cadence.env '$deploy_dir/.env' && rm -f /tmp/cadence.env"

echo "Server env installed at $deploy_user@$server_ip:$deploy_dir/.env"
