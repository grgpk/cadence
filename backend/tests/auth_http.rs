//! Accounts over HTTP: password reset by email, profile and password changes,
//! and the limits that slow down guessing and spam.

// Clippy's test escapes cover `#[test]` functions only; the helpers here are test code too.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use cadence_backend::{
    app_state::AppState,
    build_app,
    domain::{auth::service::make_token, rate_limits::service as rate_limits},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const PASSWORD: &str = "password123";
const NEW_PASSWORD: &str = "a-better-password";
const LOGIN: &str = "/api/auth/login";
const FORGOT: &str = "/api/auth/forgot-password";
const RESET: &str = "/api/auth/reset-password";
const ME: &str = "/api/auth/me";
const CHANGE_PASSWORD: &str = "/api/auth/password";

fn app(pool: &PgPool) -> Router {
    // JWT_SECRET lives in the workspace .env, next to DATABASE_URL.
    dotenvy::dotenv().ok();
    build_app(AppState { pool: pool.clone() })
}

/// `address` is sent as `x-forwarded-for`; without it the request has no
/// client address at all.
async fn send(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    address: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(address) = address {
        request = request.header("x-forwarded-for", address);
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("request is well formed");
    let response = app.clone().oneshot(request).await.expect("router responds");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body is readable");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn post(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    send(app, Method::POST, path, None, None, Some(body)).await
}

/// Registers a user with `PASSWORD` and returns their session cookie.
async fn register(app: &Router, full_name: &str, email: &str) -> String {
    let (status, user) = post(
        app,
        "/api/auth/register",
        json!({ "email": email, "password": PASSWORD, "full_name": full_name }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let unid: Uuid = serde_json::from_value(user["unid"].clone()).expect("user has an id");
    let token = make_token(unid).expect("JWT_SECRET is set for tests");
    format!("session={token}")
}

async fn login(app: &Router, email: &str, password: &str) -> StatusCode {
    post(app, LOGIN, json!({ "email": email, "password": password }))
        .await
        .0
}

/// The reset tokens waiting in the queue for `email`, read from the links
/// their emails carry.
async fn reset_tokens(pool: &PgPool, email: &str) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT payload->>'reset_url' AS "link!" FROM email_queue
           WHERE template = 'password_reset' AND booking_unid IS NULL AND email = $1
           ORDER BY created_at"#,
        email,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
    .into_iter()
    .map(|link| {
        let (page, token) = link.split_once("?token=").expect("the link has a token");
        assert!(page.ends_with("/reset-password"), "{page}");
        token.to_owned()
    })
    .collect()
}

/// A fixed window can roll over in the middle of a test, so the refusal is
/// not expected at one exact request: the first `limit` requests pass, and
/// one of the next `limit + 1` is refused.
async fn assert_limited(
    app: &Router,
    path: &str,
    limit: usize,
    passes_with: StatusCode,
    request: impl Fn(usize) -> (String, Value),
) {
    for attempt in 0..limit {
        let (address, body) = request(attempt);
        let (status, _) = send(app, Method::POST, path, None, Some(&address), Some(body)).await;
        assert_eq!(status, passes_with, "attempt {attempt}");
    }
    let mut refused = false;
    for attempt in limit..=limit * 2 {
        let (address, body) = request(attempt);
        let (status, error) = send(app, Method::POST, path, None, Some(&address), Some(body)).await;
        if status == StatusCode::TOO_MANY_REQUESTS {
            assert_eq!(
                error["error"],
                "too many attempts, try again in a few minutes"
            );
            refused = true;
            break;
        }
    }
    assert!(refused, "{path} was never limited");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_forgotten_password_is_reset_once_through_the_emailed_link(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;

    // An unknown address gets the same answer and no email.
    let (status, unknown) = post(&app, FORGOT, json!({ "email": "nobody@example.com" })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, known) = post(&app, FORGOT, json!({ "email": " Ada@Example.com " })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(known, unknown);
    let queued = sqlx::query_scalar!(
        r#"SELECT template || ' -> ' || email AS "line!" FROM email_queue
           WHERE booking_unid IS NULL"#
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(queued, ["password_reset -> ada@example.com"]);
    let tokens = reset_tokens(&pool, "ada@example.com").await;
    let token = tokens[0].as_str();
    // The table keeps a hash, never the token.
    let stored = sqlx::query_scalar!("SELECT token_hash FROM password_resets")
        .fetch_all(&pool)
        .await
        .expect("resets are readable");
    assert_eq!(stored.len(), 1);
    assert_ne!(stored[0], token);

    for (body, why) in [
        (
            json!({ "token": "nope", "password": NEW_PASSWORD }),
            "unknown",
        ),
        (json!({ "token": token, "password": "short" }), "weak"),
    ] {
        let (status, _) = post(&app, RESET, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{why}");
    }
    assert_eq!(
        login(&app, "ada@example.com", PASSWORD).await,
        StatusCode::OK
    );

    let (status, _) = post(
        &app,
        RESET,
        json!({ "token": token, "password": NEW_PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        login(&app, "ada@example.com", NEW_PASSWORD).await,
        StatusCode::OK
    );
    assert_eq!(
        login(&app, "ada@example.com", PASSWORD).await,
        StatusCode::UNAUTHORIZED
    );

    let (status, error) = post(
        &app,
        RESET,
        json!({ "token": token, "password": "yet-another-password" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "used");
    assert_eq!(
        error["error"],
        "invalid input: this reset link is no longer valid"
    );
    assert_eq!(
        login(&app, "ada@example.com", NEW_PASSWORD).await,
        StatusCode::OK
    );

    // A banned account gets the same answer and no link.
    sqlx::query_scalar!(
        "UPDATE users SET status = 'Banned' WHERE email = 'ada@example.com' RETURNING 1"
    )
    .fetch_one(&pool)
    .await
    .expect("the user exists");
    let (status, banned) = post(&app, FORGOT, json!({ "email": "ada@example.com" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(banned, unknown);
    assert_eq!(reset_tokens(&pool, "ada@example.com").await.len(), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_expired_or_replaced_reset_link_does_not_work(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    let ask = || post(&app, FORGOT, json!({ "email": "ada@example.com" }));
    let reset = |token: String| {
        post(
            &app,
            RESET,
            json!({ "token": token, "password": NEW_PASSWORD }),
        )
    };

    ask().await;
    let expired = reset_tokens(&pool, "ada@example.com").await.remove(0);
    sqlx::query_scalar!(
        "UPDATE password_resets SET expires_at = now() - interval '1 second' RETURNING 1"
    )
    .fetch_one(&pool)
    .await
    .expect("the reset exists");
    let (status, _) = reset(expired).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "expired");
    assert_eq!(
        login(&app, "ada@example.com", PASSWORD).await,
        StatusCode::OK
    );

    // Using one link switches off the others still waiting.
    ask().await;
    ask().await;
    let mut tokens = reset_tokens(&pool, "ada@example.com").await;
    let (newest, older) = (
        tokens.pop().expect("a third link"),
        tokens.pop().expect("a second link"),
    );
    let (status, _) = reset(newest).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = reset(older).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "replaced");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_user_changes_their_own_name(pool: PgPool) {
    let app = app(&pool);
    let cookie = register(&app, "Ada Lovelace", "ada@example.com").await;
    let rename = |name: Value| {
        send(
            &app,
            Method::PUT,
            ME,
            Some(&cookie),
            None,
            Some(json!({ "full_name": name })),
        )
    };

    let (status, _) = send(
        &app,
        Method::PUT,
        ME,
        None,
        None,
        Some(json!({ "full_name": "Nobody" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    for bad in [json!("   "), json!("a".repeat(121))] {
        let (status, _) = rename(bad.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }

    let (status, user) = rename(json!("  Ada King  ")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(user["full_name"], "Ada King");
    assert_eq!(user["email"], "ada@example.com");
    let (status, me) = send(&app, Method::GET, ME, Some(&cookie), None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me, user);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_user_changes_their_password_with_the_current_one(pool: PgPool) {
    let app = app(&pool);
    let cookie = register(&app, "Ada Lovelace", "ada@example.com").await;
    let change = |current: &str, new: &str| {
        send(
            &app,
            Method::POST,
            CHANGE_PASSWORD,
            Some(&cookie),
            None,
            Some(json!({ "current_password": current, "new_password": new })),
        )
    };

    let (status, error) = change("not-my-password", NEW_PASSWORD).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "invalid input: current password is wrong");
    let (status, _) = change(PASSWORD, "short").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Neither refusal ends the session or changes the password.
    let (status, _) = send(&app, Method::GET, ME, Some(&cookie), None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        login(&app, "ada@example.com", PASSWORD).await,
        StatusCode::OK
    );

    let (status, _) = change(PASSWORD, NEW_PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        login(&app, "ada@example.com", NEW_PASSWORD).await,
        StatusCode::OK
    );
    assert_eq!(
        login(&app, "ada@example.com", PASSWORD).await,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn logins_are_limited_per_email_and_per_address(pool: PgPool) {
    let app = app(&pool);
    let wrong = |email: &str| json!({ "email": email, "password": "not-my-password" });

    // One email tried from many addresses.
    assert_limited(&app, LOGIN, 10, StatusCode::UNAUTHORIZED, |attempt| {
        (format!("198.51.100.{attempt}"), wrong("Ada@example.com"))
    })
    .await;
    // Someone else, somewhere else, is not held up by it.
    let (status, _) = send(
        &app,
        Method::POST,
        LOGIN,
        None,
        Some("203.0.113.1"),
        Some(wrong("grace@example.com")),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // One address trying many emails.
    assert_limited(&app, LOGIN, 10, StatusCode::UNAUTHORIZED, |attempt| {
        (
            "203.0.113.2".to_owned(),
            wrong(&format!("user{attempt}@example.com")),
        )
    })
    .await;
}

#[sqlx::test(migrations = "../migrations")]
async fn reset_emails_and_sign_ups_are_limited(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;

    // Three links an hour for one inbox, wherever the requests come from.
    assert_limited(&app, FORGOT, 3, StatusCode::OK, |attempt| {
        (
            format!("198.51.100.{attempt}"),
            json!({ "email": "ada@example.com" }),
        )
    })
    .await;

    assert_limited(&app, FORGOT, 5, StatusCode::OK, |attempt| {
        (
            "203.0.113.3".to_owned(),
            json!({ "email": format!("user{attempt}@example.com") }),
        )
    })
    .await;

    assert_limited(
        &app,
        "/api/auth/register",
        5,
        StatusCode::CREATED,
        |attempt| {
            (
                "203.0.113.4".to_owned(),
                json!({
                    "email": format!("host{attempt}@example.com"),
                    "password": PASSWORD,
                    "full_name": "A Host",
                }),
            )
        },
    )
    .await;
}

#[sqlx::test(migrations = "../migrations")]
async fn old_rate_limit_windows_are_forgotten(pool: PgPool) {
    for age_hours in [1, 25] {
        sqlx::query_scalar!(
            "INSERT INTO rate_limits (action, subject, window_start)
             VALUES ('login_address', 'unknown', now() - make_interval(hours => $1)) RETURNING 1",
            age_hours,
        )
        .fetch_one(&pool)
        .await
        .expect("window is written");
    }
    rate_limits::forget_old(&pool)
        .await
        .expect("old windows are removed");
    // The development seed has windows of its own.
    let left = sqlx::query_scalar!("SELECT count(*) FROM rate_limits WHERE subject = 'unknown'")
        .fetch_one(&pool)
        .await
        .expect("windows are readable");
    assert_eq!(left, Some(1));
}
