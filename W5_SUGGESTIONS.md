# Code Review: Suggestions

Review of `cadence` at `main` (backend axum API, TanStack Start frontend, Postgres, one Hetzner box behind nginx). Focus: what can lose a booking, double book a host, leak a credential, or silently lose an email. Suggestions only; no application code changed.

The backend is already in good shape: rate limits are in Postgres, the slot reservation uses an advisory lock, reset tokens are stored hashed, login takes the same time for unknown emails, email sending is queued with `SKIP LOCKED`. The items below are the gaps that remain, ordered by importance.

Severity: **HIGH** means a realistic path loses data, double books a host, or ships a regression unnoticed. **MEDIUM** means a realistic failure leaves a wrong or stuck state, or a credential is weaker than it should be. **LOW** means a confusing edge case, a missing safeguard, or cleanup.

## Summary

### Backend and project

| Id | Severity | Finding |
|----|----------|---------|
| S1 | HIGH | The 55 integration tests never run in CI or at deploy |
| S2 | HIGH | A failing Google Calendar read is treated as "no busy time", so the host can be double booked |
| S3 | MEDIUM | A failed calendar delete leaves an event that blocks the cancelled slot forever |
| S4 | MEDIUM | Google OAuth tokens are stored in plain text, and so are in every backup |
| S5 | MEDIUM | Email retries are flat, then a failed email is dropped silently after 20 minutes |
| S6 | MEDIUM | The calendar id is a global env var; the per-host `calendar_id` column is ignored |
| S7 | MEDIUM | Every public slots request calls Google; no cache, no refresh lock, no revoked-grant handling |
| S8 | MEDIUM | Backups live only on the production server and nothing alerts when one fails |
| S9 | MEDIUM | A booking whose calendar event fails to be created is never retried |
| S10 | LOW | Logout does not revoke the session; no "sign out everywhere" |
| S11 | LOW | `register` tells the caller which emails already have an account; no email verification |
| S12 | LOW | Client IP headers are trusted unconditionally |
| S13 | LOW | The backend container runs as root |
| S14 | LOW | Small cleanups (hash stability, duplicated truncation, env aliases, unit struct) |
| S15 | LOW | No git hooks: failures only show up in CI after the push |
| S16 | LOW | Hand written `as_str` / `Display` / `FromStr` on enums, and a mixed DB layer style |
| S17 | MEDIUM | Google is called directly from the services, so its failure paths cannot be tested |
| S18 | LOW | Environment variables are read in about 12 places, with no single validated config |
| S19 | LOW | Slot rules are only tested through Postgres |
| S20 | LOW | No metrics or alert on the email queue or on Google failures |
| S21 | MEDIUM | No request timeout, so a slow Google call can hold a connection and exhaust the pool |
| S22 | LOW | `bookings::db::create` and the Google client take too many arguments |
| S23 | LOW | `emails/service.rs` is 1270 lines (rendering, sending and tests together) |
| S28 | LOW | Email HTML is built inside Rust `format!` strings instead of template files |

### Frontend

| Id | Severity | Finding |
|----|----------|---------|
| S24 | LOW | Several files call `fetch` directly instead of going through `lib/api.ts` |
| S25 | LOW | Query keys are flat, so one mutation invalidates several keys by hand |
| S26 | LOW | No default `staleTime`, so every mount refetches |
| S27 | LOW | The same host settings mutation is written twice |

---

## HIGH severity

### S1. The 55 integration tests never run in CI or at deploy (HIGH: regressions ship untested)

**Files:** `.github/workflows/quality.yml:39`, `deploy_prod.sh:30`, `backend/tests/auth_http.rs`, `backend/tests/host_booking_http.rs`

`backend/tests/` holds 55 `#[sqlx::test]` tests: login, password reset, session versions, double booking, reschedule, cancel, email queue, host isolation. They are the only coverage of the rules that matter most. But both the CI job and the deploy script only run:

```yaml
- run: cargo test --workspace --test export_bindings
```

No `cargo test` of the rest, and the `quality` job has no Postgres service, so `#[sqlx::test]` could not run even if it were asked to. The unit tests inside `backend/src` (11 modules) are skipped too. `deploy.yml` calls `quality.yml`, so a deploy goes out green with the whole safety net switched off.

**After:** add a Postgres service to the job and run everything.

