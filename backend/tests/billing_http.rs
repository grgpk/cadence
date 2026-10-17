//! Plans over HTTP: the public catalog, the Pro trial every new host starts
//! on, checkout and the portal against a fake Stripe, and Stripe's events.

// Clippy's test escapes cover `#[test]` functions only; the helpers here are test code too.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use cadence_backend::domain::calendly::{client::Calendly, models::CalendlyAccount};
use cadence_backend::{
    app_state::AppState,
    build_app,
    domain::{
        auth::service::make_token,
        billing::{
            plans::{BillingInterval, Plan},
            stripe::{Credit, FAKE_UNPAID_INVOICE, Payments, PriceIds},
        },
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
    },
    signature::sign,
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
const YEARLY: &str = "/api/billing/yearly";
const WEBHOOK: &str = "/api/billing/webhook";
const WEBHOOK_SECRET: &str = "whsec_test";
const BUSINESS_YEAR: &str = "price_business_year";
const PRO_MONTH: &str = "price_pro_month";
const PRO_YEAR: &str = "price_pro_year";

fn app_with(pool: &PgPool, payments: Option<Payments>) -> Router {
    dotenvy::dotenv().ok();
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
        payments,
        calendly: Calendly::fake(CalendlyAccount::default()),
    })
}

fn app(pool: &PgPool) -> Router {
    app_with(pool, Some(fake_payments()))
}

