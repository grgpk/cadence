# Plan 0: overview

The map for `cadence`. Each phase has its own `PLAN_N_*.md` with the detail.
This file holds the vision, the stages, the architecture, and the phase
index. Keep it short; the phase files carry the weight.

## North star

A booking widget that any website can embed so visitors book a call, a Rust
API that owns the calendar logic, and a dashboard to see every call booked.

- **Visitor**: fills a short form in the widget, picks a free slot in their
  own time zone, gets a confirmation email with a Google Meet link, then
  reminders before the call.
- **Host**: their Google Calendar is connected once, so busy times are never
  offered and every booking lands in it with a Meet link.
- **Admin**: connects the Google Calendar, sees every call booked, every
  lead, the email queue.

## The three stages

```
 STAGE 1  local, solid base     book a call on localhost, real email received,
                                admin logs in and sees every call
 STAGE 2  online + embeddable   deployed, the widget runs inside other sites
                                (Giorgi's site, his personal site)
 STAGE 3  open to others        new people sign up, onboard, and customise
                                their own widget
```

## Principles

- **Backend and frontend are separate apps.** `backend/` is an axum API,
  `frontend/` is a Next.js app. They talk only through JSON over HTTP, through
  one Next.js `app/api/.../route.ts` per endpoint (same origin, so the session
  cookie just works).
- **One contract, generated.** Every type that crosses the wire is a Rust
  struct with `#[derive(TS)]`, exported by `ts-rs` to `frontend/src/bindings/`.
  Never write those TypeScript types by hand, never edit `bindings/`.
- **A domain is a folder, the same shape everywhere.** `routes.rs` (URLs),
  `handlers.rs` (extract, call service, map errors), `service.rs` (business
  rules), `models.rs` (wire types and `Sql*` rows), `db.rs` (SQL only).
  Handlers never hold SQL. Pure logic (time zone conversion) lives in its own
  file with no I/O. The same domains split the frontend: `src/domain/<x>/`
  holds that domain's `api.ts`, query keys and components.
- **Admin is a guard, not a domain.** Each domain owns its admin views:
  `admin_handlers.rs` next to `handlers.rs`, mounted under `/api/admin/<x>`
  by the domain's own `routes.rs`, every handler taking the `AdminUser`
  extractor from `auth/admin_guard.rs`. Admin bookings code lives in
  `bookings/`, not in a parallel admin tree.
- **The backend owns the calendar.** Slots, busy times and time zones are
  computed server side; the widget renders what the API returns. The widget
  owns the form: it scores the answers and saves the result on the lead.
- **Google Calendar is the source of busy times.** Slots are filtered by it,
  and a booking re-checks it right before creating the event.
- **Instants in UTC, IANA zones everywhere.** `TIMESTAMPTZ` in Postgres, slot
  times defined in one reference zone (`Europe/Paris`) and converted to the
  visitor's zone, DST handled by `time-tz`.
- **Emails go through a queue.** A booking writes its emails into
  `email_queue` (confirmation now, reminders later). A worker sends them.
  A request never calls the email provider directly.
- **One host until stage 3.** Stage 1 and 2 serve one host with one global
  Google Calendar connection. Phase 9 adds `host_id`, backfilled to that
  first host, before anyone else signs up.
- **Every endpoint ships with a test.** `backend/tests/<domain>_http.rs`,
  `#[sqlx::test(migrations = "../migrations")]` against the real router.
  `backend/tests/export_bindings.rs` exports every wire type.

## Architecture

Final layout (stage 3). Stage 1 creates every top level folder; domains are
added as their phase lands.

```
cadence/
  backend/                    axum API
  frontend/                   Next.js app
  migrations/                 sqlx migrations, shared by backend, tests and prod
  Dockerfile.backend          (stage 2)
  Dockerfile.frontend         (stage 2)
  docker-compose.prod.yml     backend + frontend + Postgres + nginx (stage 2)
  nginx.prod.conf             HTTPS, frame-ancestors for the embed (stage 2)
  deploy_prod.sh              build, push, migrate, restart (stage 2)
  scripts/                    seed admin, local helpers
```

Backend:

