#!/usr/bin/env bash

set -euo pipefail

DEV_PORTS=(3000 3001)

for port in "${DEV_PORTS[@]}"; do
  pids="$(lsof -tiTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)"
  if [[ -z "$pids" ]]; then
    continue
  fi

  echo "Stopping stale development listener(s) on port $port: $pids"
  while read -r pid; do
    [[ -z "$pid" ]] || kill -9 "$pid"
  done <<< "$pids"
done