fn fake_payments() -> Payments {
    let prices = PriceIds::new(vec![
        (Plan::Pro, BillingInterval::Month, PRO_MONTH.to_owned()),
        (Plan::Pro, BillingInterval::Year, PRO_YEAR.to_owned()),
        (
            Plan::Business,
            BillingInterval::Year,
            BUSINESS_YEAR.to_owned(),
        ),
    ]);
    Payments::fake(WEBHOOK_SECRET, prices)
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
    assert_eq!(plans[0]["entitlements"]["included_bookings_per_month"], 25);
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

#[sqlx::test(migrations = "../migrations")]
async fn a_monthly_payer_switches_to_yearly_on_stripes_confirm_screen(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;

    // On the trial there is nothing to switch yet.
    let (status, _) = send(&app, Method::POST, YEARLY, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::CONFLICT);

    subscription_event(&app, "evt_1", "created", 100, unid, PRO_MONTH, "active").await;
    let (status, switch) = send(&app, Method::POST, YEARLY, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        switch["url"],
        format!("https://billing.stripe.test/cus_1/sub_1/{PRO_YEAR}")
    );

    // Once yearly, the offer is spent.
    subscription_event(&app, "evt_2", "updated", 200, unid, BUSINESS_YEAR, "active").await;
    let (status, _) = send(&app, Method::POST, YEARLY, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

/// The host starts paying monthly Pro as Stripe customer `customer`.
async fn pays(app: &Router, event_id: &str, user: Uuid, customer: &str) {
    pays_invoice(app, event_id, user, customer, "in_paid").await;
}

/// The same, with the subscription's latest invoice named.
async fn pays_invoice(app: &Router, event_id: &str, user: Uuid, customer: &str, invoice: &str) {
    let event = json!({
        "id": event_id,
        "type": "customer.subscription.updated",
        "created": 100,
        "data": { "object": {
            "id": format!("sub_{customer}"),
            "customer": customer,
            "status": "active",
            "cancel_at_period_end": false,
            "metadata": { "user_unid": user.to_string() },
            "items": { "data": [{ "price": { "id": PRO_MONTH }, "current_period_end": 1_900_000_000 }] },
            "latest_invoice": invoice,
        }},
    });
    assert_eq!(
        webhook(app, &event.to_string(), WEBHOOK_SECRET).await,
        StatusCode::OK
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn an_invitation_credits_both_hosts_a_month_once_each_pays(pool: PgPool) {
    let payments = fake_payments();
    let app = app_with(&pool, Some(payments.clone()));
    let (ada, ada_cookie) = register(&app, "ada@example.com").await;
    let (_, settings) = send(
        &app,
        Method::GET,
        "/api/host/settings",
        Some(&ada_cookie),
        None,
    )
    .await;
    let code = settings["public_slug"].as_str().expect("a link").to_owned();

    let (status, ben) = send(
        &app,
        Method::POST,
        "/api/auth/register",
        None,
        Some(
            json!({ "email": "ben@example.com", "password": "password123",
                     "full_name": "Ben Ito", "referred_by": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ben: Uuid = serde_json::from_value(ben["unid"].clone()).expect("user has an id");
    // An address nobody has is ignored, not refused.
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/auth/register",
        None,
        Some(
            json!({ "email": "cy@example.com", "password": "password123",
                     "full_name": "Cy Lee", "referred_by": "nobody-here" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let month = |customer: &str| Credit {
        customer_id: customer.to_owned(),
        amount_cents: 1000,
    };
    // Ben pays: he is credited; Ada, still on her trial, waits.
    pays(&app, "evt_1", ben, "cus_ben").await;
    assert_eq!(payments.credits(), vec![month("cus_ben")]);
    let (_, referrals) = send(
        &app,
        Method::GET,
        "/api/billing/referrals",
        Some(&ada_cookie),
        None,
    )
    .await;
    assert_eq!(referrals["code"], code.as_str());
    assert_eq!(referrals["invited"], 1);
    assert_eq!(referrals["paying"], 1);

    // Ada pays: her month arrives. Later events credit nobody again.
    pays(&app, "evt_2", ada, "cus_ada").await;
    pays(&app, "evt_3", ben, "cus_ben").await;
    pays(&app, "evt_4", ada, "cus_ada").await;
    assert_eq!(payments.credits(), vec![month("cus_ben"), month("cus_ada")]);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_ended_subscription_ends_the_plan_even_with_a_price_nobody_knows(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    subscription_event(&app, "evt_1", "created", 100, unid, PRO_MONTH, "active").await;

    let status = subscription_event(
        &app,
        "evt_2",
        "deleted",
        200,
        unid,
        "price_untagged",
        "canceled",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let ended = summary(&app, &cookie).await;
    assert_eq!(ended["plan"], "free");
    assert_eq!(ended["status"], "canceled");
}

#[sqlx::test(migrations = "../migrations")]
async fn events_from_one_second_in_any_order_leave_stripes_latest_state(pool: PgPool) {
    let payments = fake_payments();
    let app = app_with(&pool, Some(payments.clone()));
    let (unid, cookie) = register(&app, "ada@example.com").await;
    // Stripe has the subscription active now.
    payments.fake_subscription(
        "sub_1",
        json!({
            "id": "sub_1", "customer": "cus_1", "status": "active", "cancel_at_period_end": false,
            "metadata": { "user_unid": unid.to_string() },
            "items": { "data": [{ "price": { "id": PRO_MONTH }, "current_period_end": 1_900_000_000 }] },
        }),
    );

    // Both events are from second 100; the "incomplete" one arrives last.
    subscription_event(&app, "evt_2", "updated", 100, unid, PRO_MONTH, "active").await;
    subscription_event(&app, "evt_1", "created", 100, unid, PRO_MONTH, "incomplete").await;

    let now = summary(&app, &cookie).await;
    assert_eq!(now["status"], "active");
    assert_eq!(now["plan"], "pro");
}

/// Ada invites Ben; returns their ids and Ada's cookie.
async fn invitation(app: &Router) -> (Uuid, String, Uuid) {
    let (ada, ada_cookie) = register(app, "ada@example.com").await;
    let (_, settings) = send(
        app,
        Method::GET,
        "/api/host/settings",
        Some(&ada_cookie),
        None,
    )
    .await;
    let code = settings["public_slug"].as_str().expect("a link").to_owned();
    let (status, ben) = send(
        app,
        Method::POST,
        "/api/auth/register",
        None,
        Some(
            json!({ "email": "ben@example.com", "password": "password123",
                     "full_name": "Ben Ito", "referred_by": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ben = serde_json::from_value(ben["unid"].clone()).expect("user has an id");
    (ada, ada_cookie, ben)
}

#[sqlx::test(migrations = "../migrations")]
async fn an_invitation_pays_out_only_after_real_money(pool: PgPool) {
    let payments = fake_payments();
    let app = app_with(&pool, Some(payments.clone()));
    let (_, _, ben) = invitation(&app).await;

    // A promotion code made Ben's first invoice $0: no credit yet.
    pays_invoice(&app, "evt_1", ben, "cus_ben", FAKE_UNPAID_INVOICE).await;
    assert!(payments.credits().is_empty());

    // His next invoice takes money.
    pays_invoice(&app, "evt_2", ben, "cus_ben", "in_second").await;
    assert_eq!(payments.credits().len(), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn stripe_refusing_a_credit_never_holds_up_the_plan(pool: PgPool) {
    let payments = fake_payments();
    let app = app_with(&pool, Some(payments.clone()));
    let (_, _, ben) = invitation(&app).await;
    let ben_cookie = format!("session={}", make_token(ben, 0).expect("JWT_SECRET is set"));

    payments.fail_credits(true);
    pays(&app, "evt_1", ben, "cus_ben").await;
    assert_eq!(summary(&app, &ben_cookie).await["status"], "active");
    assert!(payments.credits().is_empty());

    // The next event, with Stripe back, makes the credit once.
    payments.fail_credits(false);
    pays(&app, "evt_2", ben, "cus_ben").await;
    pays(&app, "evt_3", ben, "cus_ben").await;
    assert_eq!(payments.credits().len(), 1);
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

async fn grant_admin(pool: &PgPool, user: Uuid) {
    sqlx::query_scalar!(
        "INSERT INTO roleaccesses (grantedto_unid, role) VALUES ($1, 'Admin') RETURNING 1",
        user,
    )
    .fetch_one(pool)
    .await
    .expect("role is granted");
}

async fn comp(app: &Router, cookie: &str, user: Uuid, plan: Value) -> StatusCode {
    let path = format!("/api/admin/billing/{user}");
    send(
        app,
        Method::PUT,
        &path,
        Some(cookie),
        Some(json!({ "plan": plan })),
    )
    .await
    .0
}

#[sqlx::test(migrations = "../migrations")]
async fn an_admin_sees_every_hosts_plan_and_comps_one(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, host_cookie) = register(&app, "ada@example.com").await;

    // Hosts cannot read or change plans.
    let (status, _) = send(
        &app,
        Method::GET,
        "/api/admin/billing",
        Some(&host_cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        comp(&app, &host_cookie, host, json!("business")).await,
        StatusCode::FORBIDDEN
    );
    // Only hosts have plans.
    let not_a_host = sqlx::query_scalar!(
        "INSERT INTO users (email, password_hash, full_name) VALUES ('ops@example.com', 'x', 'Ops')
         RETURNING unid",
    )
    .fetch_one(&pool)
    .await
    .expect("a user without the host role");
    assert_eq!(
        comp(&app, &admin_cookie, not_a_host, json!("business")).await,
        StatusCode::BAD_REQUEST
    );

    let (status, plans) = send(
        &app,
        Method::GET,
        "/api/admin/billing",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let row = |plans: &Value| {
        plans
            .as_array()
            .expect("a list")
            .iter()
            .find(|row| row["email"] == "ada@example.com")
            .cloned()
            .expect("the host is listed")
    };
    assert_eq!(row(&plans)["plan"], "pro");
    assert_eq!(row(&plans)["status"], "trialing");

    assert_eq!(
        comp(&app, &admin_cookie, host, json!("business")).await,
        StatusCode::NO_CONTENT
    );
    let (_, plans) = send(
        &app,
        Method::GET,
        "/api/admin/billing",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(row(&plans)["plan"], "business");
    assert_eq!(row(&plans)["status"], "comped");
    let (_, summary) = send(&app, Method::GET, ME, Some(&host_cookie), None).await;
    assert_eq!(summary["plan"], "business");

    assert_eq!(
        comp(&app, &admin_cookie, host, json!("free")).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        comp(&app, &admin_cookie, host, Value::Null).await,
        StatusCode::NO_CONTENT
    );
    let (_, summary) = send(&app, Method::GET, ME, Some(&host_cookie), None).await;
    assert_eq!(summary["plan"], "free");
    assert_eq!(summary["status"], Value::Null);
    assert_eq!(
        comp(&app, &admin_cookie, host, Value::Null).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        comp(&app, &admin_cookie, Uuid::new_v4(), json!("pro")).await,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_plan_stripe_bills_is_not_changed_by_an_admin(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, _) = register(&app, "ada@example.com").await;
    subscription_event(&app, "evt_1", "created", 100, host, PRO_MONTH, "active").await;

    assert_eq!(
        comp(&app, &admin_cookie, host, json!("business")).await,
        StatusCode::CONFLICT
    );
}