```yaml
    services:
      postgres:
        image: postgres:15-alpine
        env:
          POSTGRES_PASSWORD: postgres
        ports: ["5432:5432"]
        options: >-
          --health-cmd "pg_isready -U postgres" --health-interval 5s
          --health-timeout 5s --health-retries 10
    env:
      DATABASE_URL: postgres://postgres:postgres@localhost:5432/postgres
      SQLX_OFFLINE: "true"
      JWT_SECRET: ci-only-secret
    steps:
      # ...
      - run: cargo test --workspace
```

Keep `export_bindings` as part of that run (it is a test of the same workspace), and add `cargo test --workspace` to `deploy_prod.sh` next to clippy, so the local gate matches CI. Use the same `postgres:15` major version as `docker-compose.prod.yml`.

Run the tests locally first: if some have drifted (they have not run in CI, so they may), fix them in the same change.

### S2. A failing calendar read is treated as "no busy time" (HIGH: double booking)

**Files:** `backend/src/domain/bookings/slots.rs:59-66`, used by `open_slots` and so by `reserve`

```rust
// A broken calendar connection must not take the booking page down.
let busy = calendar
    .busy(page.host_unid, &page.timezone, from, to)
    .await
    .unwrap_or_else(|error| {
        tracing::error!(?error, "calendar busy times were not loaded");
        Vec::new()
    });
```

The comment states the intent and it is a fair trade for the public page. The problem is that the same function feeds `reserve`, which is the write path. When the host revoked access, Google answered 5xx, or the 10 s timeout hit, `busy` is empty, every slot over a real meeting looks free, and a visitor can book it. The booking is confirmed, the invitee gets an email, and the host finds a clash on their calendar. Nothing tells the host the calendar stopped being read: the only trace is one `error!` line (which `BugReportLayer` does turn into a server bug report, but no one is told the host is affected).

**Fix:** split the two paths. Reading the page may degrade; taking a slot must not.

```rust
pub enum CalendarMode {
    /// Public page: a broken calendar shows the hours it can.
    BestEffort,
    /// A booking or a move: a broken calendar refuses, the visitor retries.
    Strict,
}
```

`reserve` calls `open_slots(.., CalendarMode::Strict)` and maps `CalendarError` to a retryable `503` ("this time cannot be confirmed right now, try again in a minute"). Also record `calendar_error_at` on `google_calendar_config` when a read fails with an auth error, and show a banner in the host's settings ("Google Calendar needs to be reconnected"). A test with `InMemoryCalendar` set to fail covers both modes.

---

## MEDIUM severity

### S3. A failed calendar delete leaves an event that blocks the slot forever (MEDIUM: slot lost)

**Files:** `backend/src/domain/bookings/slots.rs:173-176`, `backend/src/domain/bookings/service.rs` (`cancel_call`)

`cancel_call` deletes the Google event after the commit and only logs a warning when it fails ("a stale calendar event must not undo that"). That is right for the cancel itself. The consequence is not handled: `open_times` ignores the calendar event of a call as "the call itself" only for calls in `booked`, and `booked_between` excludes cancelled calls. So a cancelled call whose event is still on Google is now an ordinary busy event, and the slot it held stays closed, with no way for the host to see why.

**Fix:** keep the delete in the queue, not in the request. Same pattern as emails (`AGENTS.md`: delivery goes through a queue and a worker):

```sql
-- migrations/0017_create_calendar_jobs.sql
CREATE TABLE calendar_jobs (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    booking_unid  UUID NOT NULL REFERENCES bookings(unid) ON DELETE CASCADE,
    action        TEXT NOT NULL CHECK (action IN ('create', 'move', 'delete')),
    attempts      INT  NOT NULL DEFAULT 0,
    run_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    done_at       TIMESTAMPTZ,
    last_error    TEXT
);
```

The cancel transaction inserts a `delete` job with the event id; the worker retries with backoff (see S5). This also covers S9. As a cheap interim fix, treat the event ids of cancelled calls as own events too: select `google_event_id` of cancelled calls in the window and add them to `own_events`.

### S4. Google OAuth tokens are stored in plain text (MEDIUM: credential at rest)

**Files:** `migrations/0011_create_google_calendar_config.sql`, `backend/src/domain/google_calendar/db.rs`

