# Cadence

Scheduling and call-booking app. Backend Rust/Axum. Frontend TanStack/Vite. No SEO requirement.

## Stack

- Rust, Axum, Tokio, SQLx, Postgres
- DDD backend split into `backend/src/domain/*`
- TanStack Router, TanStack Query, Vite, React, shadcn/ui
- Cookie auth with JWT session token
- `ts-rs` bindings in `frontend/src/bindings`
- Resend SMTP worker backed by `email_queue`

## Local setup

```sh
cp .env.example .env
pnpm install
./reset_db.sh --yes
pnpm run dev
```

`pnpm run dev` starts backend and frontend in parallel. Frontend waits for backend port `3001`.

- Frontend: `http://localhost:3000`
- Backend: `http://localhost:3001`
- Health: `http://localhost:3001/health`

`scripts/dev-preflight.sh` clears stale listeners on ports `3000` and `3001` before startup.

## Resend

Use token-based config. `RESEND_API_KEY` is the Resend token used as SMTP credential, never a plaintext password variable.

```dotenv
EMAIL_FROM=example@email.com
RESEND_API_KEY=re_xxx
RESEND_SMTP_HOST=smtp.resend.com
RESEND_SMTP_PORT=587
RESEND_SMTP_USERNAME=resend
```

Backend falls back to local unauthenticated SMTP on `localhost:1025` when Resend host is not configured.

## Quality

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
pnpm run check:quality
```

## Useful files

- `request.http`: API requests
- `migrations/`: one SQL table or extension per migration, numbered `0001_XXXX`
- `ARCHITECTURE_DECISIONS.md`: only deliberate divergences from reference projects
- `reset_db.sh`: local database reset and migration runner
