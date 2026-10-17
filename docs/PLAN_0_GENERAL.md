# Plan 0: overview

Map for `cadence`. Each phase has its own `PLAN_N_*.md`. This file stays short
and reflects the repository as it exists.

## North star

Simple call booking: a host registers, sets weekly hours, and shares their own
link. Rust API, email queue, and protected dashboards. Admin sees the whole
funnel. Host sees own data.

```
visitor ──▶ /book/{slug} ──▶ slots ──▶ booking ──▶ email queue
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
  `0001_XXXX.sql` names. The development seed is `seeds/dev_seed.sql`,
  applied by `reset_db.sh` or `scripts/seed-dev.sh` and never by the server.
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
  Primary color is ink (Plan 9). Legacy global `.card`, `.button`, and `.input` CSS
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
        google_calendar/       optional busy times and Meet links per host
        host_settings/         a host's link, page text and form questions
        leads/                 lead capture and admin reads
        roleaccesses/          role grants and admin reads
  frontend/
    src/
      bindings/               generated ts-rs contracts
      components/ui/           shadcn primitives
      domains/admin/           admin sidebar and data pages
      domains/dashboard/       dashboard sidebar and calendar card
      domains/landing/         home page and the day tape
      domains/widget/          public booking card
      routes/admin/            protected /admin pages
      routes/dashboard/        protected /dashboard pages
  migrations/                  SQLx schema
  seeds/                       development seed, local databases only
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
| 7 | `PLAN_7_MULTI_HOST.md` | per-host links and settings, isolation, slots from the host's hours | ✅ done |
| 8 | `PLAN_8_UI_REFRESH.md` | one token-driven look, time-first booking card, availability editor | ✅ done |
| 9 | `PLAN_9_DAY_TAPE.md` | "day tape" identity: palette, type, landing page, motion | ✅ done |
| 10 | `PLAN_10_ADMIN_CLARITY.md` | admin health overview, booking page status, grouped settings, time picker | ✅ done |
| 11 | `PLAN_11_FRONTEND_HARDENING.md` | session and error handling, booking form accessibility, typed statuses, frontend tests | 🔄 in progress |
| 12 | `PLAN_12_MOTION_POLISH.md` | view transitions, landing entrance, theme reveal, pending states, booking and loading fixes | 🔄 in progress |
| 13 | `PLAN_13_DAY_IN_APP.md` | day tape in the dashboard and sign-in, type scale, add to calendar, booking rules, booking details, cancel | 🔄 in progress |
| 14 | `PLAN_14_INVITEE_CONTROL.md` | invitee reschedule and cancel link, host emails, HTML emails, availability week view, bookings agenda, paused page | 🔄 in progress |
| 15 | `PLAN_15_ACCOUNT_SAFETY.md` | password reset and account, rate limits, gap between calls, bookings week view, admin on phones, link previews | 🔄 in progress |
| 16 | `PLAN_16_HARDENING.md` | seed out of production, stored call length and overlap lock, session versions, email worker claiming, Google timeouts, frontend bug and accessibility fixes | 🔄 in progress |
| 17 | `PLAN_17_INTEGRATIONS.md` | Integrations page, Google Calendar moved there, default meeting app (Google Meet or own link), Outlook/Apple/Zoom/Teams shown as coming soon | 🔄 in progress |
| 18 | `PLAN_18_APPS_AND_CALENDAR.md` | App store, app pages and Installed apps under an Apps nav group; bookings calendar with day, week and month | 🔄 in progress |
| 19 | `PLAN_19_AVAILABILITY.md` | weekly hours drawn, moved and stretched on the day tape; dates taken off or given other hours | 🔄 in progress |
| 20 | `PLAN_20_OVERVIEW_FOCUS.md` | overview leads with setup problems and the call that matters now; one Today card; date overrides on the day strip | 🔄 in progress |
| 21 | `PLAN_21_BOOKING_VIEWS.md` | booking page gets week and list views beside the month calendar; `?view=` opens one | 🔄 in progress |
| 22 | `PLAN_22_SLOT_POLICY.md` | one slot policy module decides and reserves open times; host calendar behind a seam with Google and in-memory adapters | 🔄 in progress |
| 23 | `PLAN_23_EMAIL_QUEUE.md` | the email queue decides which emails a booked, moved or cancelled call sends; typed payloads; one writer of the queue | 🔄 in progress |
| 24 | `PLAN_24_CIVIL_DAY.md` | one civil day module for keys, weeks, month grids and labels; zoned time moved to lib; week labels read "Oct 19 to 25" | 🔄 in progress |
| 25 | `PLAN_25_BOOKING_SESSION.md` | booking session module: visit, lead and booking outside React, one confirmation address type, one taken-time check | 🔄 in progress |
| 26 | `PLAN_26_SLOT_PICKER.md` | one slot picker for booking and rescheduling: month, week and list on both pages, one pick and set of states, one 12h/24h choice | 🔄 in progress |
| 27 | `PLAN_27_PRICING_BILLING.md` | public pricing page, Free/Pro/Business, Stripe checkout and portal, 14-day Pro trial, plan limits enforced, billing page | ✅ done |
| 28 | `PLAN_28_EVENT_TYPES.md` | several event types per host, `/book/$slug/$type`, Business only | ✅ done |
| 29 | `PLAN_29_WEBHOOKS.md` | signed booking webhooks with retries, Business only | ✅ done |
| 30 | `PLAN_30_PRICING_PROMISES.md` | Free never turns invitees away, price lock, nothing deleted; promises on `/pricing` | ✅ done |
| 31 | `PLAN_31_PRO_TYPES_AND_POLISH.md` | several kinds of call in Pro, promises on the home page, booking flow polish (taken time, Outlook, what happens next) | ✅ done |
| 32 | `PLAN_32_CALENDLY_IMPORT.md` | import event types, weekly hours, dates and timezone from Calendly with a personal access token | ✅ done |
| 33 | `PLAN_33_BUSINESS_CLIENTS.md` | Business: win back people who left, invitees confirm they are coming, no-shows, weekly numbers | ✅ done |
| 34 | `PLAN_34_PROMISE_AND_FUNNEL.md` | accent colour for Pro, Calendly import offered to new hosts, a trial end that says what this host loses | ✅ done |
| 35 | `PLAN_35_PAY_YEARLY.md` | monthly payers see what yearly saves and switch on Stripe's confirm screen | ✅ done |
| 36 | `PLAN_36_REFERRALS.md` | invite links: a month of Pro as Stripe credit to both hosts once the invited one pays; Free booking pages carry the invite | ✅ done |

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
Multi-host came after the booking core because host scoping changes every
booking query and URL. Plan 9 restyles what Plans 7 and 8 built.

## Out of scope

Teams (as in team scheduling), round robin, payments, recurring bookings,
public API keys, SMS, WhatsApp, Discord, i18n, and native mobile apps. Revisit
after multi-host onboarding. Outlook, CalDAV, Zoom and Microsoft Teams
meetings are follow-ups to Plans 17 and 18.

## Working agreement

- Land or update the phase plan with the implementation.
- Run `cargo fmt --all -- --check` and strict Clippy before commit.
- Run frontend build, typecheck, Biome, and Knip before commit.
- After schema changes, run migration tests and inspect SQLx behavior.
- After wire model changes, run `cargo test --test export_bindings`.
- Commit straight to `main`, then push.