`access_token` and `refresh_token` are plain `TEXT`. A refresh token does not expire: with it, anyone can read and write the host's whole primary calendar until the host revokes it. The tokens end up in every `pg_dump` (`scripts/backup-db.sh`), the dump is pulled to a laptop by `scripts/pull-backup.sh`, and the dump is a custom-format file that is trivial to read.

**Fix:** encrypt the two columns in the application with an AEAD key held only in the backend environment (`TOKEN_ENCRYPTION_KEY`), not in the database, so a leaked dump is not enough.

```rust
// aes-gcm, a fresh 96-bit nonce per value, stored as nonce || ciphertext (base64).
pub fn seal(key: &Key<Aes256Gcm>, plain: &str) -> AppResult<String> { /* ... */ }
pub fn open(key: &Key<Aes256Gcm>, sealed: &str) -> AppResult<String> { /* ... */ }
```

Add the new column values in a migration that does not edit `0011` (`AGENTS.md`): `0018_seal_google_tokens.sql` adds `*_sealed` columns, a one-off function re-encrypts existing rows, a later migration drops the plain ones. This needs a reason in the commit message (new dependency), which is the one given here.

### S5. Email retries are flat, then the email is dropped silently (MEDIUM: lost confirmations)

**Files:** `backend/src/domain/emails/db.rs:98-112`, `backend/src/domain/emails/worker.rs:79-98`

`mark_failed` always sets `send_at = now() + 1 minute`, and `claim_next` stops picking a row once `attempts >= 20`. So an SMTP provider outage of 20 minutes (a Resend incident is enough) uses up all attempts of every email queued in that window: the booking confirmation, the cancel notice, the password reset. After that the row sits forever with `attempts = 20`; the admin list shows it but nothing notifies anyone, and no one resends it.

A second effect: `process_next` holds one transaction, so one DB connection, for the whole SMTP send (up to the 20 s `SMTP_TIMEOUT`). The pool has 5 connections (`main.rs:22`). A slow mail server can hold the worker's connection for 20 s per email, 50 emails per pass.

**Fix:**

```rust
/// Backoff grows with each failure: 1, 2, 4, ... minutes, capped at 6 hours.
fn retry_delay(attempts: i32) -> Duration {
    Duration::minutes(2_i64.pow(attempts.clamp(0, 9) as u32).min(360))
}
```

With 12 attempts this covers about a day. When a row reaches the limit, log it at `error!` (it becomes a server bug report through `BugReportLayer`, which is exactly the alert you want) and show it in the admin emails page as "gave up" with a "retry" button. To free the connection during the send, claim the row and mark it `sending_until = now() + interval '2 minutes'` in a short transaction, send outside any transaction, then record the result in a second one; `claim_next` skips rows whose `sending_until` is in the future.

### S6. The calendar id is a global env var; the per-host column is ignored (MEDIUM: wrong calendar for every host but one)

**Files:** `backend/src/domain/google_calendar/client.rs:56-59`, `migrations/0011_create_google_calendar_config.sql`

```rust
// Use GOOGLE_CALENDAR_ID env var if set (required for Service Account),
// otherwise default to "primary" (works for OAuth)
let calendar_id =
    std::env::var("GOOGLE_CALENDAR_ID").unwrap_or_else(|_| "primary".to_owned());
```

The table already has `calendar_id TEXT NOT NULL DEFAULT 'primary'` per host, and `GoogleCalendarConfig` loads it, but `GoogleCalendarClient::new` never sees it. If `GOOGLE_CALENDAR_ID` is ever set in production (it appears in the code comment as a Service Account setting, a leftover of the old app), every host reads and writes that one calendar. With per-host OAuth (`PLAN_7_MULTI_HOST`), the service-account case does not exist any more.

**Fix:** pass the stored id and delete the env read.

```rust
pub fn new(access_token: String, calendar_id: String) -> Self { /* ... */ }
```

`access_token()` in `service.rs` returns the whole `GoogleCalendarConfig` (or a small `CalendarAccess { token, calendar_id }`) instead of only the token string.

### S7. Every public slots request calls Google (MEDIUM: latency, quota, repeated failures)

**Files:** `backend/src/domain/bookings/handlers.rs` (`public_slots`), `backend/src/domain/google_calendar/service.rs` (`access_token`)

Three related gaps on the busiest public endpoint (up to 120 requests per 10 minutes per address):

