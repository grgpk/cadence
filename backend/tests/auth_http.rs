//! Accounts over HTTP: password reset by email, profile and password changes,
//! sessions that end when the password changes, the limits that slow down
//! guessing and spam, and the development seed staying out of production.

// Clippy's test escapes cover `#[test]` functions only; the helpers here are test code too.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use cadence_backend::domain::calendly::{client::Calendly, models::CalendlyAccount};
use cadence_backend::{
    app_state::AppState,
    build_app,
    domain::{
        auth::service::make_token,
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
        rate_limits::service as rate_limits,
    },
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
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
        payments: None,
        calendly: Calendly::fake(CalendlyAccount::default()),
    })
}

/// `address` is sent as `x-forwarded-for`; without it the request has no
/// client address at all.
async fn respond(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    address: Option<&str>,
    body: Option<Value>,
) -> Response {
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
    app.clone().oneshot(request).await.expect("router responds")
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    address: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let response = respond(app, method, path, cookie, address, body).await;
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
    let token = make_token(unid, 0).expect("JWT_SECRET is set for tests");
    format!("session={token}")
}

/// The session cookie a response sets, as a browser would send it back.
fn session_cookie(response: &Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .filter(|pair| pair.starts_with("session="))
        .expect("the response sets a session cookie")
        .to_owned()
}

