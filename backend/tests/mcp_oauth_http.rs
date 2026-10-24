//! OAuth for the MCP server: discovery, client registration, the host's
//! consent, codes traded for tokens, refresh, and revoking a connected app.

// Clippy's test escapes cover `#[test]` functions only; the helpers here are test code too.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cadence_backend::domain::calendly::{client::Calendly, models::CalendlyAccount};
use cadence_backend::{
    app_state::AppState,
    build_app,
    domain::{
        auth::service::make_token,
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
    },
};
use reqwest::Url;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const REDIRECT: &str = "https://claude.ai/api/mcp/auth_callback";
const APP_REDIRECT: &str = "cursor://anysphere.cursor-retrieval/oauth/callback";
const VERIFIER: &str = "a-long-random-verifier-that-is-at-least-43-characters";

fn app(pool: &PgPool) -> Router {
    dotenvy::dotenv().ok();
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
        payments: None,
        calendly: Calendly::fake(CalendlyAccount::default()),
    })
}

async fn respond(app: &Router, request: Request<Body>) -> (StatusCode, Value, Option<String>) {
    let response = app.clone().oneshot(request).await.expect("router responds");
    let status = response.status();
    let challenge = response
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body is readable");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        challenge,
    )
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("request is well formed");
    let (status, value, _) = respond(app, request).await;
    (status, value)
}

/// The token endpoint, form-encoded as OAuth clients send it.
async fn token(app: &Router, fields: &[(&str, &str)]) -> (StatusCode, Value) {
    let body = fields
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/oauth/token")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .expect("request is well formed");
    let (status, value, _) = respond(app, request).await;
    (status, value)
}

async fn tools_list(app: &Router, access_token: &str) -> (StatusCode, Option<String>) {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} });
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/mcp")
        .header(header::HOST, "bookcadence.app")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
        .body(Body::from(body.to_string()))
        .expect("request is well formed");
    let (status, _, challenge) = respond(app, request).await;
    (status, challenge)
}

