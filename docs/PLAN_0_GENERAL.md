# Plan 0: overview

Map for `cadence`. Each phase has its own `PLAN_N_*.md`. This file stays short
and reflects the repository as it exists.

## North star

Simple call booking for a host: public widget, Rust API, email queue, and
protected dashboards. Admin sees the whole funnel. Host sees own data.

```
visitor ──▶ booking widget ──▶ slots ──▶ booking ──▶ email queue
                                      │
host ─────▶ /dashboard ◀──────────────┘
admin ────▶ /admin ─────▶ leads, bookings, visits, roles, bugs
```

## Current stack

- `backend/`: Rust 2024, axum, SQLx/Postgres, JWT cookie auth, tracing,
  `lettre`, and `ts-rs`.
- `frontend/`: TanStack Start/Vite, TanStack Router, TanStack Query,
  shadcn, Radix, Tailwind v4, Lucide, Biome, and Knip.
- `migrations/`: shared SQLx migrations. One schema concern per file,
  `0001_XXXX.sql` names, development seed reserved as
  `9999____seed_data.sql`.
- `request.http`: API smoke requests. Root `pnpm run dev` starts backend and
  frontend together.

## Principles

- Backend and frontend stay separate and communicate with JSON HTTP through
  `VITE_API_URL`.
- Wire contracts originate in Rust `#[derive(TS)]` structs and export into
  `frontend/src/bindings/`. Generated bindings are never hand-edited.
- Backend call direction is `routes -> handlers -> service -> db -> Postgres`.
  SQL stays in `db.rs`, business rules in `service.rs`.
- Admin is authorization, not a parallel backend domain. Domain-owned admin
  routes use `AdminUser`. Frontend admin URL is `/admin`, never
  `/dashboard/admin`.
- Server data uses TanStack Query. UI uses shadcn components and Lucide icons.
  Primary color is black. Legacy global `.card`, `.button`, and `.input` CSS
  do not exist.
- Booking side effects enqueue mail in `email_queue`; the worker owns SMTP.
- Store instants as `TIMESTAMPTZ`; timezone values are IANA names.
- No magic role, status, or route strings when a typed constant can own them.

## Architecture

```
cadence/
  backend/
    src/
      main.rs                 env, tracing, migrations, worker, axum server
      app_state.rs            AppState { pool }
      domain/
        auth/                  registration, login, JWT, AdminUser
        availability/          recurring availability and slots
        bookings/              booking creation and host/admin reads
        bug_reports/           client reports and admin aggregation
        calling_visits/        funnel tracking
        emails/                queue access, templates, worker
        leads/                 lead capture and admin reads
        roleaccesses/          role grants and admin reads
  frontend/
    src/
      bindings/               generated ts-rs contracts
      components/ui/           shadcn primitives
      domains/admin/           admin sidebar and data pages
      domains/dashboard/       dashboard sidebar and user menu
      domains/widget/          public booking widget
      routes/admin/            protected /admin pages
      routes/dashboard/        protected /dashboard pages
  migrations/                  SQLx schema and development seed
  docs/                        living plan files
```

## Phases

| # | File | Scope | Status |
| --- | --- | --- | --- |
| 0 | `PLAN_0_GENERAL.md` | map, architecture, working agreement | living |
| 1 | `PLAN_1_FOUNDATION.md` | workspace, tracing, migrations, dev tooling, ts-rs, shadcn, quality gates | ✅ done |
| 2 | `PLAN_2_AUTH_ADMIN.md` | auth, roleaccesses, protected routes, `/admin` and `/dashboard` shells | ✅ done |
| 3 | `PLAN_3_BOOKING_CORE.md` | leads, visits, availability, widget settings, bookings, query UI, seed data | 🔄 in progress |
| 4 | `PLAN_4_CALENDAR_EMAIL.md` | Google Calendar busy filtering, Meet links, Resend delivery and retries | 📅 planned |
| 5 | `PLAN_5_TEST_HARDENING.md` | HTTP contract tests, auth matrix, migration and UI verification | 📅 planned |
| 6 | `PLAN_6_DEPLOYMENT_EMBED.md` | production build, HTTPS deployment, embeddable widget | 📅 planned |
| 7 | `PLAN_7_MULTI_HOST.md` | host onboarding, isolation, custom widget branding | 📅 planned |

Status: ✅ done, 🔄 in progress, 📅 planned, living.

## Why this order

```
1 foundation ─▶ 2 auth/admin ─▶ 3 booking core ─▶ 4 calendar/email
                                      │                    │
                                      └──────────────▶ 5 tests/hardening
                                                         │
                                      6 deploy/embed ◀────┘
                                                         │
                                      7 multi-host ◀──────┘
```

Auth comes before host data and admin UI. Booking core comes before external
calendar integration. Tests lock API and authorization before deployment.
Multi-host comes last because host scoping changes every booking query and URL.

## Out of scope

Teams, round robin, payments, recurring bookings, public API keys, Outlook,
CalDAV, Zoom, SMS, WhatsApp, Discord, i18n, and native mobile apps. Revisit
after multi-host onboarding.

## Working agreement

- Land or update the phase plan with the implementation.
- Run `cargo fmt --all -- --check` and strict Clippy before commit.
- Run frontend build, typecheck, Biome, and Knip before commit.
- After schema changes, run migration tests and inspect SQLx behavior.
- After wire model changes, run `cargo test --test export_bindings`.
- Commit straight to `main`, then push.
