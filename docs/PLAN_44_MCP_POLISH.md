# Plan 44: MCP agents get better data, read-only tokens and a rate limit

Status: implemented.
- Backend: fmt, clippy, every test passes (MCP 12 HTTP).
- Frontend: build, typecheck, lint, knip and tests pass.

## Goal

An agent answers "what's on Thursday" and "how many intro calls this week"
without guessing, and a host can give an agent a token that only looks.

## The gap this fixes

Plan 43's tools worked, but:
- A call did not say which kind of call it was, and its start was in UTC
  only, so the agent did the timezone sums itself.
- `list_bookings` filtered `from`/`to` in memory, after the database had
  already cut the list to the newest 500 calls. An older span came back
  empty.
- `update_event_type` needed every field, so a one-field change risked
  resetting the others.
- Conferencing had no tool.
- Every token could cancel and book, and nothing slowed down an agent stuck in
  a loop.

## Data model

`migrations/0022_create_access_tokens.sql` gains, in its CREATE statement:

```
scope  TEXT NOT NULL DEFAULT 'full'  CHECK (scope IN ('full', 'read'))
```

`bookings.event_type_unid` already existed. `Booking` now carries it and the
kind's title (`None` once the kind is deleted).

## API

```
POST /api/host/access-tokens  { name, scope?: "full" | "read" }  (full when left out)
GET  /api/host/access-tokens  each token lists its scope
```

`AccessTokenScope` is a wire type. `/api/mcp` answers 429 past 300 calls
per token in 10 minutes (`rate_limits::MCP_PER_TOKEN`).

## Tools

- `list_bookings` filters status, `from` and `to` in SQL before the cut.
  Each call has `event_type_unid`, `event_type_title` and `slot_start_local`
  (`YYYY-MM-DDTHH:MM` on the host's clock).
- `get_booking` returns the same call shape. `find_open_slots` adds
  `start_local`.
- `update_event_type` changes only the fields sent.
- New `set_conferencing`: `{ app: google_meet | custom_link, meeting_link? }`.
- Every answer also goes out as `structuredContent`. A list goes under
  `items`, because structured content must be an object.
- A read-only token gets a tool error from every tool that changes
  something, and the error says how to get a full token.

## Frontend

```
┌──────────────┬──────────────────────────────────────────────────────┐
│ Apps       ▾ │ AI agents                                            │
│  App store   │ Token name [ Claude Code… ] [Full access ▾] [Create] │
│  Installed   │ ⚿ Claude Code  …a91f  [Read only]  Used …  [Revoke]  │
└──────────────┴──────────────────────────────────────────────────────┘
```

## Tests

`backend/tests/mcp_http.rs`:
- a call says its kind and the host's clock, and `from` leaves out earlier
  calls
- a kind of call changes only what is sent
- `set_conferencing` needs the link for `custom_link`
- a read-only token lists but cannot cancel
- the 301st call in the window answers 429

## Done when

- An agent lists calls with their kind and local time, and reaches any span.
- A host makes a read-only token, and it cannot change anything.
