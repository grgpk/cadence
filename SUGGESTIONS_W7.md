# Suggestions, week 7

Reviewed MCP OAuth and tool authorization at `d66733a` for the Nov 5 session. Scope limited to MCP, as requested. No MCP code changed.

`cargo test` was attempted. SQLx query macros stopped compilation because the local `DATABASE_URL` schema lacks tables such as `availability_overrides`; zero tests ran. `cargo clippy` not run. Earlier `W4_SUGGESTIONS.md` and `W5_SUGGESTIONS.md` checked; this file does not repeat them.

## Time box

About 30 minutes: fix the OAuth code exchange requirements and add the regression tests below. Keep the existing `Full`/`Read` product choice for now. Discuss finer per-tool scopes as a future design question, not a blocker.

## Summary

| # | Priority | Where | What |
|---|---|---|---|
| 1 | Medium | `backend/src/domain/mcp/service.rs:355` | Require and verify `client_id` and `redirect_uri` when exchanging an OAuth code. |
| 2 | Low | `backend/tests/mcp_http.rs:543` | Exercise read-only denial for every write tool, not only `cancel_booking`. |

## 1. Token exchange accepts missing client and redirect bindings

**Problem picture**

```text
Consent stores: code -> client_id + redirect_uri + PKCE challenge
Exchange sends: code + verifier + [client_id] + [redirect_uri]
                                             optional    optional
```

