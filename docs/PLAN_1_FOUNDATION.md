# Plan 1: foundation

Status: done.

## Goal

A contributor can clone Cadence, start backend and frontend together, run
Postgres migrations, inspect the API through `request.http`, and trust strict
format, lint, type, and dead-code checks.

## The gap this fixes

The foundation now exists in `Cargo.toml`, `backend/Cargo.toml`,
`backend/src/main.rs`, `frontend/package.json`, and `scripts/dev-preflight.sh`.
The stable boundary is Rust JSON API, generated TypeScript contracts, and a
TanStack frontend.

## Approach

- `backend/src/main.rs` loads `.env`, creates the SQLx pool, calls tracing,
  runs `sqlx::migrate!("../migrations")`, starts the email worker, and serves
  axum.
- `backend/src/domain/` uses one folder per domain and the DDD flow
  `routes -> handlers -> service -> db`.
- `backend/tests/export_bindings.rs` owns ts-rs exports.
- `frontend/` uses TanStack Start/Vite, TanStack Router, TanStack Query,
  shadcn UI, and strict Biome/Knip checks.
- Root `pnpm run dev` uses `concurrently` and `wait-on` to run both apps.

## Data model

Foundation schema lives in `migrations/0001_enable_extensions.sql` through
`0014_create_rate_limits.sql`. Each file owns one table or extension concern.
Development users and grants live in `9999____seed_data.sql`.

## API

| Method | Path | Guard | Result |
| --- | --- | --- | --- |
| GET | `/health` | public | `200 OK` |
| GET | `/api/auth/me` | logged in | current session user |

The complete domain API is listed in Plans 2 and 3.

## Flow schema

```
pnpm run dev
  ├─ scripts/dev-preflight.sh
  ├─ backend cargo watch ─▶ migrations ─▶ API :3001
  └─ wait-on :3001 ───────▶ frontend Vite :3000
```

## Impl sketch

- `AppState` owns `PgPool`.
- `build_app` merges public and auth-protected domain routers.
- `tracing_setup.rs` installs the subscriber once.
- `vite.config.ts` maps `@` and `~` to `frontend/src` for shadcn imports.

## Out of scope

- Business booking rules, phase 3.
- Google Calendar and real provider delivery, phase 4.
- Production deployment, phase 6.

## Tests

- `cargo fmt --all -- --check`.
- Strict workspace Clippy with `-D warnings`.
- `cargo test --test export_bindings`.
- Frontend build, typecheck, Biome, and Knip.
- `GET /health` in `request.http`.

## Milestones

- F0: workspace and strict lints, done.
- F1: migrations and `sqlx::migrate!`, done.
- F2: tracing, dev preflight, parallel dev command, done.
- F3: ts-rs bindings, shadcn, TanStack Query, Biome, Knip, done.
- F4: quality checks green, done.

## Definition of done

- Backend and frontend start from one root command.
- Migrations run from backend boot.
- Generated bindings compile.
- No legacy global component CSS remains.
- All foundation checks pass.

## After phase 1

Phase 2 adds authentication, role access, and protected dashboard shells.