1. Each request does a Google events request (several pages for a busy calendar, up to `EVENT_PAGES_MAX = 20`, 10 s timeout each). A page refresh or a visitor flipping weeks keeps hitting Google. A host with a popular link can hit the per-user quota, and then S2 makes the page show everything as free.
2. Two requests with an expired token both refresh it at once. Google accepts that, but each overwrites the row with its own token, and a refresh that returns a rotated refresh token (not stored here, only `access_token` is updated) would race.
3. A revoked grant (`invalid_grant`) is logged as `error!` and returned as a plain error, again on every request, forever. Each of these becomes a server bug report, so one disconnected host fills the bug report table (the layer drops reports past 64 queued, but the writer still inserts one row per failure).

**Fix:** a short in-memory cache of busy events keyed by `(host_unid, from, to)` with a 30 to 60 s TTL (`moka` or a `Mutex<HashMap>`), invalidated when Cadence itself creates, moves or deletes an event. For the refresh, `SELECT ... FOR UPDATE` the config row (or an advisory lock on the host) inside `access_token` so one request refreshes and the others read the result. On `invalid_grant`, mark the config as needing reconnection (the S2 column) and skip Google for that host until it is reconnected, logging once.

### S8. Backups live only on the production server (MEDIUM: one disk failure loses everything)

**Files:** `scripts/backup-db.sh`, `scripts/pull-backup.sh`, `DEPLOYMENT.md` "Backups"

The dump goes to `/root/cadence-backups/` on the same server as the database volume and is pruned after 14 days. `pull-backup.sh` copies the newest one to a laptop by hand. If the server or its disk is lost, or the account is compromised, the data and every backup go together. The cron line has no failure alert, and the restore check (`pg_restore --list`) only proves the file is readable, not that it restores.

**Fix, in order of effort:**

1. Push each dump off the box right after it is written (`rclone`/`restic` to an S3 compatible bucket, or Hetzner Storage Box over `scp`), with the dump encrypted (`age`) because of S4.
2. Make the cron job fail loudly: `curl -fsS https://hc-ping.com/<uuid>` on success only (Healthchecks.io), so a missed night sends an email.
3. Once a month, restore the newest dump into a throwaway container and run `SELECT count(*) FROM bookings` in `backup-db.sh --verify`. The restore command is already in `DEPLOYMENT.md`; script it.

### S9. A booking whose calendar event fails to be created is never retried (MEDIUM: missing invite)

**Files:** `backend/src/domain/bookings/service.rs` (`create`)

```rust
// A booking without a calendar event is still a booking.
let event = calendar.create_event(...).await
    .inspect_err(|error| tracing::error!(?error, "calendar event was not created"))
    .ok()
    .flatten();
```

The booking is kept, which is the right call. But the event is gone for good: the host's calendar never shows it, the invitee never gets the Google invite, and with `ConferencingApp::GoogleMeet` the confirmation email goes out with no meeting link at all (`meeting_link` stays `None` unless the host set one). Nobody sees this until the call time.

**Fix:** the `calendar_jobs` queue of S3, with a `create` job inserted in the booking transaction. The confirmation email for a Meet booking should be queued after the job succeeds (or carry a "link coming" state), so it never says "join here" with nothing behind it.

---

## LOW severity

### S10. Logout does not revoke the session (LOW: stolen cookie lives 30 days)

**Files:** `backend/src/domain/auth/handlers.rs` (`logout`), `backend/src/domain/auth/service.rs` (`make_token`)

`logout` only expires the cookie in the browser. The JWT stays valid for 30 days and is also accepted as a `Authorization: Bearer` header. The only revocation is a password change (`session_version`). A token copied from a shared machine or an old browser keeps working after the user "logs out".

**Fix:** `logout` bumps `session_version` for a "sign out everywhere" action (`POST /auth/logout-all`), and a plain `logout` keeps working per device. For per-device logout, store a `sessions` row id in the token (`jti`) and delete it on logout. Also drop the Bearer path if nothing uses it (`request.http` is the only likely consumer): fewer ways in.

### S11. `register` reveals which emails have an account (LOW: enumeration)

**Files:** `backend/src/domain/auth/service.rs` (`register`, `EMAIL_TAKEN`)

