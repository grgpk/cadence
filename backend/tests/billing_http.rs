//! Plans over HTTP: the public catalog, and the Pro trial every new host
//! starts on.

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
    domain::{
        auth::service::make_token,
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
    },
};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const PLANS: &str = "/api/billing/plans";
const ME: &str = "/api/billing/me";

fn app(pool: &PgPool) -> Router {
    dotenvy::dotenv().ok();
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
    })
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

async fn register(app: &Router, email: &str) -> (Uuid, String) {
    let (status, user) = send(
        app,
        Method::POST,
        "/api/auth/register",
        None,
        Some(json!({ "email": email, "password": "password123", "full_name": "Ada Lovelace" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let unid: Uuid = serde_json::from_value(user["unid"].clone()).expect("user has an id");
    let token = make_token(unid, 0).expect("JWT_SECRET is set for tests");
    (unid, format!("session={token}"))
}

#[sqlx::test(migrations = "../migrations")]
async fn the_catalog_is_public_and_lists_every_plan(pool: PgPool) {
    let app = app(&pool);
    let (status, plans) = send(&app, Method::GET, PLANS, None, None).await;

    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = plans
        .as_array()
        .expect("a list of plans")
        .iter()
        .filter_map(|offer| offer["plan"].as_str())
        .collect();
    assert_eq!(names, ["free", "pro", "business"]);
    assert_eq!(plans[0]["entitlements"]["max_bookings_per_month"], 25);
    assert_eq!(plans[1]["prices"][1]["monthly_cents"], 800);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_new_host_is_on_a_pro_trial(pool: PgPool) {
    let app = app(&pool);
    let (_, cookie) = register(&app, "ada@example.com").await;

    let (status, summary) = send(&app, Method::GET, ME, Some(&cookie), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["plan"], "pro");
    assert_eq!(summary["status"], "trialing");
    assert_eq!(summary["bookings_this_month"], 0);
    let ends: DateTime<Utc> =
        serde_json::from_value(summary["trial_ends_at"].clone()).expect("a trial end");
    let left = ends - Utc::now();
    assert!(left > Duration::days(13) && left <= Duration::days(14));
}

#[sqlx::test(migrations = "../migrations")]
async fn an_ended_trial_is_free(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    sqlx::query_scalar!(
        "UPDATE subscriptions SET trial_ends_at = now() - interval '1 minute'
         WHERE user_unid = $1 RETURNING 1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("the trial row exists");

    let (_, summary) = send(&app, Method::GET, ME, Some(&cookie), None).await;

    assert_eq!(summary["plan"], "free");
    assert_eq!(summary["status"], "trialing");
    assert_eq!(summary["entitlements"]["custom_questions"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn the_summary_needs_a_session(pool: PgPool) {
    let (status, _) = send(&app(&pool), Method::GET, ME, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