/// Logs in through the endpoint, as another device would.
async fn login_session(app: &Router, email: &str, password: &str) -> String {
    let response = respond(
        app,
        Method::POST,
        LOGIN,
        None,
        None,
        Some(json!({ "email": email, "password": password })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    session_cookie(&response)
}

async fn me_status(app: &Router, cookie: &str) -> StatusCode {
    send(app, Method::GET, ME, Some(cookie), None, None).await.0
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
        // Registering also queued the trial notice, weeks ahead.
        r#"SELECT template || ' -> ' || email AS "line!" FROM email_queue
           WHERE booking_unid IS NULL AND template <> 'trial_ending'"#
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
async fn logins_are_limited_per_address_without_locking_the_owner_out(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    let wrong = |email: &str| json!({ "email": email, "password": "not-my-password" });

    // Someone guessing one account's password from one address.
    // Five tries, so this stops them before the address limit of ten would.
    assert_limited(&app, LOGIN, 5, StatusCode::UNAUTHORIZED, |_| {
        ("198.51.100.1".to_owned(), wrong("Ada@example.com"))
    })
    .await;
    // The account's owner, somewhere else, still gets in.
    let (status, _) = send(
        &app,
        Method::POST,
        LOGIN,
        None,
        Some("203.0.113.1"),
        Some(json!({ "email": "ada@example.com", "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // The limit counts the email and the address together, never the email alone.
    let subjects = sqlx::query_scalar!(
        "SELECT DISTINCT subject FROM rate_limits WHERE action = 'login_email_address' ORDER BY subject"
    )
    .fetch_all(&pool)
    .await
    .expect("windows are readable");
    assert_eq!(
        subjects,
        [
            "198.51.100.1|ada@example.com",
            "203.0.113.1|ada@example.com"
        ]
    );

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

    // Guessing reset tokens from one address.
    assert_limited(&app, RESET, 10, StatusCode::BAD_REQUEST, |attempt| {
        (
            "203.0.113.5".to_owned(),
            json!({ "token": format!("guess-{attempt}"), "password": NEW_PASSWORD }),
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
    let left = sqlx::query_scalar!("SELECT count(*) FROM rate_limits WHERE subject = 'unknown'")
        .fetch_one(&pool)
        .await
        .expect("windows are readable");
    assert_eq!(left, Some(1));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_password_change_signs_out_other_devices_and_keeps_this_one(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    let this_device = login_session(&app, "ada@example.com", PASSWORD).await;
    let other_device = login_session(&app, "ada@example.com", PASSWORD).await;
    assert_eq!(me_status(&app, &other_device).await, StatusCode::OK);

    let response = respond(
        &app,
        Method::POST,
        CHANGE_PASSWORD,
        Some(&this_device),
        None,
        Some(json!({ "current_password": PASSWORD, "new_password": NEW_PASSWORD })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let renewed = session_cookie(&response);
    assert_ne!(renewed, this_device);

    assert_eq!(me_status(&app, &renewed).await, StatusCode::OK);
    for (old, which) in [
        (&other_device, "other device"),
        (&this_device, "old cookie"),
    ] {
        let (status, error) = send(&app, Method::GET, ME, Some(old), None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{which}");
        assert_eq!(error["error"], "invalid session", "{which}");
    }
    // A new login starts from the new version.
    let fresh = login_session(&app, "ada@example.com", NEW_PASSWORD).await;
    assert_eq!(me_status(&app, &fresh).await, StatusCode::OK);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_password_reset_signs_out_every_session(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    let session = login_session(&app, "ada@example.com", PASSWORD).await;
    assert_eq!(me_status(&app, &session).await, StatusCode::OK);

    post(&app, FORGOT, json!({ "email": "ada@example.com" })).await;
    let token = reset_tokens(&pool, "ada@example.com").await.remove(0);
    let response = respond(
        &app,
        Method::POST,
        RESET,
        None,
        None,
        Some(json!({ "token": token, "password": NEW_PASSWORD })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers().get(header::SET_COOKIE).is_none(),
        "a reset does not log in"
    );

    assert_eq!(me_status(&app, &session).await, StatusCode::UNAUTHORIZED);
    let fresh = login_session(&app, "ada@example.com", NEW_PASSWORD).await;
    assert_eq!(me_status(&app, &fresh).await, StatusCode::OK);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_session_from_before_versions_works_until_the_password_changes(pool: PgPool) {
    let app = app(&pool);
    let cookie = register(&app, "Ada Lovelace", "ada@example.com").await;
    let unid = sqlx::query_scalar!("SELECT unid FROM users WHERE email = 'ada@example.com'")
        .fetch_one(&pool)
        .await
        .expect("the user exists");
    // A token as it was issued before the version claim existed.
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is set for tests");
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &json!({ "sub": unid, "exp": chrono::Utc::now().timestamp() + 3600 }),
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("token is signed");
    let old_session = format!("session={token}");
    assert_eq!(me_status(&app, &old_session).await, StatusCode::OK);

    let (status, _) = send(
        &app,
        Method::POST,
        CHANGE_PASSWORD,
        Some(&cookie),
        None,
        Some(json!({ "current_password": PASSWORD, "new_password": NEW_PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        me_status(&app, &old_session).await,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_banned_user_is_refused_like_a_wrong_password(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    let attempt = |password: &'static str| {
        post(
            &app,
            LOGIN,
            json!({ "email": "ada@example.com", "password": password }),
        )
    };
    let wrong = attempt("not-my-password").await;
    assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);

    sqlx::query_scalar!(
        "UPDATE users SET status = 'Banned' WHERE email = 'ada@example.com' RETURNING 1"
    )
    .fetch_one(&pool)
    .await
    .expect("the user exists");
    assert_eq!(attempt(PASSWORD).await, wrong);
    // An address without an account answers the same too.
    let unknown = post(
        &app,
        LOGIN,
        json!({ "email": "nobody@example.com", "password": PASSWORD }),
    )
    .await;
    assert_eq!(unknown, wrong);
}

#[sqlx::test(migrations = "../migrations")]
async fn one_email_registers_once_in_any_letter_case(pool: PgPool) {
    let app = app(&pool);
    let sign_up = |email: &'static str| {
        post(
            &app,
            "/api/auth/register",
            json!({ "email": email, "password": PASSWORD, "full_name": "Ada Lovelace" }),
        )
    };

    // Two sign-ups at once: both pass the "is it taken" read, one insert wins.
    let (first, second) = tokio::join!(sign_up("ada@example.com"), sign_up(" Ada@Example.com "));
    let mut statuses = [first.0, second.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    let refused = if first.0 == StatusCode::CONFLICT {
        first.1
    } else {
        second.1
    };
    assert_eq!(refused["error"], "conflict: email already registered");

    let (status, _) = sign_up("ADA@EXAMPLE.COM").await;
    assert_eq!(status, StatusCode::CONFLICT);
    let stored = sqlx::query_scalar!("SELECT email FROM users")
        .fetch_all(&pool)
        .await
        .expect("users are readable");
    assert_eq!(stored, ["ada@example.com"]);

    // From another address, so the sign-up limit above stays out of the way.
    for bad in ["ada", "ada@example", "ada @example.com", "@example.com"] {
        let (status, error) = send(
            &app,
            Method::POST,
            "/api/auth/register",
            None,
            Some("203.0.113.6"),
            Some(json!({ "email": bad, "password": PASSWORD, "full_name": "Ada Lovelace" })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        assert_eq!(error["error"], "invalid input: valid email required");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn login_ignores_the_letter_case_of_the_email(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    assert_eq!(
        login(&app, " ADA@Example.com ", PASSWORD).await,
        StatusCode::OK
    );

    // A row written before emails were stored in lower case.
    sqlx::query_scalar!("UPDATE users SET email = 'Ada@Example.com' RETURNING 1")
        .fetch_one(&pool)
        .await
        .expect("the user exists");
    assert_eq!(
        login(&app, "ada@example.com", PASSWORD).await,
        StatusCode::OK
    );
}

const SEED: &str = include_str!("../../seeds/dev_seed.sql");

#[test]
fn the_development_seed_is_not_a_migration() {
    let migrations = concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations");
    let names: Vec<String> = std::fs::read_dir(migrations)
        .expect("migrations are readable")
        .map(|entry| {
            entry
                .expect("entry is readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(names.len() > 10, "{names:?}");
    for name in &names {
        assert!(!name.starts_with("9999"), "{name}");
        assert!(!name.contains("seed_data"), "{name}");
    }
}

/// Seeded logins as "email status", and how many rows the seeded tables hold.
async fn seed_state(pool: &PgPool) -> (Vec<String>, Option<i64>) {
    let logins = sqlx::query_scalar!(
        r#"SELECT email || ' ' || status AS "line!" FROM users ORDER BY email"#
    )
    .fetch_all(pool)
    .await
    .expect("users are readable");
    let rows = sqlx::query_scalar!(
        "SELECT (SELECT count(*) FROM users) + (SELECT count(*) FROM roleaccesses)
              + (SELECT count(*) FROM widget_settings) + (SELECT count(*) FROM availability)
              + (SELECT count(*) FROM calling_visits) + (SELECT count(*) FROM leads)
              + (SELECT count(*) FROM bookings) + (SELECT count(*) FROM email_queue)
              + (SELECT count(*) FROM google_calendar_config) + (SELECT count(*) FROM oauth_states)
              + (SELECT count(*) FROM bug_reports) + (SELECT count(*) FROM audit_logs)
              + (SELECT count(*) FROM rate_limits WHERE action IN ('login', 'calling_visit'))"
    )
    .fetch_one(pool)
    .await
    .expect("tables are readable");
    (logins, rows)
}

#[sqlx::test(migrations = "../migrations")]
async fn the_development_seed_applies_after_migrations_and_again(pool: PgPool) {
    let app = app(&pool);
    let seed = || async {
        sqlx::raw_sql(SEED)
            .execute(&pool)
            .await
            .expect("the seed applies");
    };
    let active = [
        "admin@example.com Active",
        "host@example.com Active",
        "root@example.com Active",
    ];

    assert_eq!(
        seed_state(&pool).await.0,
        [""; 0],
        "no seed after migrations"
    );
    seed().await;
    let seeded = seed_state(&pool).await;
    assert_eq!(seeded.0, active);
    assert_eq!(
        login(&app, "root@example.com", "password").await,
        StatusCode::OK
    );
    // Running it again adds nothing.
    seed().await;
    assert_eq!(seed_state(&pool).await, seeded);

    // Locally, the seed switches its logins back on.
    sqlx::query_scalar!("UPDATE users SET status = 'Banned' RETURNING 1")
        .fetch_all(&pool)
        .await
        .expect("users are writable");
    assert_eq!(
        login(&app, "root@example.com", "password").await,
        StatusCode::UNAUTHORIZED
    );
    seed().await;
    assert_eq!(seed_state(&pool).await, seeded);
    assert_eq!(
        login(&app, "root@example.com", "password").await,
        StatusCode::OK
    );
}
