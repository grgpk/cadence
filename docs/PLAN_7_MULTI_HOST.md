# Plan 7: multi-host onboarding

Status: planned.

## Goal

A new host creates an account, configures a widget and calendar, and shares a
unique booking URL. Hosts never see one another's leads or bookings.

## The gap this fixes

Current tables carry `host_unid` for leads, availability, widget settings, and
bookings, but the public widget uses configured host IDs and onboarding/settings
experience is not complete.

## Approach

- Add a unique public host slug with a safe migration and backfill.
- Scope every host read and write by authenticated host or resolved public host.
- Keep admin queries unscoped only behind `AdminUser`.
- Add onboarding as small idempotent steps.

## Data model

Audit host-owned tables before new columns. Backfill the first host, then add
`NOT NULL`. Add unique indexes for public slug and provider account identity.

## API

| Method | Path | Guard | Result |
| --- | --- | --- | --- |
| POST | `/api/hosts` | logged in | host profile |
| GET | `/api/hosts/{slug}/slots` | public | scoped slots |
| GET | `/api/host/settings` | Host | own settings |
| PUT | `/api/host/settings` | Host | updated settings |
| GET | `/api/host/bookings` | Host | own bookings |

## Flow schema

```
register ─▶ onboarding ─▶ timezone ─▶ availability ─▶ calendar ─▶ preview
visitor ─────────────────────────────▶ /book/{slug}
```

## Out of scope

- Teams, round robin, delegated admins, custom domains, paid plans.

## Milestones

- H0: schema audit and host slug.
- H1: isolation middleware and query coverage.
- H2: onboarding/settings UI.
- H3: per-host calendar and widget.
- H4: cross-host security tests.

## Definition of done

- Host A cannot read or mutate Host B data.
- Admin sees all hosts through explicit admin endpoints.
- A new host publishes a working booking URL without manual SQL.
- All host queries and migrations have tests.
