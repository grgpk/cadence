# Plan 46: MCP OAuth for desktop editors and browser clients

Status: implemented.
- Backend: fmt, clippy, every test passes (OAuth 8 HTTP).
- Frontend: build, typecheck, lint, knip and tests pass.
- Checked with curl against the dev backend. A preflight from any origin is
  allowed on `/api/oauth/token` and `/api/mcp`, and refused on `/api/auth/me`.

## Goal

Every kind of MCP client can connect through OAuth:
- CLIs and server-side connectors (Plan 45)
- desktop editors that call back to their own scheme
- clients that run in a browser

## The gap this fixes

- Registration only took `https://` and `http://localhost` callbacks, so an
  editor calling back to `cursor://…` was refused.
- CORS only allowed the site, so a browser client such as the MCP Inspector
  could not reach discovery, register, token or `/api/mcp`.
- Open registration and expired codes left rows behind forever.

## Data model

No change.

## API

- **Redirect URIs** may now also use an app's own scheme (RFC 8252):
  - any scheme except `http` off localhost, `javascript`, `data`, `file`,
    `blob`, `vbscript` and `about`
  - still no fragment
- **CORS** (`cadence_backend::cors`, used by `main.rs`):
  - Any origin may call the agent paths (`mcp::routes::is_agent_path`):
    `/api/mcp`, `/api/oauth/register`, `/api/oauth/token` and
    `/.well-known/oauth-*`. These read a Bearer token or a form, never the
    session cookie.
  - Everything else, including `/api/oauth/authorize` and
    `/api/oauth/clients`, stays with `CORS_ORIGINS`.
  - Requests may send `mcp-protocol-version` and `mcp-session-id`.
    `WWW-Authenticate` and `mcp-session-id` are exposed.
- **Sweep:** the minute sweep (`emails::worker::sweep`) runs
  `mcp::service::forget_old`. It drops expired codes, and clients older than a
  day that no token or code points at.

## Frontend

The consent footer names a desktop app by its scheme ("You go back to
cursor…") and a site by its host, as before.

## Tests

`backend/tests/mcp_oauth_http.rs`:
- a desktop app gets its code on its own scheme, and unsafe schemes are
  refused
- browser clients reach only the agent paths, and the session paths stay
  with the site
- old codes and unused clients are swept; a connected client, a fresh one,
  and one with a live code stay

Also `oauth-api.test.ts`, which checks that a custom scheme is named by its
scheme.

## Done when

- An editor with an app-scheme callback, and the MCP Inspector in a browser,
  complete the OAuth flow.
