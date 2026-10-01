# cadence, agent guide

- Commit and push straight to `main`, no branches or PRs for normal work.
- If a change belongs to `PLAN_N`, update that plan in the same change.
- Never edit a migration that may already have run. Add the next migration.
- Never edit generated `frontend/src/bindings/` files by hand. Change Rust,
  run the export test, then commit the generated TypeScript.
- Keep migration filenames numeric and descriptive: `0001_XXXX.sql`. The
  development seed is the reserved exception `9999____seed_data.sql`.

## Before writing code

Stop at the first yes:

1. Does the change need to exist at all?
2. Does the behavior already exist in this repository?
3. Is it provided by Rust, TanStack, shadcn, or a listed dependency?
4. Can it stay in the existing domain without a new abstraction?
5. Only then write the smallest code that solves the request.

New dependencies, modules, migrations, or abstractions need a reason in the
plan or commit message.

## Architecture rules

- `backend/` is the Rust axum API. `frontend/` is the TanStack Start/Vite
  application. They communicate through JSON HTTP using `VITE_API_URL`.
- Backend domains live under `backend/src/domain/<domain>/` and keep this
  direction: `routes -> handlers -> service -> db -> Postgres`.
- `handlers.rs` extracts HTTP data and maps errors. Business rules live in
  `service.rs`. SQL lives in `db.rs`. Wire models derive `TS`.
- Admin endpoints stay owned by their domain and use `AdminUser`. Admin UI is
  `/admin`, never `/dashboard/admin`.
- Frontend routes live in `frontend/src/routes/`. Shared domain components live
  in `frontend/src/domains/<domain>/`. Server data uses TanStack Query.
- Use shadcn components and Lucide icons before custom UI primitives. Use
  shared shadcn tokens, never legacy `.card`, `.button`, or `.input` CSS.
- Do not add magic role/status/route strings when a typed constant, enum, or
  central route map can own the value.
- Booking email delivery goes through `email_queue` and the email worker.
- Store instants as `TIMESTAMPTZ`. Keep timezone names as IANA strings.

## Visual changes

When explaining or planning a visual change, include an ASCII sketch:

```
┌──────────────┬─────────────────────────────┐
│ sidebar      │ header / page content       │
│ nav          │ cards, tables, empty states  │
└──────────────┴─────────────────────────────┘
```

Match the existing Rustify-inspired shadcn layout. Do not invent a second
design system.

## Checks before commit

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
pnpm --dir frontend build
pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend knip
```

After schema changes, run migration tests or the local database reset. After a
Rust wire type change, run `cargo test --test export_bindings` and verify
`frontend/src/bindings/` is clean. Use `request.http` for endpoint smoke tests.

## Plans

Each phase has one `docs/PLAN_N_*.md`. Plan files state the user-visible goal,
gap, data model, API, flow, implementation sketch, tests, milestones, and
definition of done. Keep Plan 0 short and move detail into phase files.
