# Plan 6: deployment and embed

Status: planned.

## Goal

Cadence runs behind HTTPS in production and the public booking widget embeds on
an approved external website without exposing the host dashboard.

## The gap this fixes

The repository currently targets local ports `3000` and `3001`. Production
containers, TLS, domain secrets, embed script, frame policy, backups, and CI
deployment are not present.

## Approach

- Build backend and frontend separately.
- Run migrations explicitly before backend restart.
- Keep API origin configurable through environment values.
- Use an iframe loader with `postMessage`, strict origins, and no dashboard
  cookie access in the embedded surface.

## API

| Method | Path | Guard | Result |
| --- | --- | --- | --- |
| GET | `/embed.js` | public | widget loader |
| GET | `/embed/booking` | public | iframe booking surface |
| GET | `/health` | public | deploy health probe |

## Flow schema

```
external page ─▶ embed.js ─▶ iframe /embed/booking ─▶ public API
                                      └─ postMessage ready / booked
```

## Out of scope

- Multi-host custom domains, phase 7.
- Payments and public API keys, later.

## Milestones

- D0: production env contract and builds.
- D1: TLS, reverse proxy, backups, deploy script.
- D2: iframe and origin policy.
- D3: CI deploy and rollback check.

## Definition of done

- HTTPS health check passes.
- Secrets are not committed.
- Migration failure stops rollout.
- Embed rejects unapproved origins and reports booking success.

## After phase 6

Phase 7 introduces multiple hosts and scopes every host-owned query.