The exchange skips both checks when their fields are omitted. RFC 6749 §4.1.3 requires `client_id` for unauthenticated clients and `redirect_uri` when authorization included it. The stored code still identifies its client, but require both request bindings to prevent code substitution and keep the OAuth flow conformant. [RFC 6749 §4.1.3](https://www.rfc-editor.org/rfc/rfc6749.html#section-4.1.3)

**Where:** `backend/src/domain/mcp/service.rs:355-372`

**Test first:** add to `backend/tests/mcp_oauth_http.rs`. It should compile, then fail now because each request missing a binding currently succeeds and mints a token. It should pass after the guard is added.

```rust
#[sqlx::test(migrations = "../migrations")]
async fn code_exchange_requires_client_and_redirect_bindings(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "bindings@example.com").await;
    let client_id = register_client(&app).await;

    for missing_field in ["client_id", "redirect_uri"] {
        let back = consent(&app, &cookie, &client_id, true).await;
        let code = query(&back, "code").expect("a code");
        let mut fields = vec![
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", REDIRECT),
            ("client_id", client_id.as_str()),
            ("code_verifier", VERIFIER),
        ];
        fields.retain(|(name, _)| *name != missing_field);

        let (status, refused) = token(&app, &fields).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{missing_field}: {refused}");
        assert_eq!(refused["error"], "invalid_request");
    }
}
```

**Before:**

```rust
async fn exchange_code(pool: &PgPool, request: TokenRequest) -> Result<TokenResponse, TokenError> {
    let (Some(code), Some(verifier)) = (request.code, request.code_verifier) else {
        return Err(TokenError::InvalidRequest(
            "code and code_verifier are needed",
        ));
    };
    let found = db::take_code(pool, &hash(&code))
        .await?
        .ok_or(TokenError::InvalidGrant)?;
    let matches = found.expires_at > Utc::now()
        && request
            .client_id
            .is_none_or(|id| id == found.oauth_client_unid)
        && request
            .redirect_uri
            .as_deref()
            .is_none_or(|uri| uri == found.redirect_uri)
        && URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == found.code_challenge;
    if !matches {
        return Err(TokenError::InvalidGrant);
    }
    let client = db::find_client(pool, found.oauth_client_unid)
        .await?
        .ok_or(TokenError::InvalidGrant)?;
    let user_active = users::find_by_unid(pool, found.user_unid)
        .await?
        .is_some_and(|user| user.status == UserStatus::Active);
    if !user_active {
        return Err(TokenError::InvalidGrant);
    }
    let (access_token, refresh_token) = (new_token(TOKEN_PREFIX), new_token(REFRESH_PREFIX));
    let expires_at = Utc::now() + OAUTH_TOKEN_LIFE;
    db::replace_grant(
        pool,
        found.user_unid,
        client.unid,
        &client.client_name,
        &hash(&access_token),
        &last_four(&access_token),
        found.scope,
        expires_at,
        &hash(&refresh_token),
        TOKENS_MAX,
    )
    .await?
    .ok_or(TokenError::InvalidGrant)?;
    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: OAUTH_TOKEN_LIFE.num_seconds(),
        refresh_token,
        scope: found.scope,
    })
}
```

**After:**

```rust
async fn exchange_code(pool: &PgPool, request: TokenRequest) -> Result<TokenResponse, TokenError> {
    let (Some(code), Some(redirect_uri), Some(client_id), Some(verifier)) = (
        request.code,
        request.redirect_uri,
        request.client_id,
        request.code_verifier,
    ) else {
        return Err(TokenError::InvalidRequest(
            "code, redirect_uri, client_id and code_verifier are needed",
        ));
    };
    let found = db::take_code(pool, &hash(&code))
        .await?
        .ok_or(TokenError::InvalidGrant)?;
    let matches = found.expires_at > Utc::now()
        && client_id == found.oauth_client_unid
        && redirect_uri == found.redirect_uri
        && URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == found.code_challenge;
    if !matches {
        return Err(TokenError::InvalidGrant);
    }
    let client = db::find_client(pool, found.oauth_client_unid)
        .await?
        .ok_or(TokenError::InvalidGrant)?;
    let user_active = users::find_by_unid(pool, found.user_unid)
        .await?
        .is_some_and(|user| user.status == UserStatus::Active);
    if !user_active {
        return Err(TokenError::InvalidGrant);
    }
    let (access_token, refresh_token) = (new_token(TOKEN_PREFIX), new_token(REFRESH_PREFIX));
    let expires_at = Utc::now() + OAUTH_TOKEN_LIFE;
    db::replace_grant(
        pool,
        found.user_unid,
        client.unid,
        &client.client_name,
        &hash(&access_token),
        &last_four(&access_token),
        found.scope,
        expires_at,
        &hash(&refresh_token),
        TOKENS_MAX,
    )
    .await?
    .ok_or(TokenError::InvalidGrant)?;
    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: OAUTH_TOKEN_LIFE.num_seconds(),
        refresh_token,
        scope: found.scope,
    })
}
```

This also keeps missing-field requests from consuming a valid one-use code. Existing wrong-verifier, code-reuse, and refresh tests stay. No plan promise needs changing.

## 2. Read-only denial test covers only cancellation

`McpHost::scope` is checked manually by each mutating tool. The existing integration test directly calls only `cancel_booking`. `tools/list` filtering is helpful, but a client can still send a direct `tools/call`; keep the per-call guard protected as new write tools arrive.

**Where:** `backend/tests/mcp_http.rs:543-595`, write guards in `backend/src/domain/mcp/server.rs`

**Add:** extend the existing read-only integration test with valid minimal arguments for every mutating tool (`mark_no_show`, `reschedule_booking`, booking/event-type changes, availability changes, profile/conferencing changes, and `follow_up_lead`). Assert each direct call returns an MCP tool error. Keep the existing state assertion for cancellation. No app code change unless a test exposes a missing guard.

## Order

```text
Missing-field test
      |
      v
Require client_id + redirect_uri and compare to stored code
      |
      v
Run OAuth tests

Read-only call matrix can land independently
```

## Future design topics

These are discussion topics for the next session, not W7 defects or work to start now.

### Least-privilege tool access

Cadence currently offers `Full` or `Read`. If users need agents to do only specific jobs, consider narrower scopes such as bookings-write, availability-write, or lead-follow-up. Check each scope on every tool call, and record allowed and denied actions. First ask: which real agent tasks need different permissions?

### Split the tool server as it grows

`server.rs` currently holds about 20 tools. Keep it together while it stays easy to navigate. When changes start crossing unrelated features, split tool routers and parameter types by feature, while continuing to call the existing domain services.

### Stable error codes for agents

Tool errors are readable text today. If clients need to branch on errors, return stable codes with the message, for example `booking_conflict` or `scope_denied`. Keep internal errors logged and return no database details to the agent.
