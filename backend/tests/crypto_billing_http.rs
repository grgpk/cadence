//! Plans paid for in crypto over HTTP, against a fake provider: the opt-in
//! checkout, the provider's events, and how a prepaid period sits next to
//! Stripe's plan without touching it.

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
        calendly::{client::Calendly, models::CalendlyAccount},
        crypto_billing::{
            gateway::{CryptoPayments, FAKE_SECRET_HEADER, PaymentStatus},
            worker::reconcile,
        },
        emails::{
            service::RenderedEmail,
            worker::{Mailer, process_due},
        },
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
    },
};
use chrono::{DateTime, Duration, Months, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const ME: &str = "/api/billing/me";
const CRYPTO: &str = "/api/billing/crypto";
const CHECKOUT: &str = "/api/billing/crypto/checkout";
const WEBHOOK: &str = "/api/billing/crypto/webhook";
const SECRET: &str = "crypto_test_secret";

fn app_with(pool: &PgPool, crypto: Option<CryptoPayments>) -> Router {
    dotenvy::dotenv().ok();
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
        payments: None,
        crypto,
        calendly: Calendly::fake(CalendlyAccount::default()),
    })
}

fn app(pool: &PgPool) -> Router {
    app_with(pool, Some(CryptoPayments::fake(SECRET)))
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

/// Ends the host's trial, so the plan they have is only what they pay for.
async fn end_trial(pool: &PgPool, user: Uuid) {
    sqlx::query_scalar!(
        "UPDATE subscriptions SET trial_ends_at = now() - interval '1 day' WHERE user_unid = $1 RETURNING 1",
        user,
    )
    .fetch_one(pool)
    .await
    .expect("the trial row exists");
}

/// Opens a crypto checkout and answers the order id the provider was given.
async fn checkout(app: &Router, cookie: &str, plan: &str, interval: &str) -> Uuid {
    let (status, body) = send(
        app,
        Method::POST,
        CHECKOUT,
        Some(cookie),
        Some(json!({ "plan": plan, "interval": interval })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let url = body["url"].as_str().expect("a checkout url");
    url.rsplit('/')
        .next()
        .and_then(|order| order.parse().ok())
        .expect("the fake's url ends with the order id")
}

/// The provider's event about `order`'s invoice.
async fn event(app: &Router, pool: &PgPool, order: Uuid, status: &str, secret: &str) -> StatusCode {
    let invoice = sqlx::query_scalar!(
        "SELECT provider_invoice_id FROM crypto_payments WHERE unid = $1",
        order,
    )
    .fetch_optional(pool)
    .await
    .expect("payments are readable")
    .flatten()
    .unwrap_or_default();
    event_for(app, order, &invoice, status, secret).await
}

async fn event_for(
    app: &Router,
    order: Uuid,
    invoice: &str,
    status: &str,
    secret: &str,
) -> StatusCode {
    let request = Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK)
        .header(FAKE_SECRET_HEADER, secret)
        .body(Body::from(
            json!({ "order_id": order, "invoice_id": invoice, "status": status }).to_string(),
        ))
        .expect("request is well formed");
    app.clone()
        .oneshot(request)
        .await
        .expect("router responds")
        .status()
}

async fn period_end(app: &Router, cookie: &str) -> Option<DateTime<Utc>> {
    let (_, crypto) = send(app, Method::GET, CRYPTO, Some(cookie), None).await;
    serde_json::from_value(crypto["active"]["period_end"].clone()).ok()
}

/// A notice still to go out to the host.
struct Notice {
    template: String,
    send_at: DateTime<Utc>,
}

/// Notices still to go out, receipts aside: those go at once and have a test
/// of their own.
async fn waiting_notices(pool: &PgPool, user: Uuid) -> Vec<(String, DateTime<Utc>)> {
    sqlx::query_as!(
        Notice,
        r#"SELECT e.template AS "template!", e.send_at AS "send_at!"
           FROM email_queue e JOIN users u ON u.email = e.email
           WHERE u.unid = $1 AND e.booking_unid IS NULL AND e.sent_at IS NULL
             AND e.template <> 'crypto_receipt'
           ORDER BY e.template"#,
        user,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
    .into_iter()
    .map(|notice| (notice.template, notice.send_at))
    .collect()
}

fn near(left: DateTime<Utc>, right: DateTime<Utc>) -> bool {
    (left - right).abs() < Duration::minutes(1)
}

#[sqlx::test(migrations = "../migrations")]
async fn without_a_provider_crypto_is_off(pool: PgPool) {
    let app = app_with(&pool, None);
    let (_, cookie) = register(&app, "ada@example.com").await;

    let (status, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(crypto["available"], false);
    assert_eq!(crypto["active"], Value::Null);

    let (status, _) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(json!({ "plan": "pro", "interval": "month" })),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_paid_invoice_gives_the_plan_for_its_interval(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, unid).await;
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "free"
    );

    let order = checkout(&app, &cookie, "business", "month").await;
    assert_eq!(
        event(&app, &pool, order, "paid", SECRET).await,
        StatusCode::OK
    );

    let (_, summary) = send(&app, Method::GET, ME, Some(&cookie), None).await;
    assert_eq!(summary["plan"], "business");
    assert_eq!(summary["entitlements"]["webhooks"], true);
    // Stripe's row is untouched: still the ended trial.
    assert_eq!(summary["status"], "trialing");
    let end = period_end(&app, &cookie).await.expect("a crypto period");
    let expected = Utc::now()
        .checked_add_months(Months::new(1))
        .expect("in range");
    assert!(near(end, expected));
    let notices = waiting_notices(&pool, unid).await;
    assert_eq!(notices.len(), 2);
    assert_eq!(notices[0].0, "crypto_ended");
    assert!(near(notices[0].1, end));
    assert_eq!(notices[1].0, "crypto_renewal");
    assert!(near(notices[1].1, end - Duration::days(3)));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_repeated_event_counts_once_and_renewing_adds_to_the_period(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;

    let first = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, first, "paid", SECRET).await;
    let after_first = period_end(&app, &cookie).await.expect("a crypto period");
    event(&app, &pool, first, "paid", SECRET).await;
    assert_eq!(period_end(&app, &cookie).await, Some(after_first));

    let second = checkout(&app, &cookie, "pro", "year").await;
    event(&app, &pool, second, "paid", SECRET).await;
    let after_second = period_end(&app, &cookie).await.expect("a crypto period");
    let expected = after_first
        .checked_add_months(Months::new(12))
        .expect("in range");
    assert!(near(after_second, expected));
    // One reminder and one end notice, for the new end; the trial notice is gone.
    let notices = waiting_notices(&pool, unid).await;
    assert_eq!(notices.len(), 2);
    assert_eq!(notices[0].0, "crypto_ended");
    assert!(near(notices[0].1, after_second));
    assert_eq!(notices[1].0, "crypto_renewal");
    assert!(near(notices[1].1, after_second - Duration::days(3)));
}

#[sqlx::test(migrations = "../migrations")]
async fn an_unpaid_invoice_gives_nothing_and_a_seen_payment_is_confirming(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, unid).await;

    for status in ["expired", "underpaid", "failed"] {
        let order = checkout(&app, &cookie, "pro", "month").await;
        assert_eq!(
            event(&app, &pool, order, status, SECRET).await,
            StatusCode::OK
        );
        // An ended invoice never goes back to confirming.
        event(&app, &pool, order, "processing", SECRET).await;
    }
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "free"
    );
    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["confirming"], false);

    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "processing", SECRET).await;
    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["confirming"], true);
    assert_eq!(crypto["active"], Value::Null);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_event_with_a_wrong_secret_or_for_another_invoice_changes_nothing(pool: PgPool) {
    let app = app(&pool);
    let (_, cookie) = register(&app, "ada@example.com").await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    assert_eq!(
        event(&app, &pool, order, "paid", "wrong").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        event_for(&app, order, "inv_other", "paid", SECRET).await,
        StatusCode::OK
    );
    assert_eq!(
        event(&app, &pool, Uuid::new_v4(), "paid", SECRET).await,
        StatusCode::OK
    );
    assert_eq!(period_end(&app, &cookie).await, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn checkout_is_refused_for_free_or_while_stripe_bills(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    let pick = |plan: &str| json!({ "plan": plan, "interval": "month" });

    let (status, _) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(pick("free")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    let (status, _) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(pick("pro")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    sqlx::query_scalar!(
        "UPDATE subscriptions SET status = 'active', stripe_subscription_id = 'sub_1' WHERE user_unid = $1 RETURNING 1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("the trial row exists");
    let (status, _) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(pick("pro")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_ended_period_leaves_the_stripe_plan_or_free(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, unid).await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "pro"
    );

    sqlx::query_scalar!(
        "UPDATE crypto_payments SET period_end = now() - interval '1 minute' WHERE user_unid = $1 RETURNING 1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("the payment exists");
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "free"
    );
    assert_eq!(period_end(&app, &cookie).await, None);
}

/// Makes the payment look untouched for a while, as one whose events were lost.
async fn quiet(pool: &PgPool, order: Uuid) -> String {
    sqlx::query_scalar!(
        r#"UPDATE crypto_payments SET updated_at = now() - interval '10 minutes'
           WHERE unid = $1 RETURNING provider_invoice_id AS "invoice!""#,
        order,
    )
    .fetch_one(pool)
    .await
    .expect("the payment has an invoice")
}

#[sqlx::test(migrations = "../migrations")]
async fn a_paid_invoice_whose_events_were_lost_is_credited_on_the_next_pass(pool: PgPool) {
    let crypto = CryptoPayments::fake(SECRET);
    let app = app_with(&pool, Some(crypto.clone()));
    let (unid, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, unid).await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    let invoice = quiet(&pool, order).await;

    // Nothing new at the provider: nothing to apply.
    assert_eq!(
        reconcile(&pool, &crypto, Utc::now()).await.expect("a pass"),
        0
    );

    crypto.set_fake_status(&invoice, PaymentStatus::Paid);
    assert_eq!(
        reconcile(&pool, &crypto, Utc::now()).await.expect("a pass"),
        1
    );
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "pro"
    );
    let end = period_end(&app, &cookie).await.expect("a crypto period");

    // A paid payment is never asked about again, so it is never paid twice.
    assert_eq!(
        reconcile(&pool, &crypto, Utc::now()).await.expect("a pass"),
        0
    );
    assert_eq!(period_end(&app, &cookie).await, Some(end));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_payment_that_just_changed_is_left_to_its_events(pool: PgPool) {
    let crypto = CryptoPayments::fake(SECRET);
    let app = app_with(&pool, Some(crypto.clone()));
    let (_, cookie) = register(&app, "ada@example.com").await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    let invoice = sqlx::query_scalar!(
        r#"SELECT provider_invoice_id AS "invoice!" FROM crypto_payments WHERE unid = $1"#,
        order,
    )
    .fetch_one(&pool)
    .await
    .expect("the payment has an invoice");

    crypto.set_fake_status(&invoice, PaymentStatus::Paid);
    assert_eq!(
        reconcile(&pool, &crypto, Utc::now()).await.expect("a pass"),
        0
    );
    assert_eq!(period_end(&app, &cookie).await, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_payment_landing_after_expiry_still_counts(pool: PgPool) {
    let crypto = CryptoPayments::fake(SECRET);
    let app = app_with(&pool, Some(crypto.clone()));
    let (unid, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, unid).await;
    let order = checkout(&app, &cookie, "business", "year").await;
    event(&app, &pool, order, "expired", SECRET).await;
    let invoice = quiet(&pool, order).await;

    crypto.set_fake_status(&invoice, PaymentStatus::Paid);
    assert_eq!(
        reconcile(&pool, &crypto, Utc::now()).await.expect("a pass"),
        1
    );
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "business"
    );
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

#[sqlx::test(migrations = "../migrations")]
async fn the_admin_sees_a_plan_paid_in_crypto_and_until_when(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let order = checkout(&app, &cookie, "business", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;

    let (status, rows) = send(
        &app,
        Method::GET,
        "/api/admin/billing",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let row = rows
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["email"] == "ada@example.com"))
        .expect("the host is listed");
    assert_eq!(row["plan"], "business");
    let until: DateTime<Utc> =
        serde_json::from_value(row["crypto_until"].clone()).expect("a crypto end");
    assert_eq!(period_end(&app, &cookie).await, Some(until));
    let other = rows
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["email"] == "admin@example.com"))
        .expect("the admin is listed");
    assert_eq!(other["crypto_until"], Value::Null);
}

#[sqlx::test(migrations = "../migrations")]
async fn the_pricing_page_learns_whether_crypto_is_on_without_a_session(pool: PgPool) {
    let (status, options) = send(
        &app(&pool),
        Method::GET,
        "/api/billing/crypto/options",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(options, json!({ "available": true, "coins": "test coins" }));

    let (_, options) = send(
        &app_with(&pool, None),
        Method::GET,
        "/api/billing/crypto/options",
        None,
        None,
    )
    .await;
    assert_eq!(options, json!({ "available": false, "coins": null }));
}

struct PaymentShape {
    kind: String,
    amount_cents: i32,
    billing_interval: String,
}

async fn shape_of(pool: &PgPool, order: Uuid) -> PaymentShape {
    sqlx::query_as!(
        PaymentShape,
        "SELECT kind, amount_cents, billing_interval FROM crypto_payments WHERE unid = $1",
        order,
    )
    .fetch_one(pool)
    .await
    .expect("the payment exists")
}

#[sqlx::test(migrations = "../migrations")]
async fn a_running_pro_period_moves_to_business_for_the_difference(pool: PgPool) {
    let app = app(&pool);
    let (unid, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, unid).await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    let end = period_end(&app, &cookie).await.expect("a crypto period");

    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["upgrade"]["plan"], "business");
    let quoted = crypto["upgrade"]["amount_cents"].as_i64().expect("a price");
    // $6 a month more, for about a month left.
    assert!((590..=625).contains(&quoted), "{quoted}");

    // The interval asked for does not matter: an upgrade keeps the running one.
    let upgrade = checkout(&app, &cookie, "business", "year").await;
    let shape = shape_of(&pool, upgrade).await;
    assert_eq!(shape.kind, "upgrade");
    assert_eq!(shape.billing_interval, "month");
    assert!((i64::from(shape.amount_cents) - quoted).abs() <= 1);
    event(&app, &pool, upgrade, "paid", SECRET).await;

    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "business"
    );
    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["active"]["plan"], "business");
    assert_eq!(crypto["upgrade"], Value::Null);
    assert_eq!(period_end(&app, &cookie).await, Some(end));

    // Lower is refused; renewing renews Business after the same end.
    let pick = |plan: &str| json!({ "plan": plan, "interval": "month" });
    let (status, _) = send(
        &app,
        Method::POST,
        CHECKOUT,
        Some(&cookie),
        Some(pick("pro")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let renewal = checkout(&app, &cookie, "business", "month").await;
    assert_eq!(shape_of(&pool, renewal).await.amount_cents, 1600);
    event(&app, &pool, renewal, "paid", SECRET).await;
    let renewed = period_end(&app, &cookie).await.expect("a crypto period");
    assert!(near(
        renewed,
        end.checked_add_months(Months::new(1)).expect("in range")
    ));
    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["active"]["plan"], "business");
}

/// The host's payment paid last and not refunded yet, else the one refunded last.
async fn last_paid(pool: &PgPool, user: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "SELECT unid FROM crypto_payments WHERE user_unid = $1 AND paid_at IS NOT NULL
         ORDER BY status = 'paid' DESC, paid_at DESC LIMIT 1",
        user,
    )
    .fetch_one(pool)
    .await
    .expect("a paid payment")
}

/// Refunds one payment, all of what is left or `amount` cents of it.
async fn refund_payment(
    app: &Router,
    cookie: &str,
    payment: Uuid,
    amount: Option<u32>,
) -> (StatusCode, Value) {
    let body = amount.map_or_else(|| json!({}), |cents| json!({ "amount_cents": cents }));
    send(
        app,
        Method::POST,
        &format!("/api/admin/billing/crypto-payments/{payment}/refund"),
        Some(cookie),
        Some(body),
    )
    .await
}

#[sqlx::test(migrations = "../migrations")]
async fn a_refund_takes_back_the_time_its_payment_bought(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let first = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, first, "paid", SECRET).await;
    let one_month = period_end(&app, &cookie).await.expect("a crypto period");
    let second = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, second, "paid", SECRET).await;

    let (status, refund) =
        refund_payment(&app, &admin_cookie, last_paid(&pool, host).await, None).await;
    assert_eq!(status, StatusCode::OK, "{refund}");
    assert_eq!(refund["amount_cents"], 1000);
    assert!(
        refund["claim_url"]
            .as_str()
            .is_some_and(|url| url.starts_with("https://crypto.test/refund/"))
    );
    assert_eq!(period_end(&app, &cookie).await, Some(one_month));
    let notices = waiting_notices(&pool, host).await;
    let templates: Vec<&str> = notices
        .iter()
        .map(|(template, _)| template.as_str())
        .collect();
    assert_eq!(
        templates,
        ["crypto_ended", "crypto_refund", "crypto_renewal"]
    );
    assert!(near(notices[0].1, one_month));
    assert!(near(notices[2].1, one_month - Duration::days(3)));

    // The first payment goes too: no period is left, nor its reminder.
    refund_payment(&app, &admin_cookie, last_paid(&pool, host).await, None).await;
    assert_eq!(period_end(&app, &cookie).await, None);
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "free"
    );
    let templates: Vec<String> = waiting_notices(&pool, host)
        .await
        .into_iter()
        .map(|(template, _)| template)
        .collect();
    assert_eq!(templates, ["crypto_refund", "crypto_refund"]);

    let (status, _) = refund_payment(&app, &admin_cookie, last_paid(&pool, host).await, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

/// A provider may send "settled" again, such as a redelivery from its
/// dashboard. A refunded payment stays refunded and buys no time back.
#[sqlx::test(migrations = "../migrations")]
async fn a_settled_event_after_a_full_refund_changes_nothing(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    refund_payment(&app, &admin_cookie, order, None).await;

    assert_eq!(
        event(&app, &pool, order, "paid", SECRET).await,
        StatusCode::OK
    );
    let status = sqlx::query_scalar!("SELECT status FROM crypto_payments WHERE unid = $1", order)
        .fetch_one(&pool)
        .await
        .expect("the payment is readable");
    assert_eq!(status, "refunded");
    assert_eq!(period_end(&app, &cookie).await, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn refunding_an_upgrade_goes_back_to_the_lower_plan_until_the_same_end(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    let end = period_end(&app, &cookie).await.expect("a crypto period");
    let upgrade = checkout(&app, &cookie, "business", "month").await;
    event(&app, &pool, upgrade, "paid", SECRET).await;

    let (status, _) = refund_payment(&app, &admin_cookie, last_paid(&pool, host).await, None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["active"]["plan"], "pro");
    assert_eq!(period_end(&app, &cookie).await, Some(end));
}

#[sqlx::test(migrations = "../migrations")]
async fn only_an_admin_refunds(pool: PgPool) {
    let app = app(&pool);
    let (host, cookie) = register(&app, "ada@example.com").await;
    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;

    let (status, _) = refund_payment(&app, &cookie, last_paid(&pool, host).await, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(period_end(&app, &cookie).await.is_some());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_full_refund_during_the_trial_brings_its_ending_notice_back(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    let trial_notice = waiting_notices(&pool, host).await;
    assert_eq!(trial_notice.len(), 1);
    assert_eq!(trial_notice[0].0, "trial_ending");

    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    let paid: Vec<String> = waiting_notices(&pool, host)
        .await
        .into_iter()
        .map(|(template, _)| template)
        .collect();
    assert_eq!(paid, ["crypto_ended", "crypto_renewal"]);

    refund_payment(&app, &admin_cookie, last_paid(&pool, host).await, None).await;
    let refunded = waiting_notices(&pool, host).await;
    let templates: Vec<&str> = refunded
        .iter()
        .map(|(template, _)| template.as_str())
        .collect();
    assert_eq!(templates, ["crypto_refund", "trial_ending"]);
    assert!(near(refunded[1].1, trial_notice[0].1));
}

async fn receipts(pool: &PgPool, user: Uuid) -> Vec<Value> {
    sqlx::query_scalar!(
        r#"SELECT e.payload AS "payload!" FROM email_queue e JOIN users u ON u.email = e.email
           WHERE u.unid = $1 AND e.template = 'crypto_receipt' ORDER BY e.created_at"#,
        user,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
}

#[sqlx::test(migrations = "../migrations")]
async fn a_credited_payment_sends_a_receipt_with_what_it_bought(pool: PgPool) {
    let app = app(&pool);
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let order = checkout(&app, &cookie, "business", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    event(&app, &pool, order, "paid", SECRET).await;

    let sent = receipts(&pool, host).await;
    assert_eq!(sent.len(), 1, "a repeated event sends no second receipt");
    let end = period_end(&app, &cookie).await.expect("a crypto period");
    assert_eq!(
        sent[0]["lines"],
        json!([
            format!("Business until {}", end.format("%-d %B %Y")),
            "Paid $16.00"
        ])
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_payment_during_the_trial_waits_for_its_end(pool: PgPool) {
    let app = app(&pool);
    let (host, cookie) = register(&app, "ada@example.com").await;
    let trial_end: DateTime<Utc> = serde_json::from_value(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["trial_ends_at"].clone(),
    )
    .expect("a trial end");

    let order = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    let end = period_end(&app, &cookie).await.expect("a crypto period");
    assert!(near(
        end,
        trial_end
            .checked_add_months(Months::new(1))
            .expect("in range")
    ));
    let _ = host;
}

#[sqlx::test(migrations = "../migrations")]
async fn a_higher_plan_than_the_trial_starts_at_once(pool: PgPool) {
    let app = app(&pool);
    let (_, cookie) = register(&app, "ada@example.com").await;

    let order = checkout(&app, &cookie, "business", "month").await;
    event(&app, &pool, order, "paid", SECRET).await;
    let end = period_end(&app, &cookie).await.expect("a crypto period");
    assert!(near(
        end,
        Utc::now()
            .checked_add_months(Months::new(1))
            .expect("in range")
    ));
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "business"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_short_payment_is_told_and_a_plain_expiry_is_not(pool: PgPool) {
    let app = app(&pool);
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;

    let expired = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, expired, "expired", SECRET).await;
    let templates = |notices: Vec<(String, DateTime<Utc>)>| -> Vec<String> {
        notices.into_iter().map(|(template, _)| template).collect()
    };
    // Only sign-up's trial notice waits.
    assert_eq!(
        templates(waiting_notices(&pool, host).await),
        ["trial_ending"]
    );

    let short = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, short, "underpaid", SECRET).await;
    event(&app, &pool, short, "underpaid", SECRET).await;
    assert_eq!(
        templates(waiting_notices(&pool, host).await),
        ["crypto_underpaid", "trial_ending"]
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn checkouts_are_limited_per_host(pool: PgPool) {
    let app = app(&pool);
    let (_, cookie) = register(&app, "ada@example.com").await;
    let pick = json!({ "plan": "pro", "interval": "month" });
    for _ in 0..10 {
        let (status, _) = send(
            &app,
            Method::POST,
            CHECKOUT,
            Some(&cookie),
            Some(pick.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, _) = send(&app, Method::POST, CHECKOUT, Some(&cookie), Some(pick)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrations = "../migrations")]
async fn renewing_before_the_end_keeps_the_price_last_paid(pool: PgPool) {
    let app = app(&pool);
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let first = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, first, "paid", SECRET).await;
    // As if the host had paid $8 before today's $10.
    sqlx::query_scalar!(
        "UPDATE crypto_payments SET amount_cents = 800 WHERE unid = $1 RETURNING 1",
        first,
    )
    .fetch_one(&pool)
    .await
    .expect("the payment exists");

    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(
        crypto["renewal"],
        json!([
            { "interval": "month", "amount_cents": 800 },
            { "interval": "year", "amount_cents": 9600 },
        ])
    );
    let renewal = checkout(&app, &cookie, "pro", "month").await;
    assert_eq!(shape_of(&pool, renewal).await.amount_cents, 800);

    // Once the period has ended, the host pays today's price.
    sqlx::query_scalar!(
        "UPDATE crypto_payments SET period_end = now() - interval '1 minute'
         WHERE user_unid = $1 AND status = 'paid' RETURNING 1",
        host,
    )
    .fetch_all(&pool)
    .await
    .expect("the payments exist");
    let (_, crypto) = send(&app, Method::GET, CRYPTO, Some(&cookie), None).await;
    assert_eq!(crypto["renewal"], json!([]));
    let after_lapse = checkout(&app, &cookie, "pro", "month").await;
    assert_eq!(shape_of(&pool, after_lapse).await.amount_cents, 1000);
}

/// Records the subjects the email worker sends.
#[derive(Default)]
struct Outbox {
    sent: std::sync::Mutex<Vec<String>>,
}

impl Mailer for Outbox {
    async fn send(&self, _recipient: &str, email: RenderedEmail) -> Result<(), String> {
        self.sent
            .lock()
            .map_err(|_| "outbox poisoned".to_owned())?
            .push(email.subject);
        Ok(())
    }
}

/// Makes the host's crypto reminder due now.
async fn reminder_due(pool: &PgPool, user: Uuid) {
    sqlx::query_scalar!(
        "UPDATE email_queue e SET send_at = now() - interval '1 minute' FROM users u
         WHERE u.unid = $1 AND e.email = u.email AND e.template = 'crypto_renewal' RETURNING 1",
        user,
    )
    .fetch_one(pool)
    .await
    .expect("a reminder is queued");
}

#[sqlx::test(migrations = "../migrations")]
async fn crypto_reminders_stay_quiet_once_a_card_keeps_the_plan(pool: PgPool) {
    let app = app(&pool);
    let (card, card_cookie) = register(&app, "card@example.com").await;
    let (crypto_only, crypto_cookie) = register(&app, "crypto@example.com").await;
    for (user, cookie) in [(card, &card_cookie), (crypto_only, &crypto_cookie)] {
        end_trial(&pool, user).await;
        let order = checkout(&app, cookie, "pro", "month").await;
        event(&app, &pool, order, "paid", SECRET).await;
        reminder_due(&pool, user).await;
    }
    sqlx::query_scalar!(
        "UPDATE subscriptions SET status = 'active', stripe_subscription_id = 'sub_1'
         WHERE user_unid = $1 RETURNING 1",
        card,
    )
    .fetch_one(&pool)
    .await
    .expect("the trial row exists");

    let outbox = Outbox::default();
    process_due(&pool, &outbox).await;
    let mut sent = outbox.sent.lock().expect("outbox is not poisoned").clone();
    sent.sort();
    // Both receipts, and the one reminder the card does not make untrue.
    assert_eq!(
        sent,
        [
            "Your crypto payment arrived",
            "Your crypto payment arrived",
            "Your crypto plan ends in 3 days",
        ]
    );
    let card_notices: Vec<String> = waiting_notices(&pool, card)
        .await
        .into_iter()
        .map(|(template, _)| template)
        .collect();
    assert_eq!(
        card_notices,
        ["crypto_ended"],
        "the dropped reminder is gone"
    );
}

fn payments_path(user: Uuid) -> String {
    format!("/api/admin/billing/{user}/crypto-payments")
}

#[sqlx::test(migrations = "../migrations")]
async fn an_admin_lists_a_hosts_payments_and_refunds_part_then_the_rest(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let first = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, first, "paid", SECRET).await;
    let one_month = period_end(&app, &cookie).await.expect("a crypto period");
    let second = checkout(&app, &cookie, "pro", "month").await;
    event(&app, &pool, second, "paid", SECRET).await;
    let two_months = period_end(&app, &cookie).await.expect("a crypto period");
    // An open invoice is listed too, so the admin sees it is waiting.
    let open = checkout(&app, &cookie, "pro", "month").await;

    let (status, _) = send(&app, Method::GET, &payments_path(host), Some(&cookie), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, listed) = send(
        &app,
        Method::GET,
        &payments_path(host),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let unids: Vec<&str> = listed
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|payment| payment["unid"].as_str())
        .collect();
    assert_eq!(
        unids,
        [open.to_string(), second.to_string(), first.to_string()]
    );
    assert_eq!(listed[0]["status"], "pending");

    // The older payment, in part: money back, time kept.
    let (status, made) = refund_payment(&app, &admin_cookie, first, Some(300)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!(made["amount_cents"], 300);
    assert!(
        made["claim_url"]
            .as_str()
            .is_some_and(|url| url.ends_with("/300"))
    );
    assert_eq!(period_end(&app, &cookie).await, Some(two_months));
    let told = sqlx::query_scalar!(
        r#"SELECT e.payload AS "payload!" FROM email_queue e
           WHERE e.email = 'ada@example.com' AND e.template = 'crypto_refund'"#,
    )
    .fetch_one(&pool)
    .await
    .expect("the refund email is queued");
    assert_eq!(
        told["lines"],
        json!([
            "We have refunded $3.00 of your $10.00 crypto payment.",
            "Your plan keeps the time it paid for."
        ])
    );

    let (status, _) = refund_payment(&app, &admin_cookie, first, Some(701)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = refund_payment(&app, &admin_cookie, first, Some(0)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // The rest of it: now the month it bought comes off.
    let (status, made) = refund_payment(&app, &admin_cookie, first, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(made["amount_cents"], 700);
    assert_eq!(period_end(&app, &cookie).await, Some(one_month));
    let (_, listed) = send(
        &app,
        Method::GET,
        &payments_path(host),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(listed[2]["status"], "refunded");
    assert_eq!(listed[2]["refunded_cents"], 1000);
    assert_eq!(listed[1]["status"], "paid");

    let (status, _) = refund_payment(&app, &admin_cookie, first, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = refund_payment(&app, &admin_cookie, Uuid::new_v4(), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../migrations")]
async fn admins_hear_about_a_short_payment(pool: PgPool) {
    let app = app(&pool);
    let (admin, _) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let short = checkout(&app, &cookie, "business", "year").await;
    event(&app, &pool, short, "underpaid", SECRET).await;

    let alerts = sqlx::query_scalar!(
        r#"SELECT e.payload AS "payload!" FROM email_queue e
           WHERE e.email = 'admin@example.com' AND e.template = 'admin_crypto_underpaid'"#,
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(alerts.len(), 1);
    assert_eq!(
        alerts[0]["lines"],
        json!([
            "Ada Lovelace (ada@example.com)",
            "Business for 1 year, invoiced $156.00"
        ])
    );
    assert!(
        alerts[0]["billing_url"]
            .as_str()
            .is_some_and(|url| url.ends_with("/admin/plans"))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_short_payment_refunds_what_arrived_once(pool: PgPool) {
    let crypto = CryptoPayments::fake(SECRET);
    let app = app_with(&pool, Some(crypto.clone()));
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let short = checkout(&app, &cookie, "pro", "month").await;
    let invoice = quiet(&pool, short).await;
    crypto.set_fake_arrived(&invoice, 500);

    // Open, nothing to refund yet.
    let (status, _) = refund_payment(&app, &admin_cookie, short, None).await;
    assert_eq!(status, StatusCode::CONFLICT);

    event(&app, &pool, short, "underpaid", SECRET).await;
    let (status, _) = refund_payment(&app, &admin_cookie, short, Some(100)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "no amount: what arrived");
    let (status, made) = refund_payment(&app, &admin_cookie, short, None).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!(made["amount_cents"], 500);
    assert!(
        made["claim_url"]
            .as_str()
            .is_some_and(|url| url.ends_with("/arrived"))
    );
    assert_eq!(period_end(&app, &cookie).await, None);
    assert_eq!(
        send(&app, Method::GET, ME, Some(&cookie), None).await.1["plan"],
        "free"
    );

    let (status, _) = refund_payment(&app, &admin_cookie, short, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "once only");
    let (_, listed) = send(
        &app,
        Method::GET,
        &payments_path(host),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(listed[0]["status"], "underpaid");
    assert_eq!(listed[0]["refunded_cents"], 500);
}

async fn queued(pool: &PgPool, email: &str, template: &str) -> Vec<Value> {
    sqlx::query_scalar!(
        r#"SELECT e.payload AS "payload!" FROM email_queue e
           WHERE e.email = $1 AND e.template = $2 ORDER BY e.created_at"#,
        email,
        template,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
}

#[sqlx::test(migrations = "../migrations")]
async fn a_payment_keeps_its_coin_and_admins_hear_when_more_arrived(pool: PgPool) {
    let crypto = CryptoPayments::fake(SECRET);
    let app = app_with(&pool, Some(crypto.clone()));
    let (admin, admin_cookie) = register(&app, "admin@example.com").await;
    grant_admin(&pool, admin).await;
    let (host, cookie) = register(&app, "ada@example.com").await;
    end_trial(&pool, host).await;
    let over = checkout(&app, &cookie, "pro", "month").await;
    let invoice = quiet(&pool, over).await;
    crypto.set_fake_coin(&invoice, "LTC");
    crypto.set_fake_arrived(&invoice, 1100);
    event(&app, &pool, over, "paid", SECRET).await;

    assert_eq!(
        receipts(&pool, host).await[0]["lines"][1],
        "Paid $10.00 in LTC"
    );
    let (_, listed) = send(
        &app,
        Method::GET,
        &payments_path(host),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(listed[0]["coin"], "LTC");
    let alerts = queued(&pool, "admin@example.com", "admin_crypto_overpaid").await;
    assert_eq!(alerts.len(), 1);
    assert_eq!(
        alerts[0]["lines"],
        json!([
            "Ada Lovelace (ada@example.com)",
            "Pro for 1 month, invoiced $10.00, arrived $11.00"
        ])
    );

    // A rate that moved a little while paying is nobody's business.
    let close = checkout(&app, &cookie, "pro", "month").await;
    crypto.set_fake_arrived(&quiet(&pool, close).await, 1005);
    event(&app, &pool, close, "paid", SECRET).await;
    assert_eq!(
        queued(&pool, "admin@example.com", "admin_crypto_overpaid")
            .await
            .len(),
        1
    );
    assert_eq!(receipts(&pool, host).await[1]["lines"][1], "Paid $10.00");

    // The refund says which coin it is claimed in.
    refund_payment(&app, &admin_cookie, over, Some(100)).await;
    let refunds = queued(&pool, "ada@example.com", "crypto_refund").await;
    assert_eq!(
        refunds[0]["lines"][2],
        "You claim it in LTC, to a wallet address of yours."
    );
}
