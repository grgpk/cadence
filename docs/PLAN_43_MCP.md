# Plan 43: hosts run Cadence from their AI agent (MCP)

Status: implemented.
- Backend: fmt, clippy, every test passes (MCP 7 HTTP; booking 57 with the
  host move).
- Frontend: build, typecheck, lint, knip and tests pass.
- Checked with curl against the dev backend, using a token made through the
  API: `initialize`, `tools/list` (20 tools) and `get_profile`. Without a
  token the endpoint answers 401, and an unknown path still answers 404.

## Goal

A host manages their calls from Claude, Cursor or any other MCP client:
- "what's on tomorrow"
- "move Ada's call to Thursday at 3"
- "take next Friday off"
- "book Sam in for an intro call"

The agent works with the host's own data, under the same rules and plan gates
as the dashboard.

## The gap this fixes

Until this plan the web app was the only way in. The session JWT is meant for
a browser, and the backend had no long-lived credential an agent could keep
and the host could revoke. Hosts also had no way to move a call themselves:
only the invitee could, through the manage link.

## Data model

`migrations/0022_create_access_tokens.sql`, a new table:

```
access_tokens  unid, user_unid, name (1–80), token_hash (sha256, unique),
               last_four, last_used_at, created_at
```

- The token is `cad_` followed by 64 hex characters (two v4 UUIDs, as webhook
  secrets are made).
- It is shown once; only its hash is kept.
- A host has at most 20 tokens.
- Revoking a token deletes its row.
- A banned user's tokens stop working. A password change does not revoke them;
  the host revokes a token on its own.

## API

```
GET    /api/host/access-tokens          the host's tokens, never the secret
POST   /api/host/access-tokens          { name } → 201 { access_token, token }; 409 past 20
DELETE /api/host/access-tokens/{unid}
POST   /api/bookings/{unid}/reschedule  { slot_start }: the host moves their own call
POST   /api/mcp                         MCP Streamable HTTP, `Authorization: Bearer cad_…`
```

`/api/mcp` sits under nginx's existing `location /api/`, so deploying it needs
no proxy change.

The server runs stateless and answers JSON (`legacy_session_mode: false`,
`json_response: true`):
- There are no MCP sessions and no SSE streams.
- Each POST stands on its own, so it works behind the proxy and across
  restarts.
- rmcp's Host allowlist is off. The bearer token guards the endpoint, and a
  browser never sends it on its own, so DNS rebinding gets an attacker nothing.

## Tools

Each tool acts only for the token's host, and each one calls the service the
dashboard uses.

| Area | Tools |
|---|---|
| Calls | `list_bookings` (status, from, to, limit), `get_booking`, `cancel_booking`, `mark_no_show`, `reschedule_booking`, `find_open_slots`, `create_booking`, `get_insights` |
| Kinds of call | `list_event_types`, `create_event_type`, `update_event_type`, `delete_event_type` |
| Hours | `get_availability`, `set_weekly_hours`, `set_date_override`, `clear_date_override` |
| Booking page | `get_profile`, `update_profile` (only the fields sent change) |
| Win back | `list_unfinished_leads`, `follow_up_lead` |

Some areas have no tools:
- billing, Google and Calendly connect, and webhooks, which need a browser or
  a secret
- every admin endpoint

Errors split two ways:
- **Tool errors** (`isError`): anything the host could act on, so the agent
  can read it and try again. That covers a taken time, a call that has already
  started, someone else's id, or a feature past the plan; the last one points
  to `/dashboard/billing`.
- **Protocol errors**: database and internal failures, which are also logged.

## Flow

```
agent ──POST /api/mcp, Bearer cad_…──▶ require_token (route_layer)
          │ sha256 → access_tokens → active user → McpHost on the request
          ▼
   rmcp StreamableHttpService ──▶ CadenceMcp tool ──▶ domain service
          (bookings / event_types / availability / host_settings / leads)
          ──▶ same emails (email_queue) and webhooks as the dashboard
```

The token check uses `route_layer`. A plain `layer` would also guard the
router's fallback, and unknown paths would answer 401 instead of 404.

## Frontend

The "AI agents (MCP)" app goes in the App store under Automation (Lucide
`Bot`), and its panel also appears on Installed apps:

```
┌──────────────┬──────────────────────────────────────────────────┐
│ Apps       ▾ │ AI agents                                        │
│  App store   │ Server URL                                  [⧉]  │
│  Installed   │ https://bookcadence.app/api/mcp                  │
│              │ Token name [ Claude Code…    ] [ Create token ]  │
│              │ ⚿ Claude Code  …a91f  Used Oct 24  Made Oct 24  │
│              │                                       [Revoke]   │
└──────────────┴──────────────────────────────────────────────────┘
 After create, the form gives way to the token's one showing:
 Token [⧉] · Claude Code command [⧉] · mcpServers JSON [⧉] · [I saved it]
```

The server URL is `VITE_API_URL` + `/api/mcp`.

## Tests

- `backend/tests/mcp_http.rs`:
  - a token is shown once and listed without its secret; revoking it shuts
    the agent out
  - no token, a garbage token, or a session JWT gets 401
  - `initialize` and `tools/list` work
  - an agent books, moves and cancels a call, and emails are queued
  - a taken time comes back as a tool error
  - another host's calls and kinds of call cannot be reached
  - new weekly hours change the open slots, and bad hours are a tool error
  - a feature past the plan answers with an upgrade hint
- `host_booking_http.rs` `a_host_moves_their_own_call`: someone else's call
  answers 404 and a taken time 409; both sides are emailed, and the old time
  opens again.
- `frontend/src/domains/integrations/ai-agents.test.ts`: the server URL and
  the setup snippets.

## Done when

- A host makes a token in Apps and pastes the `claude mcp add` line.
- Their agent lists, books, moves and cancels calls, and changes hours and
  kinds of call, with the same emails and rules as the dashboard.
- Revoking the token shuts the agent out at once.

## Later

OAuth 2.1 (discovery, dynamic client registration, PKCE, a consent screen),
so the claude.ai and ChatGPT "add connector by URL" flows work without
pasting a token.
