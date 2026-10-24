# Plan 47: MCP polish for read-only agents, new hosts, and token hygiene

Status: implemented.
- Backend: fmt, clippy, every test passes (MCP 12 HTTP, OAuth 10 HTTP).
- Frontend: build, typecheck, lint, knip and tests pass.

## Goal

- A read-only agent never plans a change it would be refused.
- Someone without an account can sign up mid-connect and still land on the
  consent page.
- An OAuth token works only for this server, and a client can disconnect
  itself.

## The gap this fixes

- `tools/list` showed every tool to a read-only token. Calling a write tool
  failed only at call time.
- Login's "Create an account" link dropped the `redirect`, so a new host
  ended up on the dashboard instead of the consent page.
- The `resource` parameter (RFC 8707) was ignored.
- Clients had no way to revoke their own grant.

## Data model

No change.

## API

- **`tools/list`** for a read-only token returns only tools annotated
  `read_only_hint`. `CadenceMcp::list_tools` overrides the macro's version
  and sends no shared cache hint, because the list differs per token.
  `call_tool` still refuses writes as before.
- **`resource`** is checked when sent: on consent (`AuthorizeRequest.resource`,
  passed through from the consent page's search) and on `/api/oauth/token`.
  - It must be this server's MCP address, with or without a trailing slash.
  - Anything else answers `invalid_target`: as a redirect error from
    consent, as a 400 from the token endpoint.
  - Leaving it out is allowed, for older clients.
- **`POST /api/oauth/revoke`** (RFC 7009), form `token` (access or refresh)
  and optional `client_id`:
  - deletes that OAuth grant and answers 200, whether the token was known or
    not
  - tokens a host made by hand are not touched
  - it is listed as `revocation_endpoint` in the metadata, and it is an agent
    path, so CORS is open

## Frontend

- `/register` takes `?redirect=` through `safeReturnPath`, and goes there
  after sign-up.
- Login and register pass `redirect` to each other's links.
- Registration sets no session, so a new host goes through login on the way
  back, and the route guard keeps the return path.

## Tests

- `mcp_http.rs`: a read-only token's tool list has `list_bookings` and not
  `cancel_booking`; a full token sees both.
- `mcp_oauth_http.rs`:
  - a token is only for this server: consent and the token endpoint both
    answer `invalid_target` for another resource
  - a client can disconnect itself: another client's id does nothing, the
    right one kills the access and refresh tokens, an unknown token gets 200,
    and a hand-made token survives