```
backend/
  Cargo.toml                  strict clippy lints (deny unwrap, expect, panic)
  clippy.toml  deny.toml  rustfmt.toml  rust-toolchain.toml
  src/
    main.rs                   env, tracing, pool, migrations on boot, spawn workers,
                              session layer, merge domain routers, serve
    lib.rs                    pub mod app_state, domain, utils
    app_state.rs              AppState { pool }
    utils/                    setup_tracing.rs, iso_date.rs
    domain/
      mod.rs
                              every domain: routes, handlers, service, models, db
      auth/                   + session.rs, admin_guard.rs (AdminUser: Role::Root,
                              else 401 / 403); GitHub login, current user, logout
      leads/                  + dropoff_task.rs (worker, every minute)
                              + admin_handlers.rs   GET /api/admin/leads
      google_calendar/        + client.rs (Google API), environment.rs (local or
                              prod credentials), timezone.rs (pure);
                              db.rs holds google_calendar_config
      bookings/               call_bookings rows
                              + admin_handlers.rs   GET /api/admin/bookings
      emails/                 + templates/*.html, queue_task.rs (worker, every
                              minute, Resend)
                              + admin_handlers.rs   GET /api/admin/email-queue
      dashboard/              admin overview: next calls, numbers of the week
                              GET /api/admin/dashboard
      hosts/                  (stage 3) host profile, lookup by username
      onboarding/             (stage 3) sign up wizard steps
      widget_settings/        (stage 3) questions, slot times, branding
  tests/
    export_bindings.rs        ts-rs export of every wire type to frontend/src/bindings/
    <domain>_http.rs          #[sqlx::test(migrations = "../migrations")] per domain
```

Frontend:

```
frontend/
  proxy.ts                            Next 16 request proxy (redirects)
  app/
    (site)/page.tsx                   local demo page with the widget inline
    (funnel)/calling/page.tsx         the widget, full page and iframe target
    (funnel)/calling/booking-confirmation/page.tsx
    (protected)/layout.tsx            redirects to /login when logged out
    (protected)/login/page.tsx
    (protected)/auth/github/callback/page.tsx
    (protected)/admin/                thin pages, each renders one domain's component:
                                      page.tsx (dashboard), bookings/, leads/,
                                      email-queue/, google-oauth-callback/
    (protected)/dashboard/            host dashboard (stage 3)
    api/                              one route.ts per backend endpoint:
      auth/  leads/[unid]/  leads/[unid]/submit/
      google-calendar/{login,oauth-config,disconnect,available-slots,
                       available-slots-batch,book}/
      admin/{dashboard,bookings,leads,email-queue}/
  src/
    bindings/                         generated by ts-rs, never edited
    domain/                           every domain: api.ts, query-keys-factory.ts,
                                      components/
      calling/                        the widget: qualification.ts, calendar-grid.ts,
                                      booking-widget-{2-steps,step1,step2,calendar}.tsx,
                                      timezone-combobox.tsx, inline-booking-widget.tsx,
                                      floating-booking-widget.tsx
      auth/                           login-page.tsx, use-current-user.ts
      bookings/                       bookings-table.tsx
      leads/                          leads-table.tsx
      emails/                         email-queue-table.tsx
      dashboard/                      dashboard-page.tsx, stat-card.tsx
    components/                       ui/ (shadcn), admin-sidenav.tsx,
                                      app-providers.tsx, query-provider.tsx
    hooks/
    lib/                              backend-origin.ts, backend-proxy.ts,
                                      backend-auth-proxy.ts, github-oauth.ts,
                                      query-client.ts
  public/embed.js                     the snippet other sites load (stage 2)
```

Inside one backend domain:

```
 domain/google_calendar/
   mod.rs          pub mod routes; handlers; admin_handlers; service; models; db; ...
   routes.rs       pub fn google_calendar_routes() -> Router<AppState>
                   public routes (slots, book) + admin ones (login, disconnect)
   handlers.rs     axum extractors in, Json out, error -> StatusCode
   admin_handlers.rs  same, every fn takes AdminUser (still /api/google-calendar/login)
   service.rs      rules and orchestration, calls other domains' services
   db.rs           one fn per query, nothing else
   models.rs       #[derive(Serialize, TS)] #[ts(export)] wire types + Sql* rows

 call direction:  routes ─▶ handlers ─▶ service ─▶ db ─▶ Postgres
                                          │
                                          └─▶ other domain's service (never its handlers)

 browser ─▶ frontend/app/api/<domain>/<endpoint>/route.ts ─▶ backend (same origin,
            session cookie forwarded by backend-auth-proxy.ts)
```

A booking, end to end:

```
 frontend (widget)                    backend                              outside
 ────────────                         ───────                              ───────
 step 1 form
   each field ──▶ POST /api/leads/{id}          leads: save one LeadUpdate
   score (widget) ▶ POST /api/leads/{id}        leads: save Qualification
   submit ──────▶ POST /api/leads/{id}/submit   leads: mark submitted
                                                dropoff_task (every minute):
                                                  idle 3 min, not submitted
 step 2 calendar
   month view ──▶ POST /api/google-calendar/    refresh token ◀────────────── Google
                       available-slots-batch    busy times ◀───────────────── Google
                                                slot times (reference zone)
                                                  ─▶ visitor zone, 30 min,
                                                  15 min buffer, 4 h notice
   pick slot ───▶ POST /api/google-calendar/    re-check busy ◀───────────── Google
                       book                     create event + Meet ───────▶ Google
                                                call_bookings row
                                                email_queue rows
 step 3 confirmation ◀── 201 { meet_link }
                                                queue worker (every minute)
                                                  confirmation, reminders ─▶ Resend
 admin dashboard
   /admin/bookings ──▶ GET /api/admin/bookings  bookings/admin_handlers.rs,
                                                AdminUser, bookings/db.rs
```

## Phases

| # | File | Scope | Status |
| --- | --- | --- | --- |
| 0 | `PLAN_0_GENERAL.md` | this overview | living |
| **1** | | **Stage 1: local, solid base** | |
| 1 | `PLAN_1_FOUNDATION.md` | `backend/` + `frontend/` + `migrations/`, local Postgres, `AppState`, `setup_tracing`, lints, `tests/export_bindings.rs` writing to `frontend/src/bindings/`, `lib/backend-origin.ts` + `backend-proxy.ts`, `app/api/health/route.ts`, TanStack Query, shadcn, `/health` seen from the frontend, one `#[sqlx::test]` | 👉 todo |
| 2 | `PLAN_2_AUTH_ADMIN.md` | `users` + roles, Postgres sessions (`axum_session`), GitHub login, current user, logout, `AdminUser` extractor, login page, `(protected)` layout that redirects when logged out, first `Root` user seeded | 👉 todo |
| 3 | `PLAN_3_LEADS.md` | `leads` table, autosave per field (`LeadUpdate`), submit, qualification scored in the widget and saved on the lead (`qualified`, `not_sure`, `disqualified`), dropoff checker (every minute, 3 min idle) | 👉 todo |
| 4 | `PLAN_4_CALENDAR_BOOKING.md` | Google Calendar connected once by the admin (code exchange, refresh token in the single row `google_calendar_config`), slots (fixed slot times in the reference zone, converted to the visitor zone, DST, 30 min, 15 min buffer, 4 h notice, busy times), batch of up to 14 days for the month view, booking: re-check busy, Google event + Meet link, `call_bookings` row, `email_queue` rows, queue worker (every minute) sending with Resend (Giorgi's account, `RESEND_API_KEY`), confirmation + 24h / 2h / 30min reminders (only those still ahead) | 👉 todo |
| 5 | `PLAN_5_BOOKING_WIDGET.md` | widget in `frontend/src/domain/calling/`: step 1 form with autosave, step 2 month calendar + slots + time zone picker, step 3 `/calling/booking-confirmation`, disqualified screen, mobile first; page `/calling`, `inline-booking-widget.tsx` on the local demo page, `floating-booking-widget.tsx` | 👉 todo |
| 6 | `PLAN_6_ADMIN_DASHBOARD.md` | `/admin`: overview (next calls, numbers of the week), bookings list (upcoming, past) with the lead's answers, leads list (booked, dropped off, disqualified), email queue (sent, pending, failed). **Stage 1 done** | 👉 todo |
| **2** | | **Stage 2: online and embeddable** | |
| 7 | `PLAN_7_DEPLOY.md` | `Dockerfile.backend`, `Dockerfile.frontend`, `docker-compose.prod.yml`, `nginx.prod.conf` + HTTPS, env and secrets, Google and GitHub OAuth on the real domain, Resend domain verified, DB backups, `deploy_prod.sh`, CI (`fmt`, `clippy`, `test`, frontend `lint`, `typecheck`) | 👉 todo |
| 8 | `PLAN_8_EMBED.md` | `embed.js`: inline and floating button modes, iframe to `/calling` with auto height, `postMessage` events (`ready`, `booked`), `frame-ancestors` allow list, copy paste snippet; live on Giorgi's site and his personal site. **Stage 2 done** | 👉 todo |
| **3** | | **Stage 3: onboarding and customisation** | |
| 9 | `PLAN_9_MULTI_HOST.md` | sign up open, `Host` role, unique `username`, `host_id` on every host table (backfilled to the first host), Google Calendar connection per host, `/calling/[username]`, every route scoped by `host_id`, tests that host A never sees host B's data, admin sees all hosts | 👉 todo |
| 10 | `PLAN_10_ONBOARDING.md` | wizard: account, time zone, connect Google Calendar, slot times, preview of the widget, embed snippet ready to copy | 👉 todo |
| 11 | `PLAN_11_WIDGET_CUSTOMIZATION.md` | per host settings: call duration, buffer, notice, slot times editor, form questions and qualification rules, branding (colour, logo, texts); the widget reads them | 👉 todo |
| 12 | `PLAN_12_HOST_DASHBOARD.md` | `/dashboard` for each host: their bookings and leads (admin views scoped by `host_id`), cancel a call, notification settings. **Stage 3 done** | 👉 todo |

Status legend: ✅ done, 🔄 in progress, 📅 planned (the `PLAN_N` file is
written), 👉 todo (name reserved, file written when the phase starts),
living (this page).

Why this order:

```
 STAGE 1   1 foundation ─▶ 2 auth + admin ─▶ 3 leads ─▶ 4 calendar + booking
                                                              │
                              6 admin dashboard ◀── 5 widget ◀┘   ★ booked locally,
                                       │                            email received
 STAGE 2                   7 deploy ◀──┘ ─▶ 8 embed                ★ works on other sites
                                                │
 STAGE 3   9 multi host ─▶ 10 onboarding ─▶ 11 customisation ─▶ 12 host dashboard
                                                                   ★ others use it
```

- **Foundation first**: the domain layout, bindings and proxy are copied by
  every later phase. Getting them right once is cheaper than fixing them in
  six places.
- **Auth before leads**: the admin routes that show leads and bookings need
  `AdminUser` from day one, and stage 2 exposes the API publicly.
- **API before widget** (3, 4 before 5): the widget renders what the API
  returns; stable JSON first, otherwise the widget is rewritten twice.
- **Deploy before embed**: an iframe on another site needs a public HTTPS
  URL.
- **Multi host before onboarding and customisation**: settings mean nothing
  until a second host exists, and `host_id` must be on every row before a
  stranger signs up.

### Out of scope (not planned)

Teams and round robin, payments, webhooks, public API keys, recurring
bookings, Outlook / CalDAV / Zoom, SMS and WhatsApp, Discord, third party
analytics, i18n, marketing site. Revisit once stage 3 is live.

## Definition of done per stage

**Stage 1**, on `localhost`:

- a visitor fills the widget, books a slot, and the confirmation email lands
  in a real inbox with a Meet link; the event is in the host's Google
  Calendar; a busy event in that calendar removes the slot;
- the admin logs in with GitHub and sees that booking and that lead in
  `/admin`; a logged out user hitting `/api/admin/*` gets 401;
- `cargo clippy --all-targets -- -D warnings` and `cargo test` green, frontend
  `lint` and `typecheck` green, `frontend/src/bindings/` regenerated and clean.

**Stage 2**: the same, on the public domain, booked from the widget embedded
in Giorgi's site and in his personal site.

**Stage 3**: a stranger signs up, finishes onboarding, customises the widget,
embeds it on their site, gets a booking, and sees it in their dashboard,
without help and without seeing anyone else's data.

## Working agreement

- Each phase: land its `PLAN_N` file first, then implement against it, then
  tick the milestones in that file. Use `__SOP/create-new-plan-file.md` to
  write it and `__SOP/improve-a-plan-before-code.md` to review it.
- Before each commit: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`,
  and in `frontend/`: `lint` and `typecheck`. All green.
- After any query change: `cargo sqlx prepare` and commit `.sqlx/`. After any
  `TS` type change: regenerate `bindings/` and commit it.
- One migration per schema change, never edit a migration that already ran.
- Update this table's Status column as phases move.

## Later, not in scope

- **Desktop and mobile app with Tauri.** The same `frontend/` wrapped in a
  Tauri shell (macOS, Windows, iOS, Android) so a host checks bookings and
  gets notified without a browser. It talks to the same API, so it needs no
  backend change. To decide once stage 3 is live.