async fn host(app: &Router, email: &str) -> String {
    let (status, user) = send(
        app,
        Method::POST,
        "/api/auth/register",
        None,
        Some(json!({ "email": email, "password": "password123", "full_name": "Nino Tbilisi" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let unid: Uuid = serde_json::from_value(user["unid"].clone()).expect("user has an id");
    format!(
        "session={}",
        make_token(unid, 0).expect("JWT_SECRET is set for tests")
    )
}

async fn register_client(app: &Router) -> String {
    let (status, client) = send(
        app,
        Method::POST,
        "/api/oauth/register",
        None,
        Some(json!({ "client_name": "Claude", "redirect_uris": [REDIRECT] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{client}");
    assert_eq!(client["token_endpoint_auth_method"], "none");
    client["client_id"]
        .as_str()
        .expect("a client id")
        .to_owned()
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// The host's answer on the consent screen, as the address it sends them to.
async fn consent(app: &Router, cookie: &str, client_id: &str, approve: bool) -> Url {
    let (status, answer) = send(
        app,
        Method::POST,
        "/api/oauth/authorize",
        Some(cookie),
        Some(json!({
            "client_id": client_id, "redirect_uri": REDIRECT, "code_challenge": challenge(VERIFIER),
            "code_challenge_method": "S256", "state": "xyz", "scope": "full", "approve": approve,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    Url::parse(answer["url"].as_str().expect("an address")).expect("a valid address")
}

fn query(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

/// A host connects a client and the client trades its code: the token pair.
async fn connect(app: &Router, cookie: &str) -> (String, Value) {
    let client_id = register_client(app).await;
    let back = consent(app, cookie, &client_id, true).await;
    let code = query(&back, "code").expect("a code");
    let (status, granted) = token(
        app,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", REDIRECT),
            ("client_id", &client_id),
            ("code_verifier", VERIFIER),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{granted}");
    (client_id, granted)
}

#[sqlx::test(migrations = "../migrations")]
async fn a_client_finds_the_server_from_a_401(pool: PgPool) {
    let app = app(&pool);
    let (status, challenge) = tools_list(&app, "cad_nothing").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let challenge = challenge.expect("a WWW-Authenticate header");
    assert!(
        challenge.contains("resource_metadata=")
            && challenge.contains("/.well-known/oauth-protected-resource/api/mcp"),
        "{challenge}"
    );

    let (status, resource) = send(
        &app,
        Method::GET,
        "/.well-known/oauth-protected-resource/api/mcp",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        resource["resource"]
            .as_str()
            .expect("a resource")
            .ends_with("/api/mcp")
    );
    let issuer = resource["authorization_servers"][0]
        .as_str()
        .expect("an issuer");

    let (status, server) = send(
        &app,
        Method::GET,
        "/.well-known/oauth-authorization-server",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(server["issuer"], issuer);
    assert_eq!(server["code_challenge_methods_supported"], json!(["S256"]));
    for endpoint in [
        "authorization_endpoint",
        "token_endpoint",
        "registration_endpoint",
    ] {
        assert!(server[endpoint].is_string(), "{endpoint}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_connected_app_reaches_the_tools(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "oauth@example.com").await;
    let (_, granted) = connect(&app, &cookie).await;
    assert_eq!(granted["token_type"], "Bearer");
    assert_eq!(granted["scope"], "full");
    assert!(granted["expires_in"].as_i64().expect("a life") > 0);
    let access = granted["access_token"].as_str().expect("an access token");

    let (status, _) = tools_list(&app, access).await;
    assert_eq!(status, StatusCode::OK);

    // The host sees it among their tokens and can shut it out.
    let (_, tokens) = send(
        &app,
        Method::GET,
        "/api/host/access-tokens",
        Some(&cookie),
        None,
    )
    .await;
    let listed = &tokens[0];
    assert_eq!(listed["name"], "Claude");
    assert_eq!(listed["connector"], true);
    let path = format!(
        "/api/host/access-tokens/{}",
        listed["unid"].as_str().expect("an id")
    );
    let (status, _) = send(&app, Method::DELETE, &path, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = tools_list(&app, access).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_code_is_refused_unless_it_is_right(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "codes@example.com").await;
    let client_id = register_client(&app).await;

    let code = query(&consent(&app, &cookie, &client_id, true).await, "code").expect("a code");
    let exchange = |verifier: &'static str, code: String| {
        let (app, client_id) = (app.clone(), client_id.clone());
        async move {
            token(
                &app,
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", REDIRECT),
                    ("client_id", &client_id),
                    ("code_verifier", verifier),
                ],
            )
            .await
        }
    };
    // A wrong verifier spends the code, so the right one is too late.
    let (status, refused) =
        exchange("the-wrong-verifier-the-wrong-verifier-wrong!", code.clone()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(refused["error"], "invalid_grant");
    let (status, _) = exchange(VERIFIER, code).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A code works once.
    let code = query(&consent(&app, &cookie, &client_id, true).await, "code").expect("a code");
    let (status, _) = exchange(VERIFIER, code.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = exchange(VERIFIER, code).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A code past its ten minutes.
    let code = query(&consent(&app, &cookie, &client_id, true).await, "code").expect("a code");
    sqlx::query_scalar!(
        "UPDATE oauth_codes SET expires_at = now() - interval '1 second' RETURNING 1"
    )
    .fetch_all(&pool)
    .await
    .expect("codes are writable");
    let (status, _) = exchange(VERIFIER, code).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, refused) = token(&app, &[("grant_type", "password")]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(refused["error"], "unsupported_grant_type");
}

#[sqlx::test(migrations = "../migrations")]
async fn consent_only_sends_the_host_where_the_client_registered(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "consent@example.com").await;
    let client_id = register_client(&app).await;

    let (status, _) = send(
        &app,
        Method::POST,
        "/api/oauth/authorize",
        Some(&cookie),
        Some(json!({
            "client_id": client_id, "redirect_uri": "https://evil.example/callback",
            "code_challenge": challenge(VERIFIER), "code_challenge_method": "S256",
            "scope": "full", "approve": true,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let denied = consent(&app, &cookie, &client_id, false).await;
    assert_eq!(query(&denied, "error").as_deref(), Some("access_denied"));
    assert_eq!(query(&denied, "state").as_deref(), Some("xyz"));
    assert!(query(&denied, "code").is_none());

    // Without a session there is no consent.
    let (status, _) = send(
        &app,
        Method::GET,
        &format!("/api/oauth/clients/{client_id}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    for redirect_uris in [
        json!([]),
        json!(["http://example.com/callback"]),
        json!(["not a url"]),
    ] {
        let (status, _) = send(
            &app,
            Method::POST,
            "/api/oauth/register",
            None,
            Some(json!({ "client_name": "Bad", "redirect_uris": redirect_uris })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{redirect_uris}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_refresh_token_works_once(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "refresh@example.com").await;
    let (client_id, granted) = connect(&app, &cookie).await;
    let old_access = granted["access_token"].as_str().expect("an access token");
    let old_refresh = granted["refresh_token"].as_str().expect("a refresh token");

    let refresh = |refresh_token: String| {
        let (app, client_id) = (app.clone(), client_id.clone());
        async move {
            token(
                &app,
                &[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", &refresh_token),
                    ("client_id", &client_id),
                ],
            )
            .await
        }
    };
    let (status, renewed) = refresh(old_refresh.to_owned()).await;
    assert_eq!(status, StatusCode::OK, "{renewed}");
    let new_access = renewed["access_token"].as_str().expect("an access token");
    assert_ne!(new_access, old_access);

    assert_eq!(tools_list(&app, new_access).await.0, StatusCode::OK);
    assert_eq!(
        tools_list(&app, old_access).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, _) = refresh(old_refresh.to_owned()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // An access token past its hour is refused.
    sqlx::query_scalar!(
        "UPDATE access_tokens SET expires_at = now() - interval '1 second' RETURNING 1"
    )
    .fetch_all(&pool)
    .await
    .expect("tokens are writable");
    assert_eq!(
        tools_list(&app, new_access).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_desktop_app_gets_its_code_on_its_own_scheme(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "editor@example.com").await;
    let (status, client) = send(
        &app,
        Method::POST,
        "/api/oauth/register",
        None,
        Some(json!({ "client_name": "Cursor", "redirect_uris": [APP_REDIRECT] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{client}");

    let (status, answer) = send(
        &app,
        Method::POST,
        "/api/oauth/authorize",
        Some(&cookie),
        Some(json!({
            "client_id": client["client_id"], "redirect_uri": APP_REDIRECT,
            "code_challenge": challenge(VERIFIER), "code_challenge_method": "S256",
            "state": "xyz", "scope": "read", "approve": true,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let back = Url::parse(answer["url"].as_str().expect("an address")).expect("a valid address");
    assert_eq!(back.scheme(), "cursor");
    assert!(query(&back, "code").is_some());

    for unsafe_uri in [
        "javascript:alert(1)",
        "data:text/html,hi",
        "file:///etc/passwd",
    ] {
        let (status, _) = send(
            &app,
            Method::POST,
            "/api/oauth/register",
            None,
            Some(json!({ "client_name": "Bad", "redirect_uris": [unsafe_uri] })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{unsafe_uri}");
    }
}

/// A browser's preflight from `origin`: the origin it is allowed, if any.
async fn preflight(app: &Router, path: &str, origin: &str) -> Option<String> {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri(path)
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization, mcp-protocol-version",
        )
        .body(Body::empty())
        .expect("request is well formed");
    let response = app.clone().oneshot(request).await.expect("router responds");
    response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[sqlx::test(migrations = "../migrations")]
async fn browser_clients_reach_only_the_agent_paths(pool: PgPool) {
    let site = "https://bookcadence.app";
    let app = app(&pool).layer(cadence_backend::cors(vec![
        site.parse().expect("an origin"),
    ]));
    let inspector = "https://inspector.example";

    for path in [
        "/api/mcp",
        "/api/oauth/token",
        "/api/oauth/register",
        "/.well-known/oauth-authorization-server",
    ] {
        assert_eq!(
            preflight(&app, path, inspector).await.as_deref(),
            Some(inspector),
            "{path}"
        );
    }
    // What the session reaches stays with the site.
    for path in [
        "/api/oauth/authorize",
        "/api/auth/me",
        "/api/host/access-tokens",
    ] {
        assert_eq!(preflight(&app, path, inspector).await, None, "{path}");
        assert_eq!(
            preflight(&app, path, site).await.as_deref(),
            Some(site),
            "{path}"
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn old_codes_and_unused_clients_are_swept(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "sweep@example.com").await;
    let (connected, _) = connect(&app, &cookie).await;
    let fresh = register_client(&app).await;
    let stale = register_client(&app).await;
    let waiting = register_client(&app).await;
    consent(&app, &cookie, &waiting, true).await;
    let ids = |ids: &[&str]| {
        ids.iter()
            .map(|id| id.parse::<Uuid>().expect("a client id"))
            .collect::<Vec<_>>()
    };
    sqlx::query_scalar!(
        "UPDATE oauth_clients SET created_at = now() - interval '2 days' WHERE unid = ANY($1) RETURNING 1",
        &ids(&[&connected, &stale, &waiting]),
    )
    .fetch_all(&pool)
    .await
    .expect("clients are writable");

    cadence_backend::domain::mcp::service::forget_old(&pool)
        .await
        .expect("the sweep runs");
    let left = sqlx::query_scalar!("SELECT unid FROM oauth_clients")
        .fetch_all(&pool)
        .await
        .expect("clients are readable");
    for kept in ids(&[&connected, &fresh, &waiting]) {
        assert!(left.contains(&kept));
    }
    assert!(!left.contains(&ids(&[&stale])[0]));

    // Once the waiting code runs out, its client goes with the next sweep.
    sqlx::query_scalar!(
        "UPDATE oauth_codes SET expires_at = now() - interval '1 second' RETURNING 1"
    )
    .fetch_all(&pool)
    .await
    .expect("codes are writable");
    cadence_backend::domain::mcp::service::forget_old(&pool)
        .await
        .expect("the sweep runs");
    let codes = sqlx::query_scalar!("SELECT count(*) FROM oauth_codes")
        .fetch_one(&pool)
        .await
        .expect("codes are readable");
    assert_eq!(codes, Some(0));
    let left = sqlx::query_scalar!("SELECT unid FROM oauth_clients")
        .fetch_all(&pool)
        .await
        .expect("clients are readable");
    assert!(!left.contains(&ids(&[&waiting])[0]));
}

/// The MCP server's address, as discovery names it.
async fn resource(app: &Router) -> String {
    let (_, metadata) = send(
        app,
        Method::GET,
        "/.well-known/oauth-protected-resource/api/mcp",
        None,
        None,
    )
    .await;
    metadata["resource"]
        .as_str()
        .expect("a resource")
        .to_owned()
}

#[sqlx::test(migrations = "../migrations")]
async fn a_token_is_only_for_this_server(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "resource@example.com").await;
    let client_id = register_client(&app).await;
    let ours = resource(&app).await;

    let ask = |resource: &str| {
        json!({
            "client_id": client_id, "redirect_uri": REDIRECT, "code_challenge": challenge(VERIFIER),
            "code_challenge_method": "S256", "state": "xyz", "scope": "full", "approve": true,
            "resource": resource,
        })
    };
    let (_, answer) = send(
        &app,
        Method::POST,
        "/api/oauth/authorize",
        Some(&cookie),
        Some(ask("https://other.example/mcp")),
    )
    .await;
    let back = Url::parse(answer["url"].as_str().expect("an address")).expect("a valid address");
    assert_eq!(query(&back, "error").as_deref(), Some("invalid_target"));
    assert!(query(&back, "code").is_none());

    let (_, answer) = send(
        &app,
        Method::POST,
        "/api/oauth/authorize",
        Some(&cookie),
        Some(ask(&ours)),
    )
    .await;
    let back = Url::parse(answer["url"].as_str().expect("an address")).expect("a valid address");
    let code = query(&back, "code").expect("a code");
    let fields = |resource: &str| {
        vec![
            ("grant_type", "authorization_code".to_owned()),
            ("code", code.clone()),
            ("redirect_uri", REDIRECT.to_owned()),
            ("client_id", client_id.clone()),
            ("code_verifier", VERIFIER.to_owned()),
            ("resource", resource.to_owned()),
        ]
    };
    let wrong = fields("https://other.example/mcp");
    let wrong: Vec<(&str, &str)> = wrong
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    let (status, refused) = token(&app, &wrong).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(refused["error"], "invalid_target");

    let right = fields(&ours);
    let right: Vec<(&str, &str)> = right
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    let (status, granted) = token(&app, &right).await;
    assert_eq!(status, StatusCode::OK, "{granted}");
}

async fn revoke(app: &Router, fields: &[(&str, &str)]) -> StatusCode {
    let body = fields
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/oauth/revoke")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .expect("request is well formed");
    respond(app, request).await.0
}

#[sqlx::test(migrations = "../migrations")]
async fn a_client_can_disconnect_itself(pool: PgPool) {
    let app = app(&pool);
    let cookie = host(&app, "revoke@example.com").await;
    let (client_id, granted) = connect(&app, &cookie).await;
    let access = granted["access_token"].as_str().expect("an access token");
    let refresh = granted["refresh_token"].as_str().expect("a refresh token");

    // Another client's id leaves the grant alone.
    let other = register_client(&app).await;
    assert_eq!(
        revoke(&app, &[("token", refresh), ("client_id", &other)]).await,
        StatusCode::OK
    );
    assert_eq!(tools_list(&app, access).await.0, StatusCode::OK);

    assert_eq!(
        revoke(&app, &[("token", refresh), ("client_id", &client_id)]).await,
        StatusCode::OK
    );
    assert_eq!(tools_list(&app, access).await.0, StatusCode::UNAUTHORIZED);
    let (status, _) = token(
        &app,
        &[("grant_type", "refresh_token"), ("refresh_token", refresh)],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Unknown tokens answer the same, and a token made by hand is not an
    // OAuth grant to revoke.
    assert_eq!(
        revoke(&app, &[("token", "cad_unknown")]).await,
        StatusCode::OK
    );
    let (status, created) = send(
        &app,
        Method::POST,
        "/api/host/access-tokens",
        Some(&cookie),
        Some(json!({ "name": "Claude Code" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let manual = created["token"].as_str().expect("a token");
    assert_eq!(revoke(&app, &[("token", manual)]).await, StatusCode::OK);
    assert_eq!(tools_list(&app, manual).await.0, StatusCode::OK);

    let (_, server) = send(
        &app,
        Method::GET,
        "/.well-known/oauth-authorization-server",
        None,
        None,
    )
    .await;
    assert!(
        server["revocation_endpoint"]
            .as_str()
            .expect("a revocation endpoint")
            .ends_with("/api/oauth/revoke")
    );
}