Login and forgot-password are careful to answer the same either way, but register answers `409 email already registered`. The 5 per hour per address limit slows it down without stopping it. There is also no email verification, so anyone can create a host account with someone else's address and it gets a booking page and an email queue.

**Fix:** answer `201` for both, and send the real owner an "someone tried to register with your address" email when the account exists; for a new one, send a verification link and keep `status` as `pending` until it is used (the `UserStatus` enum is already there). If open registration is not a product goal at all (this is a coaching tool for a few hosts), put it behind an invite code or an admin-created account and remove the public route.

### S12. Client IP headers are trusted unconditionally (LOW: rate limits bypassable if nginx is bypassed)

**Files:** `backend/src/tracking.rs:5-15`

`extract_client_ip` reads `cf-connecting-ip`, then `x-forwarded-for`, then `x-real-ip`, from whatever the client sent. Today `nginx.prod.conf` overwrites `X-Forwarded-For`, `X-Real-IP` and clears `CF-Connecting-IP`, and the compose file binds the port to `127.0.0.1`, so this is safe in production. It stops being safe if Cloudflare is added in front (the nginx comment says it is not) or the port is ever exposed: every rate limit (login, reset, booking) then depends on a header an attacker picks per request.

**Fix:** name the one trusted header in config and read only it.

```rust
// TRUSTED_IP_HEADER=x-real-ip in prod; cf-connecting-ip behind Cloudflare; empty = peer address
```

In the empty case use axum's `ConnectInfo<SocketAddr>` (needs `into_make_service_with_connect_info` in `main.rs`). A test in `tracking.rs` that a forged `X-Forwarded-For` is ignored when `x-real-ip` is the configured one locks the behaviour in.

### S13. The backend container runs as root (LOW: larger blast radius)

**Files:** `Dockerfile.backend`

The runtime stage has no `USER`, so the process runs as root in the container. The binary only needs to read env and talk to the network.

**Fix:**

```dockerfile
RUN useradd --system --no-create-home --shell /usr/sbin/nologin cadence
USER cadence
```

Same for the frontend stage (`node:22-alpine` ships a `node` user: `USER node`).

### S14. Small cleanups (LOW)

- **Bug report similarity hash** (`bug_reports/service.rs`): `DefaultHasher` is documented as not stable across Rust releases, so a toolchain bump (`rust-toolchain.toml` is pinned, but it does move) regroups all existing reports. Hash a stable digest of the message instead (for example SHA-256 of the message).
- **Duplicated truncation** (`bug_reports/service.rs`): `exception_message()` and the inline block in `add` do the same 512-byte cut, and `NewBugReport::bounded` does it a third time. Use `bounded` for all three.
- **Google env aliases** (`google_calendar/service.rs`): `GOOGLE_CLIENT_ID`/`CLIENT_ID_GOOGLE`, three redirect URI names and four secret names, plus a `required_env` that only calls `first_env`. Keep one name each, document it in `.env.example`, and delete `required_env` and `Environment::detect()` branching on the secret.
- **`BookingsService` unit struct** (`bookings/service.rs`): it exists for one method (`list_all`) while everything else in the module is a free function. Make `list_all` a free function like its neighbours and delete the struct (same for `BookingsDb`, `LeadsService`, `GoogleCalendarDb`).
- **`calling_visit_unid` is not checked against the host** (`bookings/service.rs`): `lead_unid` is checked with `leads::get(pool, host_unid, ..)`, but the visit only has the FK, so a request can attach a visit id from another host's page. Low impact (visits are analytics) but a one-line check keeps host isolation uniform.

---

### S15. No git hooks (LOW: slow feedback)

**Where:** no `.githooks/`, no `core.hooksPath`. RUSTIFY-APP has both.

**Problem:** `AGENTS.md` lists the checks before commit, but nothing runs them.
A fmt or clippy failure is found by CI after the push to `main`.

**Fix:** commit `.githooks/` and a `scripts/setup-hooks.sh` that runs
`git config core.hooksPath .githooks`.

- `pre-commit` (fast): `cargo fmt --all -- --check` when `*.rs` is staged,
  `pnpm --dir frontend lint` when `frontend/` is staged, and a check that
  nobody staged a hand edit of `frontend/src/bindings/` or a changed old migration.
