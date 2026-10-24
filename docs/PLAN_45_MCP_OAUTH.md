# Plan 45: connect Cadence to claude.ai and ChatGPT by URL (MCP OAuth)

Status: implemented.
- Backend: fmt, clippy, every test passes (OAuth 5 HTTP, MCP 12 HTTP).
- Frontend: build, typecheck, lint, knip and tests pass.
- Checked with curl against the dev backend. The 401 carries
  `resource_metadata`. Then: register a client, consent, trade the code, and a
  read-only connector lists calls and is refused a cancel.

## Goal

A host pastes `https://bookcadence.app/api/mcp` into claude.ai or ChatGPT as
a custom connector, logs in, picks full access or read only, and the agent is
connected. They paste no token.

## The gap this fixes

Plan 43's tokens work for clients that take a header, such as Claude Code or
Cursor. The "add connector by URL" flows speak only OAuth 2.1, as the MCP
authorization spec describes it.

## Data model

New CREATE migrations. `access_tokens` moves after `oauth_clients`, because
its new foreign key points there:

```
0022 oauth_clients   unid (= client_id), client_name (1–80), redirect_uris (1–10), created_at
0023 access_tokens   + oauth_client_unid (cascade), expires_at, refresh_hash (unique)
0024 oauth_codes     code_hash, oauth_client_unid, user_unid, redirect_uri,
                     code_challenge (S256), scope, expires_at
```

- A consent ends up as an ordinary access token row named after the client.
  So the Apps list shows it with a "Connected app" badge, and revoking it
  works the same way.
- Connecting the same client again replaces its row.
- The 20-token cap still holds.
- OAuth access tokens (`cad_…`) last 1 hour.
- The refresh token (`cadr_…`) is replaced every time it is used. An old one
  stops working.
- Codes (`cadc_…`) last 10 minutes and work once.
- Only hashes are kept.

## API

```
GET  /.well-known/oauth-protected-resource[/api/mcp]  RFC 9728: resource, authorization_servers
GET  /.well-known/oauth-authorization-server          RFC 8414: endpoints, S256, public clients
POST /api/oauth/register    RFC 7591, open, 20 per address per hour
     { client_name, redirect_uris: https, or http on localhost } → 201 { client_id, … }
GET  /api/oauth/clients/{client_id}   session: name and redirect URIs, for the consent page
POST /api/oauth/authorize   session: AuthorizeRequest → { url } back to the client,
                            with code and state, or error=access_denied
POST /api/oauth/token       form: authorization_code (PKCE S256) or refresh_token
                            → { access_token, token_type, expires_in, refresh_token, scope }
```

- A 401 from `/api/mcp` says
  `WWW-Authenticate: Bearer resource_metadata="…/.well-known/oauth-protected-resource/api/mcp"`.
- The issuer is `PUBLIC_API_URL`, or else `APP_URL`. In production that is
  the site, so nginx sends `/.well-known/oauth-*` to the backend.
- A redirect URI the client did not register is refused with a 400, and the
  host is never sent there.
- Token errors use the OAuth names (`invalid_request`, `invalid_grant`,
  `unsupported_grant_type`) and come with `Cache-Control: no-store`.

## Flow

```
client ──POST /api/mcp──▶ 401 + resource_metadata
       ──GET .well-known──▶ issuer, endpoints
       ──POST register──▶ client_id
browser ──/oauth/authorize?client_id…&code_challenge…──▶ (login if needed)
        consent page ──POST /api/oauth/authorize──▶ redirect_uri?code&state
client ──POST /api/oauth/token (code + verifier)──▶ access + refresh tokens
       ──POST /api/mcp, Bearer cad_…──▶ the Plan 43 tools, with the chosen scope
```

## Frontend

`/oauth/authorize` is a protected root, so someone signed out goes to
`/login?redirect=…` and comes back. It reuses `AuthCard`:

```
┌───────────────────────────────┬──────────────────┐
│ Connect Claude                │                  │
│ It will act for you in        │  night side      │
│ Cadence, signed in as …       │  (day tape)      │
│ ┌ Full access ─────────── (•)┐│                  │
│ └ Read only ───────────── ( )┘│                  │
│              [Cancel] [Allow] │                  │
│ You go back to claude.ai …    │                  │
└───────────────────────────────┴──────────────────┘
```

A link that is missing parts, or names an unknown client or redirect, shows
"This link does not work" and sends no one anywhere.

## Tests

`backend/tests/mcp_oauth_http.rs`:
- a client finds the server from a 401 and the discovery documents
- a connected app reaches the tools, shows up as a connector, and revoking
  it shuts it out
- a wrong verifier, a reused code, an expired code and an unknown grant type
  are each refused
- consent only redirects to a registered address; Cancel answers
  `access_denied` with the state; there is no consent without a session; bad
  redirect URIs cannot register
- a refresh token works once, the old access token stops working, and an
  expired one is refused

Frontend: `oauth-api.test.ts` (search values, the redirect host), and
`routes.test.ts` (the consent page needs a session).

## Done when

- Adding `https://bookcadence.app/api/mcp` as a custom connector in claude.ai
  goes through login and consent, and the tools work.
- The connector shows in Apps and can be revoked there.

## Later

CORS for browser clients, desktop apps' own redirect schemes, and sweeping old
clients and codes are done in Plan 46.
