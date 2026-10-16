//! Plans over HTTP: the public catalog, the Pro trial every new host starts
//! on, checkout and the portal against a fake Stripe, and Stripe's events.

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
        billing::{
            plans::{BillingInterval, Plan},
            stripe::{Payments, PriceIds, sign},
        },
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
const CHECKOUT: &str = "/api/billing/checkout";
const PORTAL: &str = "/api/billing/portal";
const WEBHOOK: &str = "/api/billing/webhook";
const WEBHOOK_SECRET: &str = "whsec_test";
const BUSINESS_YEAR: &str = "price_business_year";
const PRO_MONTH: &str = "price_pro_month";

fn app_with(pool: &PgPool, payments: Option<Payments>) -> Router {
    dotenvy::dotenv().ok();
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
        payments,
    })
}

fn app(pool: &PgPool) -> Router {
    let prices = PriceIds::new(vec![
        (Plan::Pro, BillingInterval::Month, PRO_MONTH.to_owned()),
        (
            Plan::Business,
            BillingInterval::Year,
            BUSINESS_YEAR.to_owned(),
        ),
    ]);
    app_with(pool, Some(Payments::fake(WEBHOOK_SECRET, prices)))
}

/// Posts a `customer.subscription.*` event, signed now.
async fn subscription_event(
    app: &Router,
    event_id: &str,
    kind: &str,
    created: i64,
    user: Uuid,
    price: &str,
    status: &str,
) -> StatusCode {
    let event = json!({
        "id": event_id,
        "type": format!("customer.subscription.{kind}"),
        "created": created,
        "data": { "object": {
            "id": "sub_1",
            "customer": "cus_1",
            "status": status,
            "cancel_at_period_end": false,
            "metadata": { "user_unid": user.to_string() },
            "items": { "data": [{ "price": { "id": price }, "current_period_end": 1_900_000_000 }] },
        }},
    });
    webhook(app, &event.to_string(), WEBHOOK_SECRET).await
}

async fn webhook(app: &Router, body: &str, secret: &str) -> StatusCode {
    let request = Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK)
        .header(
            "stripe-signature",
            sign(secret, body.as_bytes(), Utc::now()),
        )
        .body(Body::from(body.to_owned()))
        .expect("request is well formed");
    app.clone()
        .oneshot(request)
        .await
        .expect("router responds")
        .status()
}

async fn summary(app: &Router, cookie: &str) -> Value {
    send(app, Method::GET, ME, Some(cookie), None).await.1
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

#[sqlx::test(migrations = "../migrations")]
async fn checkout_sends_the_host_to_stripe_for_a_paid_plan(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;

    let (status, body) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(json!({ "plan": "business", "interval": "year" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["url"],
        format!("https://checkout.stripe.test/{unid}/{BUSINESS_YEAR}")
    );

    let (status, _) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(json!({ "plan": "free", "interval": "month" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../migrations")]
async fn without_stripe_keys_checkout_is_unavailable(pool: PgPool) {
    let app = app_with(&pool, None);
    let (_, cookie) = register(&app, "ada@example.com").await;
    let body = json!({ "plan": "pro", "interval": "month" });

    let (status, _) = send(&app, Method::POST, CHECKOUT, Some(&cookie), Some(body)).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_event_with_a_wrong_signature_is_refused(pool: PgPool) {
    let app = app(&pool);
    let (_, cookie) = register(&app, "ada@example.com").await;
    let body = json!({ "id": "evt_1", "type": "customer.subscription.created", "created": 1,
                       "data": { "object": {} } })
    .to_string();

    assert_eq!(
        webhook(&app, &body, "whsec_wrong").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(summary(&app, &cookie).await["status"], "trialing");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_paid_subscription_sets_the_plan_until_it_ends(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;

    let status =
        subscription_event(&app, "evt_1", "created", 100, unid, BUSINESS_YEAR, "active").await;
    assert_eq!(status, StatusCode::OK);
    let paid = summary(&app, &cookie).await;
    assert_eq!(paid["plan"], "business");
    assert_eq!(paid["status"], "active");
    assert_eq!(paid["interval"], "year");
    assert!(paid["current_period_end"].is_string());

    // Paying hosts change plans in the portal, not with a second checkout.
    let body = json!({ "plan": "pro", "interval": "month" });
    let (status, _) = send(&app, Method::POST, CHECKOUT, Some(&cookie), Some(body)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, portal) = send(&app, Method::POST, PORTAL, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(portal["url"], "https://billing.stripe.test/cus_1");

    subscription_event(
        &app,
        "evt_2",
        "deleted",
        200,
        unid,
        BUSINESS_YEAR,
        "canceled",
    )
    .await;
    let ended = summary(&app, &cookie).await;
    assert_eq!(ended["plan"], "free");
    assert_eq!(ended["status"], "canceled");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_repeated_or_late_event_changes_nothing(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    subscription_event(&app, "evt_1", "updated", 200, unid, PRO_MONTH, "active").await;

    // Stripe sent the same event again, this time saying something else.
    let status =
        subscription_event(&app, "evt_1", "updated", 200, unid, PRO_MONTH, "past_due").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary(&app, &cookie).await["status"], "active");

    // An older event arrives after a newer one.
    subscription_event(&app, "evt_0", "created", 100, unid, BUSINESS_YEAR, "active").await;
    assert_eq!(summary(&app, &cookie).await["plan"], "pro");
}

#[sqlx::test(migrations = "../migrations")]
async fn the_portal_needs_a_stripe_customer(pool: PgPool) {
    let app = app(&pool);
    let (_, cookie) = register(&app, "ada@example.com").await;

    let (status, _) = send(&app, Method::POST, PORTAL, Some(&cookie), None).await;

    assert_eq!(status, StatusCode::CONFLICT);
}

async fn waiting_notices(pool: &PgPool, user: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT e.template AS "template!" FROM email_queue e JOIN users u ON u.email = e.email
           WHERE u.unid = $1 AND e.booking_unid IS NULL AND e.sent_at IS NULL
           ORDER BY e.template"#,
        user,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
}

#[sqlx::test(migrations = "../migrations")]
async fn paying_takes_back_the_trial_notice_and_a_failed_renewal_is_told_once(pool: PgPool) {
    let app = app(&pool);
    let (unid, _) = register(&app, "ada@example.com").await;
    assert_eq!(waiting_notices(&pool, unid).await, ["trial_ending"]);

    subscription_event(&app, "evt_1", "created", 100, unid, PRO_MONTH, "active").await;
    assert!(waiting_notices(&pool, unid).await.is_empty());

    subscription_event(&app, "evt_2", "updated", 200, unid, PRO_MONTH, "past_due").await;
    subscription_event(&app, "evt_3", "updated", 300, unid, PRO_MONTH, "past_due").await;
    assert_eq!(waiting_notices(&pool, unid).await, ["payment_failed"]);
}