- `pre-push` (slow): clippy `-D warnings`, `export_bindings` plus a clean
  bindings check, `pnpm typecheck` and `test`, each only if the matching paths
  changed against `@{u}`. Add `cargo test` once S1 is done.
- Keep hooks and `quality.yml` calling the same scripts so they cannot drift.

### S16. Enum string boilerplate and a mixed DB layer style (LOW: consistency)

**Where:** `auth/models.rs` (`Role`, `UserStatus`), `bookings/models.rs`,
`host_settings/models.rs`, `bug_reports/models.rs` (`as_str`, `Display`,
`FromStr`, the `text_column!` macro). DB layer: free `pub async fn` in
`availability/db.rs` and most of `bookings/db.rs`, but `GoogleCalendarDb` and
`CallingVisitsDb` use `impl`.

**Fix 1:** add `strum` (`Display`, `EnumString`, `IntoStaticStr`) and delete the
hand written `as_str`, `Display` and `FromStr`. One derive per enum, so a new
variant cannot be forgotten in one of the three matches. `serde` and `TS`
keep owning the wire format. Reason for the plan: AGENTS.md asks for one.

**Fix 1b:** delete `backend/src/text_column.rs` and the three `text_column!`
calls (`BookingStatus`, `UserStatus`, `ConferencingApp`). It re-implements
sqlx `Type`, `Encode` and `Decode` by hand, and makes every new variant be
listed in three places (the enum, `as_str`, the macro). RUSTIFY-APP already does
the same job with a derive and no macro (`deeplinks/models.rs:9`,
`jobs/models.rs:8`):

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, sqlx::Type, TS)]
#[sqlx(type_name = "text")]
pub enum BookingStatus { Pending, Confirmed, Cancelled }
```

RUSTIFY uses `type_name = "varchar"` there; Cadence's columns are `TEXT` with a
`CHECK`, so check that `"text"` encodes, decodes and passes `query_as!` on one
enum first (`UserStatus`), then run the 55 integration tests (S1) before
converting the other two. Hand written `Role` (`Display`, `FromStr`) goes the
same way. The unknown-value error then comes from sqlx instead of the macro.

**Fix 1c:** remove every hand written string conversion on enums, with `strum`
derives. Inventory (all in `backend/src/domain/`):

| Enum | Hand written today | Replace with |
|------|--------------------|--------------|
| `Role` (`auth/models.rs:23`, `:34`) | `impl fmt::Display` (a `match` returning the variant name), `impl FromStr` (`Err = ()`) | `#[derive(Display, EnumString)]` |
| `UserStatus` (`auth/models.rs:57`) | `as_str` | `#[derive(Display, IntoStaticStr)]`, call sites use `.to_string()` or `<&'static str>::from(x)` |
| `BookingStatus` (`bookings/models.rs:38`) | `as_str` | same |
| `ConferencingApp` (`host_settings/models.rs:23`) | `as_str` | same |
| `AdminBugType` (`bug_reports/models.rs:28`) | `as_str`, and a comment saying "No strum dependency" | same, and delete that comment |

Details to keep:

- `Role::is_elevated` stays: it is behaviour, not conversion.
- The variant names are the stored and wire values, so the default strum
  casing (the variant name as is) matches today's output. Add
  `#[strum(serialize_all = "...")]` only if a value ever differs, and check it
  against the `CHECK` constraints and the generated `frontend/src/bindings/`.
- `Role`'s `FromStr` returned `Err = ()`. `EnumString` returns
  `strum::ParseError`; check the callers that map the error.
- No change to the wire format, so the bindings should come out identical.
  Run `cargo test --test export_bindings` and confirm `frontend/src/bindings/`
  is clean.
- Net effect: about 75 lines of `match` boilerplate removed, and a new variant
  is written once.

**Fix 2:** one DB style, the common sqlx one: free functions in `db.rs`
(a module already is a namespace), with the entity in the name so the call
reads alone. Today `db::create(...)` appears in `bookings`, `calling_visits`
and `availability`, and the bare `create` says nothing at the call site, in
logs or in diffs. Rename to `create_calling_visit`, `create_availability_rule`,
`create_booking`, `list_availability_rules`, `find_booking`, `cancel_booking`
(`auth` already does this: `create_host`, `create_reset`). Then delete the
`CallingVisitsDb`, `BookingsDb`, `LeadsDb` and `BookingsService` shells (move
their `list_all` to a free function), and turn `GoogleCalendarDb::create_state`
into a free function too. One mechanical
commit per domain.

