# Plan 6: deployment and embed

Status: in progress.

## Goal

Cadence runs behind HTTPS in production and the public booking widget embeds on
an approved external website without exposing the host dashboard.

## Current gap

Production Docker images, registry deploy, SSH container swap, TLS config, and
server environment bootstrap now exist. Embed, backups, and frame policy remain.

## Approach

- Build and push backend and frontend separately.
- Keep runtime secrets on the server and pass image tags only during deploy.
- Run migrations during backend startup before serving traffic.
- Keep API origin configurable through environment values.
- Use an iframe loader with `postMessage`, strict origins, and no dashboard
  cookie access in the embedded surface.

## Milestones

- D0: production env contract and builds, done.
- D1: TLS, reverse proxy, registry deploy script, done.
- D2: backups, iframe and origin policy.
- D3: CI deploy and rollback check, in progress.

## Definition of done

- HTTPS health check passes.
- Secrets are not committed.
- Migration failure stops rollout.
- Embed rejects unapproved origins and reports booking success.
