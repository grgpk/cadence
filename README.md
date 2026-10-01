# Cadence

A small scheduling and booking backend written in Rust. Think of it as a tiny Cal.com. A host sets their weekly availability, visitors book open slots, and nobody gets double-booked. Each booking triggers a confirmation email and a reminder.

> Status: MVP in progress. The JSON API works end to end. There's no frontend or deployment yet.

## Stack

- **Axum** + **Tokio**: HTTP server and async runtime
- **Postgres** via **sqlx**: queries and migrations
- **argon2**: password hashing. Sessions use cookies.
- **lettre**: SMTP email, sent by a background **outbox** worker
- **chrono**: dates, times, slot generation

## Running locally

You need Rust, Postgres, `sqlx-cli`, and an SMTP catcher such as [Mailpit](https://mailpit.axllent.org/) listening on `localhost:1025`.

```sh
echo 'DATABASE_URL=postgres://user:pass@localhost/cadence' > .env
sqlx database create
sqlx migrate run
cargo run            # → http://127.0.0.1:3000
```

To check it's up, run `curl localhost:3000/health`, which should return `ok`.

## API

| Method | Path                       | Auth | What it does                               |
|--------|----------------------------|------|--------------------------------------------|
| POST   | `/api/register`            |      | Create a host `{name, email, password}`     |
| POST   | `/api/login`               |      | Log in and get a `session` cookie          |
| POST   | `/api/logout`              | ✓    | End the session                            |
| GET    | `/api/hosts/{id}`          |      | Public host profile                        |
| GET    | `/api/availability`        | ✓    | List your availability rules               |
| POST   | `/api/availability`        | ✓    | Add a rule `{weekday, start_time, end_time, slot_minutes}` (weekday 0 = Monday) |
| GET    | `/api/hosts/{id}/slots?days=N` |  | Free slots for the next N days             |
| POST   | `/api/hosts/{id}/bookings` |      | Book a slot `{slot_start, invitee_name, invitee_email}` |
| GET    | `/api/bookings`            | ✓    | Your bookings                              |

## How email works

Each booking writes its confirmation and reminder emails to an `outbox` table in the same transaction as the booking. A background task checks the table every 10 seconds and sends any message whose `send_after` time has passed. If the booking fails, no email exists. If SMTP is down, the emails stay queued and go out once it recovers.

## Layout

```
src/
  main.rs          wiring: pool, mailer, worker, router
  auth.rs          register / login / logout, CurrentHost extractor
  hosts.rs         public host profile
  availability.rs  weekly rules + slot generation
  bookings.rs      booking creation (double-book safe) + listing
  outbox.rs        email queue + background worker
  error.rs         shared API error type
migrations/        sqlx migrations
```

## Roadmap

Deploy (live URL) → server-rendered UI (Askama + HTMX) → timezones → multiple event types → Google Calendar sync → Stripe payments → Leptos (Rust/WASM) frontend.