Not chosen: `struct XDb; impl XDb` with no state. It gives context at the call
site (`CallingVisitsDb::create`) but a team would ask why a type holds nothing.
It stays a valid choice if consistency with RUSTIFY-APP matters more. A struct
holding the `PgPool` with `&self` methods only pays off if the DB must be
mocked behind a trait, which S19 avoids with pure functions.

### S17. Google is called directly from the services (MEDIUM: failure paths untestable)

**Where:** `bookings/slots.rs`, `google_calendar/service.rs`, `client.rs`.

**Problem:** S2, S3 and S9 are all "Google is down or slow" bugs, and no test
can reproduce them without the network.

**Fix:** one trait, `CalendarProvider` (`busy`, `create_event`, `delete_event`),
with the Google client as the real implementation and a fake in tests. Do it
together with S2/S3/S9, since they touch the same seam. This is the only port
worth adding: Postgres and SMTP have one implementation each, so a full
hexagonal layout would only add indirection and would lose the compile time
checked `query_as!`.

### S18. No single config (LOW: scattered, unvalidated env reads)

**Where:** `std::env` is read in `main.rs`, `auth`, `emails` (service and
worker), `bug_reports`, and `google_calendar` (see also the aliases in S14).

**Fix:** one `Config` struct built once in `main.rs`, validated at startup
(fail fast with a clear message), and passed through `AppState`. Services stop
calling `env::var` themselves, which also makes them testable.

### S19. Slot rules are only tested through Postgres (LOW: slow tests)

**Where:** `backend/tests/host_booking_http.rs` covers slot logic with
`#[sqlx::test]`.

**Fix:** keep the slot computation (rules, busy intervals, timezone, buffers)
as pure functions and unit test them in `#[cfg(test)]`, no database. Keep the
`sqlx::test` cases for the SQL and the locking only.

### S20. No metrics or alert on the queue and on Google (LOW: failures are silent)

**Where:** `emails/worker.rs`, `google_calendar/`.

**Fix:** log at `error!` when an email is given up on (S5) and when Google
fails (S2), then alert on those from a log or uptime tool. Add the oldest
pending `email_queue` age to the health check. Pairs with S8 (backup alert).

### S21. No request timeout (MEDIUM: one slow dependency stalls the API)

**Where:** `main.rs` and `lib.rs` have `DefaultBodyLimit`, `TraceLayer` and a
graceful shutdown, but no `TimeoutLayer`.

**Problem:** with S2 and S7, a slow Google call keeps the request open, and
each open request can hold one of the 5 pool connections. A few slow calls
make unrelated endpoints wait.

**Fix:** a global `tower_http::timeout::TimeoutLayer` (about 10 seconds) and an
explicit timeout on the Google HTTP client. Return `503` on timeout so the
frontend can show a retry message.

### S22. Functions with too many arguments (LOW: hard to read at the call site)

**Where:** `bookings/db.rs:24` (`create`) and `google_calendar/client.rs:159`,
both with `#[allow(clippy::too_many_arguments)]`.

**Fix:** pass a struct (`NewBooking`, `NewCalendarEvent`) instead of 8 or more
positional values. The call then names every field, and the `allow` goes away.
It fits the rename in S16.

### S23. `emails/service.rs` is too big (LOW: hard to navigate)

**Where:** `emails/service.rs` has 1270 lines; `google_calendar/calendar.rs`
has 523.

**Fix:** move template rendering to `emails/templates.rs` and the tests to a
`tests.rs` file, so `service.rs` keeps the sending rules only. Split
`calendar.rs` the same way if it keeps growing. Purely mechanical, one commit.

## Frontend

TanStack Query is used well: one `QUERY_KEYS` factory in `lib/query-keys.ts`,
no string keys in components, `queryOptions` per domain, and a correct
optimistic update with rollback in `availability/use-hours.ts`. The
`QueryClient` is built in `router.tsx` with a global `onError` and a retry
policy that does not retry 4xx. The items below are refinements.

### S24. Frontend calls `fetch` directly in several places (LOW: inconsistent errors)

