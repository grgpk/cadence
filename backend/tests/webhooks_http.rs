//! Webhooks over HTTP: who may add one, which URLs are refused, what a
//! booking queues, and how the worker signs, delivers and retries.

// Clippy's test escapes cover `#[test]` functions only; the helpers here are test code too.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Mutex;

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
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
        webhooks::worker::{Sender, process_due},
    },
    signature,
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const WEBHOOKS: &str = "/api/host/webhooks";
const RECEIVER: &str = "https://hooks.example.com/cadence";

struct Host {
    unid: Uuid,
    cookie: String,
}

fn app(pool: &PgPool) -> Router {
    dotenvy::dotenv().ok();
    build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(InMemoryCalendar::new()),
        payments: None,
        calendly: Calendly::fake(CalendlyAccount::default()),
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

async fn register(app: &Router, email: &str) -> Host {
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
    let token = make_token(unid, 0).expect("JWT_SECRET is set for tests");
    Host {
        unid,
        cookie: format!("session={token}"),
    }
}

async fn give_business(pool: &PgPool, host: &Host) {
    sqlx::query_scalar!(
        "UPDATE subscriptions SET plan = 'business', status = 'comped' WHERE user_unid = $1 RETURNING 1",
        host.unid,
    )
    .fetch_one(pool)
    .await
    .expect("the host has a plan row");
}

/// A Business host with an hour-long call at 09:00 every day; their link.
async fn business_host(pool: &PgPool, app: &Router, email: &str) -> (Host, String) {
    let host = register(app, email).await;
    give_business(pool, &host).await;
    let (_, settings) = send(
        app,
        Method::GET,
        "/api/host/settings",
        Some(&host.cookie),
        None,
    )
    .await;
    for weekday in 0..7 {
        let hours = json!({
            "weekday": weekday, "start_time": "09:00:00", "end_time": "10:00:00", "slot_minutes": 60,
        });
        let (status, _) = send(
            app,
            Method::POST,
            "/api/availability",
            Some(&host.cookie),
            Some(hours),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let slug = settings["public_slug"].as_str().expect("a link").to_owned();
    (host, slug)
}

async fn add_endpoint(app: &Router, host: &Host, url: &str) -> (StatusCode, Value) {
    send(
        app,
        Method::POST,
        WEBHOOKS,
        Some(&host.cookie),
        Some(json!({ "url": url })),
    )
    .await
}

async fn book_first_slot(app: &Router, slug: &str) {
    let (_, slots) = send(
        app,
        Method::GET,
        &format!("/api/hosts/{slug}/types/call/slots"),
        None,
        None,
    )
    .await;
    let start = slots[0]["start"].clone();
    let booking = json!({
        "lead_unid": null, "slot_start": start,
        "timezone": "Europe/Paris", "invitee_name": "Ada Lovelace", "invitee_email": "ada@example.com",
    });
    let path = format!("/api/hosts/{slug}/types/call/bookings");
    let (status, _) = send(app, Method::POST, &path, None, Some(booking)).await;
    assert_eq!(status, StatusCode::CREATED);
}

/// One delivery as the dashboard lists it.
async fn deliveries(app: &Router, host: &Host, endpoint: &str) -> Vec<Value> {
    let path = format!("{WEBHOOKS}/{endpoint}/deliveries");
    let (status, list) = send(app, Method::GET, &path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    list.as_array().expect("a list").clone()
}

/// A receiver that answers `status` and keeps what it was sent.
struct Recorder {
    status: u16,
    received: Mutex<Vec<(String, String, String, String)>>,
}

impl Recorder {
    const fn answering(status: u16) -> Self {
        Self {
            status,
            received: Mutex::new(Vec::new()),
        }
    }

    fn received(&self) -> Vec<(String, String, String, String)> {
        self.received.lock().expect("not poisoned").clone()
    }
}

impl Sender for Recorder {
    async fn post(
        &self,
        url: &str,
        event: &str,
        body: String,
        signature: String,
    ) -> Result<u16, String> {
        self.received.lock().expect("not poisoned").push((
            url.to_owned(),
            event.to_owned(),
            body,
            signature,
        ));
        Ok(self.status)
    }
}

/// Makes every waiting delivery due now, as if its retry time had come.
async fn make_due(pool: &PgPool) {
    sqlx::query_scalar!(
        "UPDATE webhook_deliveries SET next_attempt_at = now() WHERE delivered_at IS NULL RETURNING 1"
    )
    .fetch_all(pool)
    .await
    .expect("deliveries are writable");
}

#[sqlx::test(migrations = "../migrations")]
async fn webhooks_need_business_and_a_public_https_url(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "nino@example.com").await;
    assert_eq!(
        add_endpoint(&app, &host, RECEIVER).await.0,
        StatusCode::FORBIDDEN,
        "Pro trial"
    );

    give_business(&pool, &host).await;
    for bad in [
        "http://hooks.example.com/cadence",
        "https://localhost/hook",
        "https://10.0.0.5/hook",
        "https://169.254.169.254/latest/meta-data",
    ] {
        assert_eq!(
            add_endpoint(&app, &host, bad).await.0,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }
    let (status, endpoint) = add_endpoint(&app, &host, RECEIVER).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(endpoint["url"], RECEIVER);
    assert!(
        endpoint["secret"]
            .as_str()
            .is_some_and(|secret| secret.starts_with("whsec_"))
    );
    for _ in 1..5 {
        assert_eq!(
            add_endpoint(&app, &host, RECEIVER).await.0,
            StatusCode::CREATED
        );
    }
    assert_eq!(
        add_endpoint(&app, &host, RECEIVER).await.0,
        StatusCode::CONFLICT,
        "five at most"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_booking_is_delivered_signed_with_the_endpoints_secret(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = business_host(&pool, &app, "nino@example.com").await;
    let (_, endpoint) = add_endpoint(&app, &host, RECEIVER).await;
    let endpoint_unid = endpoint["unid"].as_str().expect("an id").to_owned();
    let secret = endpoint["secret"].as_str().expect("a secret").to_owned();

    book_first_slot(&app, &slug).await;
    let waiting = deliveries(&app, &host, &endpoint_unid).await;
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0]["event"], "booking.created");
    assert_eq!(waiting[0]["delivered_at"], Value::Null);

    let receiver = Recorder::answering(204);
    process_due(&pool, &receiver).await;
    let received = receiver.received();
    assert_eq!(received.len(), 1);
    let (url, event, body, signature_header) = &received[0];
    assert_eq!(url, RECEIVER);
    assert_eq!(event, "booking.created");
    assert_eq!(
        signature::verify(&secret, body.as_bytes(), signature_header, Utc::now()),
        Ok(()),
        "the receiver can check it came from Cadence"
    );
    let body: Value = serde_json::from_str(body).expect("JSON");
    assert_eq!(body["id"], waiting[0]["unid"]);
    assert_eq!(body["event"], "booking.created");
    assert_eq!(body["data"]["booking"]["invitee_email"], "ada@example.com");
    assert_eq!(body["data"]["booking"]["minutes"], 60);
    assert_eq!(body["data"]["event_type"]["slug"], "call");
    assert_eq!(body["data"]["host_page"], slug.as_str());

    let sent = deliveries(&app, &host, &endpoint_unid).await;
    assert!(sent[0]["delivered_at"].is_string());
    assert_eq!(sent[0]["last_status"], 204);
    process_due(&pool, &receiver).await;
    assert_eq!(
        receiver.received().len(),
        1,
        "a delivered event is not sent again"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_failing_receiver_is_retried_then_given_up_on(pool: PgPool) {
    let app = app(&pool);
    let (host, _) = business_host(&pool, &app, "nino@example.com").await;
    let (_, endpoint) = add_endpoint(&app, &host, RECEIVER).await;
    let endpoint_unid = endpoint["unid"].as_str().expect("an id").to_owned();
    let path = format!("{WEBHOOKS}/{endpoint_unid}/test");
    let (status, _) = send(&app, Method::POST, &path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let receiver = Recorder::answering(500);
    process_due(&pool, &receiver).await;
    let tried = &deliveries(&app, &host, &endpoint_unid).await[0];
    assert_eq!(tried["event"], "ping");
    assert_eq!(tried["attempts"], 1);
    assert_eq!(tried["last_status"], 500);
    assert_eq!(tried["failed_at"], Value::Null);
    let next: DateTime<Utc> =
        serde_json::from_value(tried["next_attempt_at"].clone()).expect("a retry time");
    assert!(next > Utc::now(), "the retry waits");
    process_due(&pool, &receiver).await;
    assert_eq!(receiver.received().len(), 1, "not before its time");

    for _ in 1..8 {
        make_due(&pool).await;
        process_due(&pool, &receiver).await;
    }
    let given_up = &deliveries(&app, &host, &endpoint_unid).await[0];
    assert_eq!(given_up["attempts"], 8);
    assert!(given_up["failed_at"].is_string());
    make_due(&pool).await;
    process_due(&pool, &receiver).await;
    assert_eq!(receiver.received().len(), 8, "eight tries, then no more");
}

#[sqlx::test(migrations = "../migrations")]
async fn pausing_an_endpoint_stops_its_retries(pool: PgPool) {
    let app = app(&pool);
    let (host, _) = business_host(&pool, &app, "nino@example.com").await;
    let (_, endpoint) = add_endpoint(&app, &host, RECEIVER).await;
    let endpoint_unid = endpoint["unid"].as_str().expect("an id").to_owned();
    let path = format!("{WEBHOOKS}/{endpoint_unid}");
    send(
        &app,
        Method::POST,
        &format!("{path}/test"),
        Some(&host.cookie),
        None,
    )
    .await;
    let receiver = Recorder::answering(500);
    process_due(&pool, &receiver).await;
    assert_eq!(receiver.received().len(), 1);

    let off = Some(json!({ "is_active": false }));
    let (status, _) = send(&app, Method::PUT, &path, Some(&host.cookie), off).await;
    assert_eq!(status, StatusCode::OK);
    make_due(&pool).await;
    process_due(&pool, &receiver).await;

    assert_eq!(
        receiver.received().len(),
        1,
        "no retry to a paused endpoint"
    );
    let given_up = &deliveries(&app, &host, &endpoint_unid).await[0];
    assert!(given_up["failed_at"].is_string());
    assert_eq!(given_up["last_error"], "the endpoint is paused");
}

#[sqlx::test(migrations = "../migrations")]
async fn an_endpoint_switched_off_hears_nothing_and_stays_its_hosts(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = business_host(&pool, &app, "nino@example.com").await;
    let other = register(&app, "grace@example.com").await;
    let (_, endpoint) = add_endpoint(&app, &host, RECEIVER).await;
    let endpoint_unid = endpoint["unid"].as_str().expect("an id").to_owned();
    let path = format!("{WEBHOOKS}/{endpoint_unid}");

    // Another host can neither change, test nor read it.
    let off = Some(json!({ "is_active": false }));
    assert_eq!(
        send(&app, Method::PUT, &path, Some(&other.cookie), off.clone())
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &app,
            Method::GET,
            &format!("{path}/deliveries"),
            Some(&other.cookie),
            None
        )
        .await
        .1,
        json!([])
    );
    assert_eq!(
        send(&app, Method::DELETE, &path, Some(&other.cookie), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (_, listed) = send(&app, Method::GET, WEBHOOKS, Some(&other.cookie), None).await;
    assert_eq!(listed, json!([]));

    let (status, saved) = send(&app, Method::PUT, &path, Some(&host.cookie), off).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["is_active"], false);
    book_first_slot(&app, &slug).await;
    assert!(deliveries(&app, &host, &endpoint_unid).await.is_empty());

    assert_eq!(
        send(&app, Method::DELETE, &path, Some(&host.cookie), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    let (_, listed) = send(&app, Method::GET, WEBHOOKS, Some(&host.cookie), None).await;
    assert_eq!(listed, json!([]));
}