**Where:** `domains/widget/cadence-api.ts`, `slots-frame.tsx`,
`bookings/manage-booking-page.tsx`, `bookings/booking-detail-sheet.tsx`,
`admin/components/admin-data-page.tsx`, `dashboard/booking-page-status.tsx`,
`lib/bug-report.ts`, `routes/admin/index.tsx`, next to `lib/api.ts`.

**Fix:** route them through the one client (base URL from `VITE_API_URL`,
credentials, typed errors, the 503 from S21). Check the widget first: it runs
inside an iframe and may need its own client for a good reason. Count based on
a grep for `fetch(`; each call was not read.

### S25. Flat query keys (LOW: manual invalidation)

**Where:** `lib/query-keys.ts`, `bookings/booking-detail-sheet.tsx:58-60`.

**Problem:** `["bookings"]` and `["booking-detail", unid]` share no prefix, so
one booking action needs two `invalidateQueries` calls. The same holds for
`availability` and `availability-overrides`, and for `booking-page` and
`public-slots`.

**Fix:** make the keys hierarchical: `["bookings", "list"]` and
`["bookings", "detail", unid]`, so `invalidateQueries({ queryKey: ["bookings"] })`
covers both. Keep the factory as the only place that builds a key. Drop the
local `KEY` alias in `use-hours.ts` while there.

### S26. No default `staleTime` (LOW: extra requests)

**Where:** `router.tsx:27-37`: `defaultOptions.queries` only sets `retry`.

**Problem:** with the default `staleTime: 0`, each mount of a dashboard route
refetches bookings, settings and availability. For data that changes rarely,
that is wasted calls and flicker. The slots query (S7) is the opposite case:
it needs a short, deliberate `staleTime`.

**Fix:** set a small default (about 30 seconds) and override per query
(`publicSlots` shorter, `hostSettings` longer). Check `routes/__root.tsx:34`,
which already sets `staleTime: Number.POSITIVE_INFINITY` for something, to see
whether it should follow the same rule.

### S27. Host settings mutation written twice (LOW: drift)

**Where:** `routes/dashboard/settings.tsx:107-109` and
`integrations/conferencing-form.tsx:72-73` both call `setQueryData` on
`hostSettings`, then invalidate `bookingPage(slug)`.

**Fix:** one `useUpdateHostSettings()` hook in `dashboard/host-api.ts` that owns
the mutation, the cache update and the invalidations. Both screens call it, so
a new dependent key cannot be forgotten in one of them.

### S28. Email HTML lives in Rust strings (LOW: hard to edit and preview)

**Where:** `backend/src/domain/emails/service.rs` (about 15 lines of inline
`<table>`, `<td>` and `<div>` markup, for example near line 740, plus the
`render` function at line 471).

**Problem:** changing a colour, a padding or a sentence means editing Rust and
recompiling. The HTML cannot be opened in a browser as is. It is also one of
the reasons `service.rs` reaches 1270 lines (S23).

**Fix:** follow RUSTIFY-APP: one `emails/templates/<name>.html` per template,
loaded with `include_str!` (as `leads/service.rs:672` does there), with the
dynamic values as placeholders. Two levels, in order of cost:

1. Plain files plus a small placeholder replace, no new dependency. Enough for
   the 8 or so templates here.
2. `askama` (compile time checked templates) if the logic grows: conditionals
   and loops for the date list. It checks variables at build time, like
   `query_as!` does for SQL. A new dependency needs a reason in the commit.

Keep `CADENCE_EMAIL_PREVIEW_DIR` working: the files make a preview page and a
snapshot test per template easy. Do it with S23 (same file).

## Suggested order of work

1. **S1** first, alone, in its own commit. Everything after it is safer with the tests running.
2. **S2**, **S3/S9** and **S17** together (one `calendar_jobs` queue, `CalendarMode`, the `CalendarProvider` trait), because they change the same seam.
3. **S5**, then **S8**: both are about not losing things silently, and both are cheap.
4. **S4** and **S6**, one migration each.
5. **S21** with S2 (same Google slowness), then **S7**, **S18**, **S19**, **S20**, then the other LOW items as time allows. Per `AGENTS.md`, each item that belongs to a `PLAN_N` updates that plan in the same change; most of these fit a new `PLAN_27_RELIABILITY.md`.
6. **Frontend** (S24 to S27): any time, independent of the backend items. Start with S24, since S21 adds a 503 that the one client should handle.
