//! Per-host booking links over HTTP: settings, isolation, question switches,
//! and slots that come from the host's own hours.

// Clippy's test escapes cover `#[test]` functions only; the helpers here are test code too.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use cadence_backend::domain::calendly::{
    client::{Calendly, FAKE_TOKEN},
    models::{ApiEventType, ApiInterval, ApiRule, ApiSchedule, ApiUser, CalendlyAccount},
};
use cadence_backend::{
    app_state::AppState,
    build_app,
    domain::{
        auth::service::make_token,
        billing::plans::Reminder,
        bookings,
        emails::{
            self,
            service::{CancelledBy, RenderedEmail},
            worker::{self, Mailer},
        },
        google_calendar::{
            calendar::{HostCalendar, InMemoryCalendar},
            db::GoogleCalendarDb,
        },
        host_settings, leads,
    },
};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// The link `cadence_host` gets: its name as a slug.
const HOST_SLUG: &str = "cadence-host";
const QUESTION_FLAGS: [&str; 5] = [
    "ask_currently_working_on",
    "ask_how_heard_about_us",
    "ask_what_stopping_you",
    "ask_investment_comfort",
    "ask_urgency_level",
];

struct Host {
    unid: Uuid,
    cookie: String,
}

fn app(pool: &PgPool) -> Router {
    app_with_calendar(pool).0
}

/// The app with a calendar the test can connect hosts to, fill and read back.
fn app_with_calendar(pool: &PgPool) -> (Router, InMemoryCalendar) {
    // JWT_SECRET lives in the workspace .env, next to DATABASE_URL.
    dotenvy::dotenv().ok();
    let calendar = InMemoryCalendar::new();
    let app = build_app(AppState {
        pool: pool.clone(),
        calendar: HostCalendar::InMemory(calendar.clone()),
        payments: None,
        calendly: Calendly::fake(calendly_account()),
    });
    (app, calendar)
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

async fn get(app: &Router, path: &str, cookie: Option<&str>) -> (StatusCode, Value) {
    send(app, Method::GET, path, cookie, None).await
}

async fn register(app: &Router, full_name: &str, email: &str) -> Host {
    let (status, user) = send(
        app,
        Method::POST,
        "/api/auth/register",
        None,
        Some(json!({ "email": email, "password": "password123", "full_name": full_name })),
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

/// A host whose page is at `HOST_SLUG`.
async fn cadence_host(app: &Router) -> Host {
    let host = register(app, "Cadence Host", "host@example.com").await;
    assert_eq!(slug(&settings(app, &host).await), HOST_SLUG);
    host
}

/// The host's settings, created on first read.
async fn settings(app: &Router, host: &Host) -> Value {
    let (status, settings) = get(app, "/api/host/settings", Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    settings
}

async fn save_settings(app: &Router, host: &Host, settings: &Value) -> (StatusCode, Value) {
    send(
        app,
        Method::PUT,
        "/api/host/settings",
        Some(&host.cookie),
        Some(settings.clone()),
    )
    .await
}

async fn save_conferencing(
    app: &Router,
    host: &Host,
    conferencing_app: &str,
    meeting_link: Option<&str>,
) -> (StatusCode, Value) {
    send(
        app,
        Method::PUT,
        "/api/host/conferencing",
        Some(&host.cookie),
        Some(json!({ "app": conferencing_app, "meeting_link": meeting_link })),
    )
    .await
}

/// The host's first kind of call, as they edit it: at `/book/<slug>/call`.
async fn first_type(app: &Router, host: &Host) -> Value {
    let (status, types) = get(app, "/api/host/event-types", Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    types[0].clone()
}

async fn save_type(app: &Router, host: &Host, kind: &Value) -> (StatusCode, Value) {
    let unid = kind["unid"].as_str().expect("the type has an id");
    send(
        app,
        Method::PUT,
        &format!("/api/host/event-types/{unid}"),
        Some(&host.cookie),
        Some(kind.clone()),
    )
    .await
}

fn slug(settings: &Value) -> &str {
    settings["public_slug"]
        .as_str()
        .expect("settings have a slug")
}

/// A host in Tbilisi (UTC+4, no clock changes) who takes one hour-long call at 09:00 every day.
async fn host_with_hours(app: &Router, email: &str) -> (Host, String) {
    let host = register(app, "Nino Tbilisi", email).await;
    let mut settings = settings(app, &host).await;
    settings["timezone"] = json!("Asia/Tbilisi");
    let (status, settings) = save_settings(app, &host, &settings).await;
    assert_eq!(status, StatusCode::OK);
    for weekday in 0..7 {
        let (status, _) = send(
            app,
            Method::POST,
            "/api/availability",
            Some(&host.cookie),
            Some(json!({
                "weekday": weekday,
                "start_time": "09:00:00",
                "end_time": "10:00:00",
                "slot_minutes": 60,
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let slug = slug(&settings).to_owned();
    (host, slug)
}

async fn slots(app: &Router, slug: &str) -> Vec<DateTime<Utc>> {
    let (status, slots) = get(app, &format!("/api/hosts/{slug}/types/call/slots"), None).await;
    assert_eq!(status, StatusCode::OK);
    slots
        .as_array()
        .expect("slots are a list")
        .iter()
        .map(|slot| {
            assert_eq!(slot["minutes"], 60);
            serde_json::from_value(slot["start"].clone()).expect("slot has a start")
        })
        .collect()
}

fn booking(start: DateTime<Utc>, lead_unid: Option<Uuid>) -> Value {
    json!({
        "lead_unid": lead_unid,
        "calling_visit_unid": null,
        "slot_start": start,
        "timezone": "Europe/Paris",
        "invitee_name": "Ada Lovelace",
        "invitee_email": "ada@example.com",
    })
}

/// Books `start` as Ada and returns the call's id and its manage link token.
async fn book(app: &Router, slug: &str, start: DateTime<Utc>) -> (Uuid, String) {
    let (status, booked) = send(
        app,
        Method::POST,
        &format!("/api/hosts/{slug}/types/call/bookings"),
        None,
        Some(booking(start, None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    (
        serde_json::from_value(booked["unid"].clone()).expect("booking has an id"),
        booked["manage_token"]
            .as_str()
            .expect("booking has a manage token")
            .to_owned(),
    )
}

/// The emails a booking still has waiting, as "template -> recipient".
async fn waiting_emails(pool: &PgPool, booking_unid: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT template || ' -> ' || email AS "line!" FROM email_queue
           WHERE booking_unid = $1 AND sent_at IS NULL ORDER BY template"#,
        booking_unid,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
}

#[sqlx::test(migrations = "../migrations")]
async fn public_page_shows_no_ids_and_asks_nothing_by_default(pool: PgPool) {
    let app = app(&pool);
    cadence_host(&app).await;

    let (status, page) = get(&app, &format!("/api/hosts/{HOST_SLUG}/types/call"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.get("host_unid").is_none());
    assert!(page.get("email").is_none());
    assert_eq!(page["host_name"], "Cadence Host");
    for flag in QUESTION_FLAGS {
        assert_eq!(page[flag], false, "{flag}");
    }

    let (status, _) = get(&app, "/api/hosts/CADENCE-HOST/types/call", None).await;
    assert_eq!(status, StatusCode::OK, "links are case-insensitive");
    let (status, _) = get(&app, "/api/hosts/nobody-here/types/call", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_switched_off_page_is_readable_and_a_banned_host_is_not(pool: PgPool) {
    let app = app(&pool);
    cadence_host(&app).await;
    let page_path = format!("/api/hosts/{HOST_SLUG}/types/call");
    let (_, page) = get(&app, &page_path, None).await;
    assert_eq!(page["accepting_bookings"], true);

    sqlx::query_scalar!(
        "UPDATE widget_settings SET is_active = false WHERE public_slug = $1 RETURNING 1",
        HOST_SLUG,
    )
    .fetch_one(&pool)
    .await
    .expect("the host exists");
    let (status, page) = get(&app, &page_path, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["accepting_bookings"], false);
    assert_eq!(page["host_name"], "Cadence Host");
    let (status, _) = get(&app, &format!("{page_path}/slots"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    sqlx::query_scalar!(
        "UPDATE users SET status = 'Banned' FROM widget_settings w
         WHERE w.host_unid = users.unid AND w.public_slug = $1 RETURNING 1",
        HOST_SLUG,
    )
    .fetch_one(&pool)
    .await
    .expect("the host exists");
    let (status, _) = get(&app, &page_path, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../migrations")]
async fn every_host_gets_their_own_link(pool: PgPool) {
    let app = app(&pool);
    let (status, _) = get(&app, "/api/host/settings", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let first = register(&app, "Ada Lovelace", "ada@example.com").await;
    let second = register(&app, "Ada Lovelace", "ada.two@example.com").await;
    let first_settings = settings(&app, &first).await;
    let second_settings = settings(&app, &second).await;

    assert_eq!(slug(&first_settings), "ada-lovelace");
    assert_ne!(slug(&first_settings), slug(&second_settings));
    assert!(slug(&second_settings).starts_with("ada-lovelace-"));
    assert_eq!(first_settings["timezone"], "UTC");
    // Reading again does not mint a new link.
    assert_eq!(settings(&app, &first).await, first_settings);

    let (status, page) = get(&app, "/api/hosts/ada-lovelace/types/call", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["host_name"], "Ada Lovelace");
    assert_eq!(page["calendar_connected"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn settings_are_validated_and_stay_with_their_host(pool: PgPool) {
    let app = app(&pool);
    let ada = register(&app, "Ada Lovelace", "ada@example.com").await;
    let grace = register(&app, "Grace Hopper", "grace@example.com").await;
    let ada_settings = settings(&app, &ada).await;
    let grace_settings = settings(&app, &grace).await;

    for (field, bad) in [
        ("public_slug", json!("Ada Lovelace!")),
        ("public_slug", json!("ab")),
        ("timezone", json!("Mars/Olympus")),
    ] {
        let mut changed = ada_settings.clone();
        changed[field] = bad.clone();
        let (status, _) = save_settings(&app, &ada, &changed).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field} = {bad}");
    }

    let mut taken = ada_settings.clone();
    taken["public_slug"] = grace_settings["public_slug"].clone();
    let (status, _) = save_settings(&app, &ada, &taken).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let mut changed = ada_settings.clone();
    changed["public_slug"] = json!("ada");
    let (status, saved) = save_settings(&app, &ada, &changed).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved, changed);
    let mut kind = first_type(&app, &ada).await;
    kind["title"] = json!("Talk to Ada");
    kind["ask_urgency_level"] = json!(true);
    let (status, _) = save_type(&app, &ada, &kind).await;
    assert_eq!(status, StatusCode::OK);

    let (_, page) = get(&app, "/api/hosts/ada/types/call", None).await;
    assert_eq!(page["title"], "Talk to Ada");
    assert_eq!(page["ask_urgency_level"], true);
    let (status, _) = get(&app, "/api/hosts/ada-lovelace/types/call", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the old link is gone");
    assert_eq!(settings(&app, &grace).await, grace_settings);
}

#[sqlx::test(migrations = "../migrations")]
async fn answers_are_kept_only_for_questions_the_host_asks(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "Ada Lovelace", "ada@example.com").await;
    settings(&app, &host).await;
    let mut kind = first_type(&app, &host).await;
    let lead_unid = Uuid::new_v4();
    let path = format!("/api/hosts/ada-lovelace/types/call/leads/{lead_unid}");
    let answers = json!({
        "first_name": "Linus",
        "email": "linus@example.com",
        "investment_comfort": "$500–2,000 (serious training)",
        "how_heard_about_us": "Google",
        "what_stopping_you": "Lack of time",
        "currently_working_on": "A kernel",
        "urgency_level": "7-8 (High priority)",
        "qualification_status": "Qualified",
    });

    let (status, lead) = send(&app, Method::POST, &path, None, Some(answers.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(lead["first_name"], "Linus");
    for field in [
        "investment_comfort",
        "how_heard_about_us",
        "what_stopping_you",
        "currently_working_on",
        "urgency_level",
        "qualification_status",
    ] {
        assert_eq!(lead[field], Value::Null, "{field}");
    }

    kind["ask_investment_comfort"] = json!(true);
    let (status, _) = save_type(&app, &host, &kind).await;
    assert_eq!(status, StatusCode::OK);
    let (_, lead) = send(&app, Method::POST, &path, None, Some(answers)).await;
    assert_eq!(lead["investment_comfort"], "$500–2,000 (serious training)");
    assert_eq!(lead["qualification_status"], "Qualified");
    assert_eq!(lead["how_heard_about_us"], Value::Null);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_lead_cannot_be_rewritten_through_another_hosts_link(pool: PgPool) {
    let app = app(&pool);
    let ada = register(&app, "Ada Lovelace", "ada@example.com").await;
    let grace = register(&app, "Grace Hopper", "grace@example.com").await;
    settings(&app, &ada).await;
    settings(&app, &grace).await;
    let lead_unid = Uuid::new_v4();

    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/ada-lovelace/types/call/leads/{lead_unid}"),
        None,
        Some(json!({ "first_name": "Linus", "email": "linus@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for suffix in ["", "/submit"] {
        let (status, _) = send(
            &app,
            Method::POST,
            &format!("/api/hosts/grace-hopper/types/call/leads/{lead_unid}{suffix}"),
            None,
            Some(json!({ "first_name": "Mallory" })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{suffix}");
    }

    let lead = leads::db::find(&pool, ada.unid, lead_unid)
        .await
        .expect("lead query runs")
        .expect("lead still belongs to its host");
    assert_eq!(lead.first_name.as_deref(), Some("Linus"));
    assert!(lead.form_submitted_at.is_none());
}

#[sqlx::test(migrations = "../migrations")]
async fn slots_follow_the_hosts_hours_and_timezone_without_google(pool: PgPool) {
    let app = app(&pool);
    let (_, slug) = host_with_hours(&app, "nino@example.com").await;

    let slots = slots(&app, &slug).await;
    // 09:00 in Tbilisi is 05:00 UTC, once a day, from 4 hours out to 7 days out.
    assert!((6..=7).contains(&slots.len()), "{} slots", slots.len());
    for start in &slots {
        assert_eq!((start.hour(), start.minute()), (5, 0), "{start}");
        assert!(*start > Utc::now());
    }

    // A host who has set no hours offers nothing, and that is not an error.
    let idle = register(&app, "Idle Host", "idle@example.com").await;
    let idle_settings = settings(&app, &idle).await;
    let (status, none) = get(
        &app,
        &format!("/api/hosts/{}/types/call/slots", self::slug(&idle_settings)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(none, json!([]));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_booked_time_is_gone_for_the_next_visitor(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let path = format!("/api/hosts/{slug}/types/call/bookings");
    let open = slots(&app, &slug).await;
    let start = open[0];

    let (status, _) = send(
        &app,
        Method::POST,
        &path,
        None,
        Some(booking(start + Duration::minutes(7), None)),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "only offered times can be booked"
    );

    let (status, _) = send(
        &app,
        Method::POST,
        &path,
        None,
        Some(booking(start, Some(Uuid::new_v4()))),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "the lead must be the host's own"
    );

    let (status, booked) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(booked["meeting_url"], Value::Null, "no Google, no own link");

    let (status, _) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(slots(&app, &slug).await, open[1..]);

    let (status, mine) = get(&app, "/api/bookings", Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mine.as_array().map(Vec::len), Some(1));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_hosts_own_meeting_link_is_validated_and_kept_private(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "Ada Lovelace", "ada@example.com").await;
    let saved = settings(&app, &host).await;
    assert_eq!(saved["meeting_link"], Value::Null);
    assert_eq!(saved["conferencing_app"], "google_meet");

    let too_long = format!("https://example.com/{}", "a".repeat(500));
    for bad in ["http://zoom.us/j/1", "not a url", too_long.as_str()] {
        let (status, _) = save_conferencing(&app, &host, "custom_link", Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
    // A custom link needs the link.
    let (status, _) = save_conferencing(&app, &host, "custom_link", Some("  ")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, with_link) = save_conferencing(
        &app,
        &host,
        "custom_link",
        Some("  https://zoom.us/j/123456789 "),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(with_link["meeting_link"], "https://zoom.us/j/123456789");
    assert_eq!(with_link["conferencing_app"], "custom_link");

    let (_, page) = get(&app, "/api/hosts/ada-lovelace/types/call", None).await;
    assert_eq!(page["has_meeting_link"], true);
    assert_eq!(page["conferencing_app"], "custom_link");
    assert!(page.get("meeting_link").is_none());
    assert!(!page.to_string().contains("zoom.us"));

    // The Settings form cannot overwrite the link it no longer shows.
    let mut stale = with_link.clone();
    stale["meeting_link"] = Value::Null;
    stale["conferencing_app"] = json!("google_meet");
    let (status, kept) = save_settings(&app, &host, &stale).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(kept["meeting_link"], "https://zoom.us/j/123456789");
    assert_eq!(kept["conferencing_app"], "custom_link");

    // Meet may be picked before a calendar is connected.
    let (status, cleared) = save_conferencing(&app, &host, "google_meet", Some("")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cleared["meeting_link"], Value::Null);
    let (_, page) = get(&app, "/api/hosts/ada-lovelace/types/call", None).await;
    assert_eq!(page["has_meeting_link"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_booking_without_google_uses_the_hosts_meeting_link(pool: PgPool) {
    const LINK: &str = "https://zoom.us/j/123456789";
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let (status, _) = save_conferencing(&app, &host, "google_meet", Some(LINK)).await;
    assert_eq!(status, StatusCode::OK);

    let start = slots(&app, &slug).await[0];
    let (status, booked) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/types/call/bookings"),
        None,
        Some(booking(start, None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(booked["meeting_url"], LINK);

    // The link reaches the inbox with the confirmation and with every reminder.
    let without_link = sqlx::query_scalar!(
        "SELECT count(*) FROM email_queue
         WHERE email = $1 AND payload->>'meeting_url' IS DISTINCT FROM $2",
        "ada@example.com",
        LINK,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(without_link, Some(0));
    let confirmations = sqlx::query_scalar!(
        "SELECT count(*) FROM email_queue
         WHERE email = $1 AND template = 'booking_confirmation' AND payload->>'meeting_url' = $2",
        "ada@example.com",
        LINK,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(confirmations, Some(1));
}

/// Which times these rules open is the slot policy's to test (`bookings/slots.rs`);
/// here only what a host can save and what a visitor can see of it.
#[sqlx::test(migrations = "../migrations")]
async fn booking_rules_are_checked_and_stay_off_the_public_page(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    assert_eq!(settings(&app, &host).await["is_active"], true);
    let mut rules = first_type(&app, &host).await;
    assert_eq!(rules["minimum_notice_hours"], 4);
    assert_eq!(rules["booking_window_days"], 7);
    assert_eq!(rules["buffer_minutes"], 0, "no gap until chosen");
    assert_eq!(
        rules["duration_minutes"],
        Value::Null,
        "the hours set the length"
    );

    for (field, bad) in [
        ("minimum_notice_hours", json!(-1)),
        ("minimum_notice_hours", json!(169)),
        ("booking_window_days", json!(0)),
        ("booking_window_days", json!(91)),
        ("buffer_minutes", json!(-1)),
        ("buffer_minutes", json!(121)),
        ("duration_minutes", json!(4)),
        ("duration_minutes", json!(241)),
        ("title", json!("  ")),
        ("slug", json!("A call")),
    ] {
        let mut changed = rules.clone();
        changed[field] = bad.clone();
        let (status, _) = save_type(&app, &host, &changed).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field} = {bad}");
    }

    rules["booking_window_days"] = json!(2);
    rules["minimum_notice_hours"] = json!(24);
    rules["buffer_minutes"] = json!(15);
    let (status, saved) = save_type(&app, &host, &rules).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved, rules);
    let (_, page) = get(&app, &format!("/api/hosts/{slug}/types/call"), None).await;
    assert_eq!(page["booking_window_days"], 2);
    assert!(page.get("minimum_notice_hours").is_none());
    assert!(page.get("buffer_minutes").is_none());
    // The saved rules reach the open times: one call a day, a day out at the earliest.
    for start in slots(&app, &slug).await {
        assert!(start > Utc::now() + Duration::hours(23), "{start}");
        assert!(start < Utc::now() + Duration::days(2), "{start}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_paused_page_can_be_read_but_not_booked(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let start = slots(&app, &slug).await[0];
    let mut paused = settings(&app, &host).await;
    paused["is_active"] = json!(false);
    let (status, saved) = save_settings(&app, &host, &paused).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["is_active"], false);

    let (status, page) = get(&app, &format!("/api/hosts/{slug}/types/call"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["accepting_bookings"], false);
    assert_eq!(page["host_name"], "Nino Tbilisi");
    let (status, _) = get(&app, &format!("/api/hosts/{slug}/types/call/slots"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let path = format!("/api/hosts/{slug}/types/call/bookings");
    let (status, _) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/types/call/leads/{}", Uuid::new_v4()),
        None,
        Some(json!({ "first_name": "Ada" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the form is closed too");

    // The host still sees their settings and can open the page again.
    paused["is_active"] = json!(true);
    let (status, _) = save_settings(&app, &host, &paused).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_host_reads_their_booking_with_the_invitees_answers(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let other = register(&app, "Grace Hopper", "grace@example.com").await;
    let mut asking = first_type(&app, &host).await;
    asking["ask_urgency_level"] = json!(true);
    let (status, _) = save_type(&app, &host, &asking).await;
    assert_eq!(status, StatusCode::OK);

    let lead_unid = Uuid::new_v4();
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/types/call/leads/{lead_unid}"),
        None,
        Some(json!({
            "first_name": "Ada",
            "phone": "555 0100",
            "country_code": "+995",
            "urgency_level": "7-8 (High priority)",
            "how_heard_about_us": "Google",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let open = slots(&app, &slug).await;
    let path = format!("/api/hosts/{slug}/types/call/bookings");
    let (status, with_lead) = send(
        &app,
        Method::POST,
        &path,
        None,
        Some(booking(open[0], Some(lead_unid))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, without_lead) = send(
        &app,
        Method::POST,
        &path,
        None,
        Some(booking(open[1], None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let detail_path = format!("/api/bookings/{}", with_lead["unid"].as_str().expect("id"));
    let (status, detail) = get(&app, &detail_path, Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    // The host reads the same call, without the invitee's manage link.
    let mut as_booked = with_lead.clone();
    assert!(
        as_booked
            .as_object_mut()
            .expect("booking is an object")
            .remove("manage_token")
            .is_some()
    );
    assert_eq!(detail["booking"], as_booked);
    assert_eq!(detail["lead"]["phone"], "555 0100");
    assert_eq!(detail["lead"]["country_code"], "+995");
    assert_eq!(detail["lead"]["urgency_level"], "7-8 (High priority)");
    assert_eq!(
        detail["lead"]["how_heard_about_us"],
        Value::Null,
        "not asked"
    );

    let (status, detail) = get(
        &app,
        &format!(
            "/api/bookings/{}",
            without_lead["unid"].as_str().expect("id")
        ),
        Some(&host.cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["lead"], Value::Null);

    let (status, _) = get(&app, &detail_path, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = get(&app, &detail_path, Some(&other.cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "another host's call");
    let unknown = format!("/api/bookings/{}", Uuid::new_v4());
    let (status, _) = get(&app, &unknown, Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_cancelled_call_frees_its_time_and_tells_the_invitee(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let other = register(&app, "Grace Hopper", "grace@example.com").await;
    let book_path = format!("/api/hosts/{slug}/types/call/bookings");
    let open = slots(&app, &slug).await;
    // Far enough out that every reminder is queued.
    let start = *open.last().expect("the host has open times");

    let (status, booked) = send(
        &app,
        Method::POST,
        &book_path,
        None,
        Some(booking(start, None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let unid: Uuid = serde_json::from_value(booked["unid"].clone()).expect("booking has an id");
    let cancel_path = format!("/api/bookings/{unid}/cancel");
    // The confirmation has gone out; the reminders are still waiting.
    sqlx::query_scalar!(
        "UPDATE email_queue SET sent_at = now()
         WHERE booking_unid = $1 AND template = 'booking_confirmation' RETURNING 1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("confirmation is queued");

    let (status, _) = send(&app, Method::POST, &cancel_path, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(&app, Method::POST, &cancel_path, Some(&other.cookie), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "another host's call");
    assert!(!slots(&app, &slug).await.contains(&start));

    let (status, cancelled) =
        send(&app, Method::POST, &cancel_path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["unid"], booked["unid"]);
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(slots(&app, &slug).await, open, "the time is open again");

    // The sent confirmation stays; the waiting reminders are gone.
    let waiting = sqlx::query_scalar!(
        "SELECT template FROM email_queue WHERE booking_unid = $1 AND sent_at IS NULL AND send_at <= now()",
        unid,
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(waiting, ["booking_cancelled"]);
    let all = sqlx::query_scalar!(
        "SELECT count(*) FROM email_queue WHERE booking_unid = $1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(all, Some(2));

    let (status, _) = send(&app, Method::POST, &cancel_path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "already cancelled");

    let (status, again) = send(
        &app,
        Method::POST,
        &book_path,
        None,
        Some(booking(start, None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "the freed time can be booked");
    let (status, _) = send(
        &app,
        Method::POST,
        &book_path,
        None,
        Some(booking(start, None)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "but only once");
    let (_, mine) = get(&app, "/api/bookings", Some(&host.cookie)).await;
    assert_eq!(mine.as_array().map(Vec::len), Some(2));

    // A call that has started can no longer be called off.
    let again_unid: Uuid =
        serde_json::from_value(again["unid"].clone()).expect("booking has an id");
    sqlx::query_scalar!(
        "UPDATE bookings SET slot_start = now() - interval '1 minute' WHERE unid = $1 RETURNING 1",
        again_unid,
    )
    .fetch_one(&pool)
    .await
    .expect("booking exists");
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/bookings/{again_unid}/cancel"),
        Some(&host.cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "already started");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_host_edits_their_own_hours_in_place(pool: PgPool) {
    let app = app(&pool);
    let (host, _) = host_with_hours(&app, "nino@example.com").await;
    let other = register(&app, "Grace Hopper", "grace@example.com").await;
    let (_, rules) = get(&app, "/api/availability", Some(&host.cookie)).await;
    let id = rules[0]["id"].as_i64().expect("rule has an id");
    let path = format!("/api/availability/{id}");
    let afternoon = json!({
        "weekday": 3,
        "start_time": "13:00:00",
        "end_time": "15:00:00",
        "slot_minutes": 30,
    });

    let (status, _) = send(&app, Method::PUT, &path, None, Some(afternoon.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(
        &app,
        Method::PUT,
        &path,
        Some(&other.cookie),
        Some(afternoon.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "another host's hours");
    let (status, _) = send(
        &app,
        Method::PUT,
        "/api/availability/0",
        Some(&host.cookie),
        Some(afternoon.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for (field, bad) in [
        ("end_time", json!("13:00:00")),
        ("weekday", json!(7)),
        ("slot_minutes", json!(2)),
    ] {
        let mut changed = afternoon.clone();
        changed[field] = bad.clone();
        let (status, _) = send(&app, Method::PUT, &path, Some(&host.cookie), Some(changed)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field} = {bad}");
    }
    let (_, unchanged) = get(&app, "/api/availability", Some(&host.cookie)).await;
    assert_eq!(unchanged, rules);

    let (status, saved) = send(
        &app,
        Method::PUT,
        &path,
        Some(&host.cookie),
        Some(afternoon.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["id"], id);
    assert_eq!(saved["host_unid"], json!(host.unid));
    for field in ["weekday", "start_time", "end_time", "slot_minutes"] {
        assert_eq!(saved[field], afternoon[field], "{field}");
    }
    let (_, after) = get(&app, "/api/availability", Some(&host.cookie)).await;
    let after = after.as_array().expect("rules are a list");
    assert_eq!(after.len(), 7, "edited in place, not added");
    assert!(after.contains(&saved));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_date_with_its_own_hours_changes_the_open_times(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let other = register(&app, "Grace Hopper", "grace@example.com").await;
    // Three days out in Tbilisi (UTC+4), well inside the window and notice.
    let day = (Utc::now() + Duration::hours(4)).date_naive() + Duration::days(3);
    let path = format!("/api/availability/overrides/{day}");
    let on_day = |slots: &[DateTime<Utc>]| -> Vec<u32> {
        slots
            .iter()
            .filter(|start| (**start + Duration::hours(4)).date_naive() == day)
            .map(Timelike::hour)
            .collect()
    };
    assert_eq!(on_day(&slots(&app, &slug).await), vec![5]);

    let (status, _) = send(
        &app,
        Method::PUT,
        &path,
        None,
        Some(json!({ "ranges": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(
        &app,
        Method::PUT,
        &path,
        Some(&host.cookie),
        Some(json!({ "ranges": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(on_day(&slots(&app, &slug).await).is_empty(), "off all day");

    let overlapping = json!({ "ranges": [
        { "start_time": "14:00:00", "end_time": "16:00:00", "slot_minutes": 60 },
        { "start_time": "15:00:00", "end_time": "17:00:00", "slot_minutes": 60 },
    ] });
    let (status, _) = send(
        &app,
        Method::PUT,
        &path,
        Some(&host.cookie),
        Some(overlapping),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let afternoon = json!({ "ranges": [
        { "start_time": "14:00:00", "end_time": "16:00:00", "slot_minutes": 60 },
    ] });
    let (status, saved) = send(
        &app,
        Method::PUT,
        &path,
        Some(&host.cookie),
        Some(afternoon),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["date"], json!(day));
    // 14:00 and 15:00 in Tbilisi, in place of the usual 09:00.
    assert_eq!(on_day(&slots(&app, &slug).await), vec![10, 11]);
    let (_, listed) = get(&app, "/api/availability/overrides", Some(&host.cookie)).await;
    assert_eq!(listed, json!([saved]));
    let (_, theirs) = get(&app, "/api/availability/overrides", Some(&other.cookie)).await;
    assert_eq!(theirs, json!([]), "another host sees none of it");

    let (status, _) = send(&app, Method::DELETE, &path, Some(&other.cookie), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "another host's date");
    let (status, _) = send(&app, Method::DELETE, &path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        on_day(&slots(&app, &slug).await),
        vec![5],
        "weekly hours again"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn an_invitee_reads_their_call_through_the_manage_link(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let admin = register(&app, "Root Admin", "root.admin@example.com").await;
    grant_admin(&pool, &admin).await;
    let start = *slots(&app, &slug).await.last().expect("open times");
    let (unid, token) = book(&app, &slug, start).await;
    assert_eq!(token.len(), 48);

    let (status, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        call,
        json!({
            "slot_start": start,
            "timezone": "Europe/Paris",
            "minutes": 60,
            "status": "confirmed",
            "invitee_name": "Ada Lovelace",
            "meeting_url": null,
            "host_name": "Nino Tbilisi",
            "title": "Book your call",
            "slug": slug,
            "event_slug": "call",
            "can_change": true,
            "attendance_confirmed": false,
        })
    );
    for wrong in ["nope".to_owned(), unid.to_string()] {
        let (status, _) = get(&app, &format!("/api/manage/{wrong}"), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{wrong}");
    }

    // The link is the invitee's secret: no host or admin view carries it.
    for (path, cookie) in [
        ("/api/bookings".to_owned(), &host.cookie),
        (format!("/api/bookings/{unid}"), &host.cookie),
        ("/api/admin/bookings".to_owned(), &admin.cookie),
    ] {
        let (status, body) = get(&app, &path, Some(cookie)).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        let body = body.to_string();
        assert!(body.contains(&unid.to_string()), "{path}");
        assert!(!body.contains("manage_token"), "{path}");
        assert!(!body.contains(&token), "{path}");
    }

    // The host hears about the new call; the invitee's emails carry the link.
    // Which emails a booking sends, and when, is the email queue's to test.
    let waiting = waiting_emails(&pool, unid).await;
    for email in [
        "booking_confirmation -> ada@example.com",
        "host_new_booking -> nino@example.com",
    ] {
        assert!(waiting.iter().any(|line| line == email), "{email}");
    }
    let without_link = sqlx::query_scalar!(
        "SELECT count(*) FROM email_queue
         WHERE booking_unid = $1 AND email = 'ada@example.com'
           AND (payload->>'manage_url' NOT LIKE '%/booking/' || $2
                OR payload->>'host_name' <> 'Nino Tbilisi' OR payload->>'minutes' <> '60')",
        unid,
        token,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(without_link, Some(0));
    let host_timezone = sqlx::query_scalar!(
        "SELECT payload->>'timezone' FROM email_queue
         WHERE booking_unid = $1 AND template = 'host_new_booking'",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(host_timezone.as_deref(), Some("Asia/Tbilisi"));
}

#[sqlx::test(migrations = "../migrations")]
async fn an_invitee_cancels_their_own_call(pool: PgPool) {
    let app = app(&pool);
    let (_, slug) = host_with_hours(&app, "nino@example.com").await;
    let open = slots(&app, &slug).await;
    let start = *open.last().expect("open times");
    let (unid, token) = book(&app, &slug, start).await;
    let cancel_path = format!("/api/manage/{token}/cancel");
    assert!(!slots(&app, &slug).await.contains(&start));

    let (status, _) = send(&app, Method::POST, "/api/manage/nope/cancel", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, cancelled) = send(&app, Method::POST, &cancel_path, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["can_change"], false);
    assert_eq!(slots(&app, &slug).await, open, "the time is open again");

    // The reminders are gone, the host is told, and the invitee who just
    // cancelled is not told again.
    assert_eq!(
        waiting_emails(&pool, unid).await,
        ["host_booking_cancelled -> nino@example.com"]
    );

    let (status, _) = send(&app, Method::POST, &cancel_path, None, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "already cancelled");
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/manage/{token}/reschedule"),
        None,
        Some(json!({ "slot_start": open[0] })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "a cancelled call stays put");
    let (status, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(call["status"], "cancelled");
    assert_eq!(call["can_change"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_invitee_moves_their_call_to_another_open_time(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let open = slots(&app, &slug).await;
    // Far enough out that every reminder is queued, before and after.
    let [.., taken, new, old] = open[..] else {
        panic!("the host has at least three open times");
    };
    let (unid, token) = book(&app, &slug, old).await;
    book(&app, &slug, taken).await;
    let path = format!("/api/manage/{token}/reschedule");
    let to = |start: DateTime<Utc>| Some(json!({ "slot_start": start }));

    let (status, _) = send(
        &app,
        Method::POST,
        "/api/manage/nope/reschedule",
        None,
        to(new),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for (start, why) in [
        (new + Duration::minutes(7), "never offered"),
        (taken, "someone else's call"),
        (old, "its own time"),
    ] {
        let (status, _) = send(&app, Method::POST, &path, None, to(start)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{why}");
    }

    let (status, moved) = send(&app, Method::POST, &path, None, to(new)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved["slot_start"], json!(new));
    assert_eq!(moved["status"], "confirmed");
    assert_eq!(moved["can_change"], true);
    assert_eq!(moved["minutes"], 60);

    // The same link still opens the call, now at its new time.
    let (status, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(call, moved);
    let now_open = slots(&app, &slug).await;
    assert!(now_open.contains(&old), "the old time is open again");
    assert!(!now_open.contains(&new));
    let (_, mine) = get(&app, "/api/bookings", Some(&host.cookie)).await;
    assert_eq!(mine.as_array().map(Vec::len), Some(2), "moved, not copied");

    // The old emails are replaced and both sides are told.
    let waiting = waiting_emails(&pool, unid).await;
    for email in [
        "booking_rescheduled -> ada@example.com",
        "host_booking_rescheduled -> nino@example.com",
    ] {
        assert!(waiting.iter().any(|line| line == email), "{email}");
    }
    let stale = sqlx::query_scalar!(
        "SELECT count(*) FROM email_queue
         WHERE booking_unid = $1 AND (payload->>'slot_start')::timestamptz <> $2",
        unid,
        new,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(stale, Some(0));

    // The freed time can be taken by someone else.
    book(&app, &slug, old).await;

    // A paused page offers no other time, and a started call stays put.
    let mut paused = settings(&app, &host).await;
    paused["is_active"] = json!(false);
    let (status, _) = save_settings(&app, &host, &paused).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&app, Method::POST, &path, None, to(open[0])).await;
    assert_eq!(status, StatusCode::CONFLICT, "paused");
    paused["is_active"] = json!(true);
    let (status, _) = save_settings(&app, &host, &paused).await;
    assert_eq!(status, StatusCode::OK);
    sqlx::query_scalar!(
        "UPDATE bookings SET slot_start = now() - interval '1 minute' WHERE unid = $1 RETURNING 1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("booking exists");
    let (status, _) = send(&app, Method::POST, &path, None, to(open[0])).await;
    assert_eq!(status, StatusCode::CONFLICT, "already started");
    let (_, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(call["can_change"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn calendar_and_old_public_routes_are_closed(pool: PgPool) {
    let app = app(&pool);

    for path in [
        "/api/google-calendar/events",
        "/api/google-calendar/oauth-config",
    ] {
        let (status, _) = get(&app, path, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/google-calendar/login",
        None,
        Some(json!({ "code": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // These used to act on whichever host saved their settings last.
    let lead = Uuid::new_v4();
    for path in [
        format!("/api/leads/{lead}"),
        "/api/google-calendar/book".to_owned(),
        "/api/google-calendar/available-slots".to_owned(),
    ] {
        let (status, _) = send(&app, Method::POST, &path, None, Some(json!({}))).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
}

const ADMIN_PATHS: [&str; 4] = [
    "/api/admin/hosts",
    "/api/admin/emails",
    "/api/admin/roleaccesses",
    "/api/admin/calling-visits/funnel",
];

async fn grant_admin(pool: &PgPool, user: &Host) {
    sqlx::query_scalar!(
        "INSERT INTO roleaccesses (grantedto_unid, role) VALUES ($1, 'Admin') RETURNING 1",
        user.unid,
    )
    .fetch_one(pool)
    .await
    .expect("role is granted");
}

#[sqlx::test(migrations = "../migrations")]
async fn admin_lists_are_closed_to_hosts(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "Plain Host", "plain@example.com").await;

    for path in ADMIN_PATHS {
        let (status, _) = get(&app, path, Some(&host.cookie)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn admin_sees_hosts_roles_and_the_funnel(pool: PgPool) {
    let app = app(&pool);
    let admin = register(&app, "Ada Admin", "ada@example.com").await;
    grant_admin(&pool, &admin).await;
    // Never opened their dashboard, so they have no settings row.
    let fresh = register(&app, "Fresh Host", "fresh@example.com").await;
    let (_, slug_with_hours) = host_with_hours(&app, "nino@example.com").await;

    let (status, hosts) = get(&app, "/api/admin/hosts", Some(&admin.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let hosts = hosts.as_array().expect("hosts is a list");
    let fresh_row = hosts
        .iter()
        .find(|host| host["unid"] == json!(fresh.unid))
        .expect("the new host is listed");
    assert_eq!(fresh_row["public_slug"], Value::Null);
    assert_eq!(fresh_row["has_hours"], json!(false));
    assert_eq!(fresh_row["has_meeting_link"], json!(false));
    let with_hours = hosts
        .iter()
        .find(|host| host["public_slug"] == json!(slug_with_hours))
        .expect("the host with hours is listed");
    assert_eq!(with_hours["has_hours"], json!(true));

    let (status, roles) = get(&app, "/api/admin/roleaccesses", Some(&admin.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        roles
            .as_array()
            .expect("roles is a list")
            .iter()
            .any(|role| role["email"] == json!("ada@example.com") && role["role"] == json!("Admin"))
    );

    let (status, funnel) = get(
        &app,
        "/api/admin/calling-visits/funnel",
        Some(&admin.cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let count = |key: &str| funnel[key].as_i64().expect("count is a number");
    assert!(count("visits") >= count("form_submitted"));
}

#[sqlx::test(migrations = "../migrations")]
async fn admin_emails_show_state_without_the_payload(pool: PgPool) {
    let app = app(&pool);
    let admin = register(&app, "Ada Admin", "ada@example.com").await;
    grant_admin(&pool, &admin).await;

    for (email, due_in_minutes, sent, attempts) in [
        ("sent@example.com", -60, true, 0),
        ("retrying@example.com", 0, false, 2),
        ("overdue@example.com", -60, false, 0),
        ("scheduled@example.com", 60, false, 0),
        (
            "last-try@example.com",
            -60,
            false,
            emails::db::MAX_ATTEMPTS - 1,
        ),
        ("failed@example.com", -60, false, emails::db::MAX_ATTEMPTS),
    ] {
        sqlx::query_scalar!(
            r#"INSERT INTO email_queue (email, template, send_at, sent_at, attempts, payload)
               VALUES ($1, 'booking_confirmation', now() + make_interval(mins => $2),
                       CASE WHEN $3::bool THEN now() END, $4,
                       '{"meeting_url":"https://secret.example.com"}')
               RETURNING 1"#,
            email,
            due_in_minutes,
            sent,
            attempts,
        )
        .fetch_one(&pool)
        .await
        .expect("email is queued");
    }

    let (status, emails) = get(&app, "/api/admin/emails", Some(&admin.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let emails = emails.as_array().expect("emails is a list");
    for (email, state) in [
        ("sent@example.com", "Sent"),
        ("retrying@example.com", "Retrying"),
        ("overdue@example.com", "Overdue"),
        ("scheduled@example.com", "Scheduled"),
        ("last-try@example.com", "Retrying"),
        ("failed@example.com", "Failed"),
    ] {
        let row = emails
            .iter()
            .find(|row| row["email"] == json!(email))
            .expect("the email is listed");
        assert_eq!(row["state"], json!(state), "{email}");
        assert!(row.get("payload").is_none());
    }
}

/// Stands in for SMTP: remembers who was written to.
#[derive(Default)]
struct Outbox {
    sent: std::sync::Mutex<Vec<String>>,
    /// Sends started so far, counted across the outboxes that share it.
    started: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// A send waits until this many have started, so workers that share
    /// `started` are in the middle of a send at the same time.
    wait_for: usize,
}

impl Outbox {
    /// Everything sent so far as "subject -> recipient", sorted.
    fn sent(&self) -> Vec<String> {
        let mut sent = self.sent.lock().expect("outbox is not poisoned").clone();
        sent.sort();
        sent
    }
}

impl Mailer for Outbox {
    async fn send(&self, recipient: &str, email: RenderedEmail) -> Result<(), String> {
        use std::sync::atomic::Ordering;
        self.started.fetch_add(1, Ordering::SeqCst);
        // Gives up after five seconds, so a worker that never gets a row
        // fails the test instead of hanging it.
        for _ in 0..2500 {
            if self.started.load(Ordering::SeqCst) >= self.wait_for {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        self.sent
            .lock()
            .expect("outbox is not poisoned")
            .push(format!("{} -> {recipient}", email.subject));
        Ok(())
    }
}

/// Queues an email about no call, due `minutes_ago`.
async fn queue_email(pool: &PgPool, email: &str, template: &str, minutes_ago: i32) {
    sqlx::query_scalar!(
        r#"INSERT INTO email_queue (email, template, send_at, payload)
           VALUES ($1, $2, now() - make_interval(mins => $3), '{"name":"Ada"}')
           RETURNING 1"#,
        email,
        template,
        minutes_ago,
    )
    .fetch_one(pool)
    .await
    .expect("email is queued");
}

/// Makes every waiting email of a call due now.
async fn make_due(pool: &PgPool, booking_unid: Uuid) {
    sqlx::query_scalar!(
        "UPDATE email_queue SET send_at = now() - interval '1 minute'
         WHERE booking_unid = $1 AND sent_at IS NULL RETURNING 1",
        booking_unid,
    )
    .fetch_all(pool)
    .await
    .expect("queue is writable");
}

#[sqlx::test(migrations = "../migrations")]
async fn an_email_out_of_attempts_is_left_alone_and_a_sent_one_forgets_its_error(pool: PgPool) {
    for (email, attempts) in [
        ("last-try@example.com", emails::db::MAX_ATTEMPTS - 1),
        ("failed@example.com", emails::db::MAX_ATTEMPTS),
    ] {
        sqlx::query_scalar!(
            "INSERT INTO email_queue (email, template, send_at, attempts, last_error)
             VALUES ($1, 'booking_confirmation', now() - interval '1 hour', $2, '550 rejected')
             RETURNING 1",
            email,
            attempts,
        )
        .fetch_one(&pool)
        .await
        .expect("email is queued");
    }

    let outbox = Outbox::default();
    worker::process_due(&pool, &outbox).await;
    assert_eq!(
        outbox.sent(),
        ["Your call is booked -> last-try@example.com"]
    );
    let rows = sqlx::query_scalar!(
        r#"SELECT email || ' sent=' || (sent_at IS NOT NULL)::text || ' error=' || COALESCE(last_error, '-') AS "row!"
           FROM email_queue ORDER BY email"#,
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(
        rows,
        [
            "failed@example.com sent=false error=550 rejected",
            "last-try@example.com sent=true error=-",
        ],
        "sent, and the old error is gone"
    );

    worker::process_due(&pool, &outbox).await;
    assert_eq!(outbox.sent().len(), 1, "nothing is sent twice");
}

#[sqlx::test(migrations = "../migrations")]
async fn two_workers_send_every_due_email_once(pool: PgPool) {
    let recipients: Vec<String> = (0..8).map(|n| format!("invitee{n}@example.com")).collect();
    for recipient in &recipients {
        queue_email(&pool, recipient, "booking_confirmation", 1).await;
    }
    queue_email(&pool, "later@example.com", "booking_confirmation", -60).await;

    // Each worker holds its first email until the other has one too.
    let started = std::sync::Arc::default();
    let worker = || Outbox {
        started: std::sync::Arc::clone(&started),
        wait_for: 2,
        ..Outbox::default()
    };
    let (first, second) = (worker(), worker());
    tokio::join!(
        worker::process_due(&pool, &first),
        worker::process_due(&pool, &second),
    );
    let mut sent = first.sent();
    sent.extend(second.sent());
    sent.sort();
    let expected: Vec<String> = recipients
        .iter()
        .map(|recipient| format!("Your call is booked -> {recipient}"))
        .collect();
    assert_eq!(sent, expected);
    assert!(
        !first.sent().is_empty() && !second.sent().is_empty(),
        "both workers took a share"
    );
    let waiting = sqlx::query_scalar!("SELECT email FROM email_queue WHERE sent_at IS NULL")
        .fetch_all(&pool)
        .await
        .expect("queue is readable");
    assert_eq!(waiting, ["later@example.com"], "not due yet");
}

#[sqlx::test(migrations = "../migrations")]
async fn an_unknown_template_fails_without_holding_up_the_queue(pool: PgPool) {
    let app = app(&pool);
    register(&app, "Ada Lovelace", "ada@example.com").await;
    // The oldest row is the one no template renders.
    queue_email(&pool, "first@example.com", "newsletter", 30).await;
    queue_email(&pool, "second@example.com", "booking_confirmation", 1).await;
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/auth/forgot-password",
        None,
        Some(json!({ "email": "ada@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let outbox = Outbox::default();
    worker::process_due(&pool, &outbox).await;
    assert_eq!(
        outbox.sent(),
        [
            "Reset your Cadence password -> ada@example.com",
            "Your call is booked -> second@example.com",
        ]
    );
    let rows = sqlx::query_scalar!(
        r#"SELECT template || ' sent=' || (sent_at IS NOT NULL)::text
                  || ' attempts=' || attempts::text
                  || ' later=' || (send_at > now())::text
                  || ' error=' || COALESCE(last_error, '-')
                  || ' payload=' || (payload IS NOT NULL)::text AS "row!"
           FROM email_queue
           -- Every new host also has a trial notice waiting.
           WHERE template <> 'trial_ending' ORDER BY template"#,
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(
        rows,
        [
            "booking_confirmation sent=true attempts=0 later=false error=- payload=true",
            "newsletter sent=false attempts=1 later=true error=unknown email template payload=true",
            // The reset link is a password in waiting; it does not stay in the table.
            "password_reset sent=true attempts=0 later=false error=- payload=false",
        ]
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn emails_about_a_cancelled_call_are_dropped(pool: PgPool) {
    // Which emails a started or cancelled call still sends is `delivery`'s to
    // test; here, that the call's status reaches the worker from its row.
    let app = app(&pool);
    let (_, slug) = host_with_hours(&app, "nino@example.com").await;
    let start = *slots(&app, &slug).await.last().expect("open times");

    // This call was cancelled with its emails still waiting.
    let (cancelled, _) = book(&app, &slug, start).await;
    make_due(&pool, cancelled).await;
    sqlx::query_scalar!(
        "UPDATE bookings SET status = 'cancelled', cancelled_at = now() WHERE unid = $1 RETURNING 1",
        cancelled,
    )
    .fetch_one(&pool)
    .await
    .expect("booking exists");
    sqlx::query_scalar!(
        r#"INSERT INTO email_queue (booking_unid, email, template, send_at, payload)
           VALUES ($1, 'ada@example.com', 'booking_cancelled', now(), '{"name":"Ada"}')
           RETURNING 1"#,
        cancelled,
    )
    .fetch_one(&pool)
    .await
    .expect("email is queued");

    let outbox = Outbox::default();
    worker::process_due(&pool, &outbox).await;
    assert_eq!(
        outbox.sent(),
        ["Your call was cancelled -> ada@example.com"]
    );
    // What was not sent is gone, so no list shows it as sent or waiting.
    let left = sqlx::query_scalar!(
        r#"SELECT template || ' sent=' || (sent_at IS NOT NULL)::text AS "row!"
           FROM email_queue
           -- Every new host also has a trial notice waiting.
           WHERE template <> 'trial_ending' ORDER BY template"#,
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(left, ["booking_cancelled sent=true"]);
}

#[sqlx::test(migrations = "../migrations")]
async fn the_sweep_removes_what_nothing_reads_any_more(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "Ada Lovelace", "ada@example.com").await;
    for (email, sent_days_ago) in [("old@example.com", 91), ("recent@example.com", 89)] {
        sqlx::query_scalar!(
            "INSERT INTO email_queue (email, template, send_at, sent_at)
             VALUES ($1, 'booking_confirmation', now() - make_interval(days => $2),
                     now() - make_interval(days => $2))
             RETURNING 1",
            email,
            sent_days_ago,
        )
        .fetch_one(&pool)
        .await
        .expect("email is queued");
    }
    queue_email(
        &pool,
        "waiting@example.com",
        "booking_confirmation",
        200_000,
    )
    .await;
    for (token_hash, hours_ago) in [("old", 25), ("recent", 23)] {
        sqlx::query_scalar!(
            "INSERT INTO password_resets (token_hash, user_unid, expires_at, created_at)
             VALUES ($1, $2, now(), now() - make_interval(hours => $3)) RETURNING 1",
            token_hash,
            host.unid,
            hours_ago,
        )
        .fetch_one(&pool)
        .await
        .expect("reset is stored");
    }
    for (state, expired_hours_ago) in [("old", 25), ("recent", 23)] {
        GoogleCalendarDb::create_state(
            &pool,
            state,
            host.unid,
            "http://localhost:3000/callback",
            Utc::now() - Duration::hours(expired_hours_ago),
        )
        .await
        .expect("state is stored");
    }

    worker::sweep(&pool).await;

    // Every host registered here also has a trial notice waiting, weeks ahead.
    let emails = sqlx::query_scalar!(
        "SELECT email FROM email_queue WHERE template <> 'trial_ending' ORDER BY email"
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(
        emails,
        ["recent@example.com", "waiting@example.com"],
        "an unsent email is kept however old it is"
    );
    let resets = sqlx::query_scalar!("SELECT token_hash FROM password_resets")
        .fetch_all(&pool)
        .await
        .expect("resets are readable");
    assert_eq!(resets, ["recent"]);
    let states = sqlx::query_scalar!("SELECT state FROM oauth_states")
        .fetch_all(&pool)
        .await
        .expect("states are readable");
    assert_eq!(states, ["recent"]);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_calendar_is_connected_only_with_the_state_its_host_was_given(pool: PgPool) {
    let app = app(&pool);
    let ada = register(&app, "Ada Lovelace", "ada@example.com").await;
    let grace = register(&app, "Grace Hopper", "grace@example.com").await;
    let in_ten_minutes = Utc::now() + Duration::minutes(10);
    for (state, expires_at) in [
        ("adas-state", in_ten_minutes),
        ("adas-used-state", in_ten_minutes),
        ("adas-expired-state", Utc::now() - Duration::minutes(1)),
    ] {
        GoogleCalendarDb::create_state(&pool, state, ada.unid, "http://localhost/cb", expires_at)
            .await
            .expect("state is stored");
    }
    // A state works once.
    let spend = |state: &'static str| GoogleCalendarDb::consume_state(&pool, state, ada.unid);
    assert!(spend("adas-used-state").await.expect("readable").is_some());
    assert!(spend("adas-used-state").await.expect("readable").is_none());

    // Every refusal below happens before anything is sent to Google.
    let login = |host: &Host, body: Value| {
        let (app, cookie) = (app.clone(), host.cookie.clone());
        async move {
            send(
                &app,
                Method::POST,
                "/api/google-calendar/login",
                Some(&cookie),
                Some(body),
            )
            .await
        }
    };
    let (status, _) = login(&ada, json!({ "code": "x" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "no state");
    for (host, state, why) in [
        (&ada, "", "empty"),
        (&ada, "never-issued", "unknown"),
        (&ada, "adas-used-state", "already used"),
        (&ada, "adas-expired-state", "expired"),
        (&grace, "adas-state", "another host's"),
    ] {
        let (status, body) = login(host, json!({ "code": "x", "state": state })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{why}");
        assert_eq!(
            body,
            json!({
                "ok": false,
                "error": "this Google sign-in has expired, connect Google Calendar again",
            }),
            "{why}"
        );
    }
    let connected = sqlx::query_scalar!("SELECT count(*) FROM google_calendar_config")
        .fetch_one(&pool)
        .await
        .expect("config is readable");
    assert_eq!(connected, Some(0));
    // Someone else trying Ada's state did not spend it for her.
    assert!(spend("adas-state").await.expect("readable").is_some());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_booking_with_bad_details_is_refused_and_takes_no_time(pool: PgPool) {
    let app = app(&pool);
    let (_, slug) = host_with_hours(&app, "nino@example.com").await;
    let open = slots(&app, &slug).await;
    let start = open[0];
    let bookings_path = format!("/api/hosts/{slug}/types/call/bookings");
    let try_booking = |field: &'static str, value: Value| {
        let mut body = booking(start, None);
        body[field] = value;
        send(&app, Method::POST, &bookings_path, None, Some(body))
    };

    for (field, value, error) in [
        ("invitee_email", json!("ada"), "invitee_email is invalid"),
        (
            "invitee_email",
            json!("ada@example"),
            "invitee_email is invalid",
        ),
        (
            "invitee_email",
            json!("ada lovelace@example.com"),
            "invitee_email is invalid",
        ),
        (
            "invitee_name",
            json!("   "),
            "invitee_name must be 1 to 120 characters",
        ),
        (
            "invitee_name",
            json!("a".repeat(121)),
            "invitee_name must be 1 to 120 characters",
        ),
        ("timezone", json!("Mars/Olympus"), "timezone is invalid"),
    ] {
        let (status, body) = try_booking(field, value.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field} {value}");
        assert_eq!(body["error"], format!("invalid input: {error}"));
    }

    // A visit that was never recorded is bad input, not a server failure.
    let (status, body) = try_booking("calling_visit_unid", json!(Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid input: booking data is invalid");

    assert_eq!(slots(&app, &slug).await, open, "nothing was booked");
    let (status, _) = try_booking("invitee_name", json!("a".repeat(120))).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_cancelled_call_no_longer_shows_where_to_join(pool: PgPool) {
    const LINK: &str = "https://zoom.us/j/123456789";
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let (status, _) = save_conferencing(&app, &host, "custom_link", Some(LINK)).await;
    assert_eq!(status, StatusCode::OK);
    let start = slots(&app, &slug).await[0];
    let (_, token) = book(&app, &slug, start).await;
    let manage_path = format!("/api/manage/{token}");

    let (status, call) = get(&app, &manage_path, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(call["meeting_url"], LINK);

    let (status, cancelled) = send(
        &app,
        Method::POST,
        &format!("{manage_path}/cancel"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["meeting_url"], Value::Null);
    let (status, call) = get(&app, &manage_path, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(call["status"], "cancelled");
    assert_eq!(call["meeting_url"], Value::Null);
}

#[sqlx::test(migrations = "../migrations")]
async fn reading_open_times_is_limited(pool: PgPool) {
    const LIMIT: usize = 120;
    let app = app(&pool);
    let (_, slug) = host_with_hours(&app, "nino@example.com").await;
    let path = format!("/api/hosts/{slug}/types/call/slots");

    for attempt in 0..LIMIT {
        let (status, _) = get(&app, &path, None).await;
        assert_eq!(status, StatusCode::OK, "attempt {attempt}");
    }
    // A fixed window can roll over in the middle of the test, so the refusal
    // is one of the next requests, not exactly the next one.
    let mut refused = false;
    for _ in LIMIT..=LIMIT * 2 {
        let (status, error) = get(&app, &path, None).await;
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
async fn an_oversized_request_is_refused_before_it_is_read(pool: PgPool) {
    let app = app(&pool);
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/calling-visits",
        None,
        Some(json!({ "source_page": "a".repeat(70 * 1024) })),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    // Within the size limit, the field's own limit answers.
    for (field, length) in [
        ("source_page", 2001),
        ("user_agent", 501),
        ("utm_source", 201),
    ] {
        let (status, error) = send(
            &app,
            Method::POST,
            "/api/calling-visits",
            None,
            Some(json!({ field: "a".repeat(length) })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(
            error["error"],
            format!("invalid input: {field} is too long")
        );
    }

    // A bug report with a long stack trace still fits.
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/bug-reports",
        None,
        Some(json!({
            "bug_type": "JsError",
            "message": "TypeError: x is undefined",
            "stack_trace": "a".repeat(100 * 1024),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// A host in Tbilisi (UTC+4) with the same hours every day, each given as
/// start, end and call length, and `buffer` minutes kept between calls.
async fn host_in_tbilisi(app: &Router, buffer: i32, hours: &[(&str, &str, i32)]) -> (Host, String) {
    let host = register(app, "Nino Tbilisi", "nino@example.com").await;
    let mut settings = settings(app, &host).await;
    settings["timezone"] = json!("Asia/Tbilisi");
    let (status, settings) = save_settings(app, &host, &settings).await;
    assert_eq!(status, StatusCode::OK);
    let mut kind = first_type(app, &host).await;
    kind["buffer_minutes"] = json!(buffer);
    let (status, _) = save_type(app, &host, &kind).await;
    assert_eq!(status, StatusCode::OK);
    for weekday in 0..7 {
        for (start_time, end_time, slot_minutes) in hours {
            let (status, _) = send(
                app,
                Method::POST,
                "/api/availability",
                Some(&host.cookie),
                Some(json!({
                    "weekday": weekday,
                    "start_time": start_time,
                    "end_time": end_time,
                    "slot_minutes": slot_minutes,
                })),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
        }
    }
    let slug = slug(&settings).to_owned();
    (host, slug)
}

/// A time on the host's clock in Tbilisi, `days` days from today: far enough
/// out to be past the notice period and inside the booking window.
fn tbilisi(days: i64, hour: u32, minute: u32) -> DateTime<Utc> {
    (Utc::now() + Duration::days(days))
        .date_naive()
        .and_hms_opt(hour, minute, 0)
        .expect("the time exists")
        .and_utc()
        - Duration::hours(4)
}

/// The open times on the day `day` falls on, as "HH:MM/length" on the host's
/// clock in Tbilisi.
async fn open_on(app: &Router, slug: &str, day: DateTime<Utc>) -> Vec<String> {
    let (status, slots) = get(app, &format!("/api/hosts/{slug}/types/call/slots"), None).await;
    assert_eq!(status, StatusCode::OK);
    slots
        .as_array()
        .expect("slots are a list")
        .iter()
        .map(|slot| {
            let start: DateTime<Utc> =
                serde_json::from_value(slot["start"].clone()).expect("slot has a start");
            (start, slot["minutes"].as_i64().expect("slot has a length"))
        })
        .filter(|(start, _)| start.date_naive() == day.date_naive())
        .map(|(start, minutes)| {
            format!("{}/{minutes}", (start + Duration::hours(4)).format("%H:%M"))
        })
        .collect()
}

async fn try_to_book(app: &Router, slug: &str, start: DateTime<Utc>) -> StatusCode {
    send(
        app,
        Method::POST,
        &format!("/api/hosts/{slug}/types/call/bookings"),
        None,
        Some(booking(start, None)),
    )
    .await
    .0
}

async fn try_to_move(app: &Router, token: &str, start: DateTime<Utc>) -> StatusCode {
    send(
        app,
        Method::POST,
        &format!("/api/manage/{token}/reschedule"),
        None,
        Some(json!({ "slot_start": start })),
    )
    .await
    .0
}

#[sqlx::test(migrations = "../migrations")]
async fn hours_that_overlap_never_open_two_calls_at_once(pool: PgPool) {
    let app = app(&pool);
    let hours = [("09:00:00", "11:00:00", 60), ("09:30:00", "10:30:00", 30)];
    let (_, slug) = host_in_tbilisi(&app, 0, &hours).await;
    let nine = tbilisi(3, 9, 0);
    let half_hour = Duration::minutes(30);
    // 10:00 is opened by both sets of hours; the earlier one names its length.
    assert_eq!(
        open_on(&app, &slug, nine).await,
        ["09:00/60", "09:30/30", "10:00/60"]
    );

    book(&app, &slug, nine).await;
    assert_eq!(open_on(&app, &slug, nine).await, ["10:00/60"]);
    assert_eq!(
        try_to_book(&app, &slug, nine + half_hour).await,
        StatusCode::CONFLICT,
        "09:30 is inside the 09:00 call"
    );
    // A call may start the minute another ends.
    book(&app, &slug, nine + half_hour * 2).await;
    assert!(open_on(&app, &slug, nine).await.is_empty());

    // The other way round: the half hour in the middle closes the hour around it.
    let next_nine = tbilisi(4, 9, 0);
    book(&app, &slug, next_nine + half_hour).await;
    assert_eq!(open_on(&app, &slug, next_nine).await, ["10:00/60"]);
    assert_eq!(
        try_to_book(&app, &slug, next_nine).await,
        StatusCode::CONFLICT,
        "09:00 to 10:00 would run over the 09:30 call"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn two_visitors_booking_at_once_cannot_both_win(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_in_tbilisi(&app, 15, &[("09:00:00", "12:00:00", 30)]).await;
    let ten = tbilisi(3, 10, 0);
    let half_hour = Duration::minutes(30);
    // Both requests find a connection waiting; otherwise the one that has to
    // open its own falls behind and they never overlap.
    let warm = tokio::join!(
        pool.acquire(),
        pool.acquire(),
        pool.acquire(),
        pool.acquire()
    );
    drop(warm);

    // Neighbouring starts: each is free on its own, together they leave no gap.
    let mut neighbours: [StatusCode; 2] = tokio::join!(
        try_to_book(&app, &slug, ten),
        try_to_book(&app, &slug, ten + half_hour),
    )
    .into();
    neighbours.sort();
    assert_eq!(neighbours, [StatusCode::CREATED, StatusCode::CONFLICT]);

    // The very same start, on another day.
    let next_ten = tbilisi(4, 10, 0);
    let mut same: [StatusCode; 2] = tokio::join!(
        try_to_book(&app, &slug, next_ten),
        try_to_book(&app, &slug, next_ten),
    )
    .into();
    same.sort();
    assert_eq!(same, [StatusCode::CREATED, StatusCode::CONFLICT]);

    let (_, mine) = get(&app, "/api/bookings", Some(&host.cookie)).await;
    assert_eq!(mine.as_array().map(Vec::len), Some(2));
    // Only the calls that were booked have emails waiting.
    let with_emails = sqlx::query_scalar!("SELECT count(DISTINCT booking_unid) FROM email_queue")
        .fetch_one(&pool)
        .await
        .expect("queue is readable");
    assert_eq!(with_emails, Some(2));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_call_keeps_its_length_when_the_hours_change(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_in_tbilisi(&app, 0, &[("09:00:00", "10:00:00", 60)]).await;
    let nine = tbilisi(3, 9, 0);
    let (unid, token) = book(&app, &slug, nine).await;

    // The host now takes half-hour calls in the same hour, then none at all.
    let (_, rules) = get(&app, "/api/availability", Some(&host.cookie)).await;
    let rules = rules.as_array().expect("rules are a list").clone();
    for rule in &rules {
        let mut shorter = rule.clone();
        shorter["slot_minutes"] = json!(30);
        let (status, _) = send(
            &app,
            Method::PUT,
            &format!("/api/availability/{}", rule["id"]),
            Some(&host.cookie),
            Some(shorter),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    assert_eq!(
        open_on(&app, &slug, tbilisi(4, 9, 0)).await,
        ["09:00/30", "09:30/30"]
    );
    assert!(
        open_on(&app, &slug, nine).await.is_empty(),
        "both half hours are inside the hour-long call"
    );
    assert_eq!(
        try_to_book(&app, &slug, nine + Duration::minutes(30)).await,
        StatusCode::CONFLICT
    );

    let still_an_hour = || async {
        let (_, mine) = get(&app, "/api/bookings", Some(&host.cookie)).await;
        assert_eq!(mine[0]["minutes"], 60);
        let (_, detail) = get(&app, &format!("/api/bookings/{unid}"), Some(&host.cookie)).await;
        assert_eq!(detail["booking"]["minutes"], 60);
        let (_, call) = get(&app, &format!("/api/manage/{token}"), None).await;
        assert_eq!(call["minutes"], 60);
    };
    still_an_hour().await;
    for rule in &rules {
        let (status, _) = send(
            &app,
            Method::DELETE,
            &format!("/api/availability/{}", rule["id"]),
            Some(&host.cookie),
            None,
        )
        .await;
        assert!(status.is_success(), "{status}");
    }
    still_an_hour().await;

    // Emails queued from here on say an hour too, as the earlier ones did.
    let (status, cancelled) = send(
        &app,
        Method::POST,
        &format!("/api/bookings/{unid}/cancel"),
        Some(&host.cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["minutes"], 60);
    let lengths = sqlx::query_scalar!(
        r#"SELECT DISTINCT payload->>'minutes' AS "minutes!" FROM email_queue WHERE booking_unid = $1"#,
        unid,
    )
    .fetch_all(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(lengths, ["60"]);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_call_can_move_onto_its_own_time_but_not_onto_anothers(pool: PgPool) {
    let app = app(&pool);
    // Hour-long calls starting every half hour from 09:00 to 11:00.
    let hours = [("09:00:00", "12:00:00", 60), ("09:30:00", "11:30:00", 60)];
    let (_, slug) = host_in_tbilisi(&app, 0, &hours).await;
    let at = |hour, minute| tbilisi(3, hour, minute);
    let (_, token) = book(&app, &slug, at(10, 0)).await;
    book(&app, &slug, at(9, 0)).await;
    assert_eq!(open_on(&app, &slug, at(9, 0)).await, ["11:00/60"]);

    // 10:30 to 11:30 runs over the call's own 10:00 to 11:00 and nothing else.
    assert_eq!(try_to_move(&app, &token, at(10, 30)).await, StatusCode::OK);
    assert_eq!(
        try_to_move(&app, &token, at(9, 30)).await,
        StatusCode::CONFLICT,
        "runs over the 09:00 call"
    );
    assert_eq!(try_to_move(&app, &token, at(10, 0)).await, StatusCode::OK);
    // How a gap treats a moving call is the slot policy's to test; this is the wiring.
    let (_, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(call["slot_start"], json!(at(10, 0)));
    assert_eq!(call["minutes"], 60);
}

/// The confirmation email waiting for a call, as its payload.
async fn confirmation_payload(pool: &PgPool, booking_unid: Uuid) -> Value {
    sqlx::query_scalar!(
        "SELECT payload FROM email_queue WHERE booking_unid = $1 AND template = 'booking_confirmation'",
        booking_unid,
    )
    .fetch_one(pool)
    .await
    .expect("the confirmation is queued")
    .unwrap_or(Value::Null)
}

#[sqlx::test(migrations = "../migrations")]
async fn a_busy_calendar_closes_nearby_times_and_a_broken_one_does_not_stop_bookings(pool: PgPool) {
    let (app, calendar) = app_with_calendar(&pool);
    let (host, slug) = host_in_tbilisi(&app, 0, &[("09:00:00", "13:00:00", 60)]).await;
    let all = ["09:00/60", "10:00/60", "11:00/60", "12:00/60"];
    let nine = tbilisi(3, 9, 0);
    assert_eq!(
        open_on(&app, &slug, nine).await,
        all,
        "no calendar, nothing busy"
    );

    // A dentist from 10:00 to 10:30, with 15 minutes kept on both sides.
    calendar.connect(host.unid);
    calendar.add_busy(host.unid, "dentist", tbilisi(3, 10, 0), tbilisi(3, 10, 30));
    assert_eq!(
        open_on(&app, &slug, nine).await,
        ["11:00/60", "12:00/60"],
        "09:00 would end as the dentist starts"
    );
    assert_eq!(try_to_book(&app, &slug, nine).await, StatusCode::CONFLICT);

    // A calendar that cannot be read leaves the hours open and still books.
    calendar.fail(true);
    assert_eq!(open_on(&app, &slug, nine).await, all);
    let (unid, _) = book(&app, &slug, nine).await;
    assert!(calendar.events(host.unid).is_empty(), "no event was made");
    let (_, detail) = get(&app, &format!("/api/bookings/{unid}"), Some(&host.cookie)).await;
    assert_eq!(detail["booking"]["google_event_id"], Value::Null);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_connected_calendar_keeps_an_event_that_follows_the_call(pool: PgPool) {
    let (app, calendar) = app_with_calendar(&pool);
    let (host, slug) = host_in_tbilisi(&app, 0, &[("09:00:00", "13:00:00", 60)]).await;
    calendar.connect(host.unid);
    let ten = tbilisi(3, 10, 0);

    let (unid, token) = book(&app, &slug, ten).await;
    let events = calendar.events(host.unid);
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!((event.start, event.minutes), (ten, 60));
    assert_eq!(event.invitee_email, "ada@example.com");
    // The host chose Google Meet: its link is the one the invitee gets.
    let meet = event.meet_link.clone().expect("Meet was asked for");
    let (_, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(call["meeting_url"], json!(meet));
    assert_eq!(
        confirmation_payload(&pool, unid).await["meeting_url"],
        json!(meet)
    );

    // The call's own event does not close more than the call does.
    assert_eq!(
        open_on(&app, &slug, ten).await,
        ["09:00/60", "11:00/60", "12:00/60"]
    );

    let twelve = tbilisi(3, 12, 0);
    assert_eq!(try_to_move(&app, &token, twelve).await, StatusCode::OK);
    assert_eq!(calendar.events(host.unid)[0].start, twelve);

    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/manage/{token}/cancel"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(calendar.events(host.unid)[0].deleted);
}

/// A booked call as the email queue is told about it, with nothing waiting:
/// the emails the booking itself queued are dropped first.
async fn call_with_empty_queue(
    pool: &PgPool,
    app: &Router,
    start: DateTime<Utc>,
) -> (
    bookings::models::Booking,
    host_settings::models::BookingPage,
) {
    let (host, slug) = host_in_tbilisi(app, 0, &[("09:00:00", "13:00:00", 60)]).await;
    let (unid, _) = book(app, &slug, start).await;
    let booking = bookings::db::find(pool, host.unid, unid)
        .await
        .expect("bookings are readable")
        .expect("the call exists");
    let page = host_settings::service::page_of_booking(pool, unid)
        .await
        .expect("the page exists");
    let mut connection = pool.acquire().await.expect("a connection");
    emails::db::drop_unsent(&mut connection, unid)
        .await
        .expect("queue is writable");
    (booking, page)
}

#[sqlx::test(migrations = "../migrations")]
async fn the_email_queue_decides_what_each_call_event_sends(pool: PgPool) {
    let app = app(&pool);
    let start = tbilisi(3, 10, 0);
    let (booking, page) = call_with_empty_queue(&pool, &app, start).await;
    let mut connection = pool.acquire().await.expect("a connection");

    // Booked 3 hours ahead: the 24 hour reminder is already past.
    let now = start - Duration::hours(3);
    emails::service::booked(&mut connection, &booking, &page, &Reminder::ALL, false, now)
        .await
        .expect("queued");
    assert_eq!(
        waiting_emails(&pool, booking.unid).await,
        [
            "booking_confirmation -> ada@example.com",
            "host_new_booking -> nino@example.com",
            "reminder_2h -> ada@example.com",
            "reminder_30min -> ada@example.com",
        ]
    );

    // Moved, seen from 30 hours ahead: what waited is replaced, every reminder is ahead.
    let now = start - Duration::hours(30);
    emails::service::rescheduled(&mut connection, &booking, &page, &Reminder::ALL, false, now)
        .await
        .expect("queued");
    assert_eq!(
        waiting_emails(&pool, booking.unid).await,
        [
            "booking_rescheduled -> ada@example.com",
            "host_booking_rescheduled -> nino@example.com",
            "reminder_24h -> ada@example.com",
            "reminder_2h -> ada@example.com",
            "reminder_30min -> ada@example.com",
        ]
    );
    let reminder = sqlx::query_scalar!(
        "SELECT send_at FROM email_queue WHERE booking_unid = $1 AND template = 'reminder_30min'",
        booking.unid,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(reminder, start - Duration::minutes(30));

    // Cancelled: nothing waiting goes out; the side that did not cancel is told.
    for (by, told) in [
        (CancelledBy::Host, "booking_cancelled -> ada@example.com"),
        (
            CancelledBy::Invitee,
            "host_booking_cancelled -> nino@example.com",
        ),
    ] {
        emails::service::cancelled(&mut connection, &booking, &page, by, now)
            .await
            .expect("queued");
        assert_eq!(waiting_emails(&pool, booking.unid).await, [told]);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn the_email_worker_stops_when_told_and_not_a_minute_later(pool: PgPool) {
    let (shutdown, shutdown_seen) = tokio::sync::watch::channel(false);
    let running = worker::start(pool, shutdown_seen);
    // Let the first pass run, then ask it to stop during its wait.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    shutdown.send(true).expect("the worker is listening");
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(5), running).await;
    assert!(
        stopped.is_ok(),
        "the worker kept sleeping after being told to stop"
    );
}

/// Ends the host's Pro trial: they are on Free from now on.
async fn end_trial(pool: &PgPool, host: &Host) {
    sqlx::query_scalar!(
        "UPDATE subscriptions SET trial_ends_at = now() - interval '1 minute'
         WHERE user_unid = $1 RETURNING 1",
        host.unid,
    )
    .fetch_one(pool)
    .await
    .expect("the host has a trial");
}

/// `count` calls booked this month, in the past so they hold no open time.
async fn past_calls_this_month(pool: &PgPool, host: &Host, count: i32) {
    sqlx::query_scalar!(
        r#"INSERT INTO bookings (host_unid, slot_start, slot_end, invitee_name, invitee_email)
           SELECT $1, now() - make_interval(hours => n + 1), now() - make_interval(hours => n),
                  'Past caller', 'past@example.com'
           FROM generate_series(1, $2) AS n
           RETURNING 1"#,
        host.unid,
        count,
    )
    .fetch_all(pool)
    .await
    .expect("calls are written");
}

async fn account_notices(pool: &PgPool, template: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM email_queue
           WHERE template = $1 AND booking_unid IS NULL AND sent_at IS NULL"#,
        template,
    )
    .fetch_one(pool)
    .await
    .expect("queue is readable")
}

#[sqlx::test(migrations = "../migrations")]
async fn a_free_host_sends_one_reminder_and_shows_the_logo(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let page = format!("/api/hosts/{slug}/types/call");
    assert_eq!(
        get(&app, &page, None).await.1["show_branding"],
        false,
        "Pro trial"
    );

    end_trial(&pool, &host).await;
    assert_eq!(get(&app, &page, None).await.1["show_branding"], true);
    // The last open time is days ahead, so every reminder would still be ahead.
    let start = *slots(&app, &slug).await.last().expect("open times");
    let (unid, _) = book(&app, &slug, start).await;

    assert_eq!(
        waiting_emails(&pool, unid).await,
        [
            "booking_confirmation -> ada@example.com",
            "host_new_booking -> nino@example.com",
            "reminder_24h -> ada@example.com",
        ]
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_free_page_keeps_taking_calls_past_the_included_month(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    end_trial(&pool, &host).await;
    past_calls_this_month(&pool, &host, 24).await;
    let open = slots(&app, &slug).await;
    let (first, second) = (open[0], open[1]);

    // The 25th call is the last Free includes, and the host hears of it once.
    book(&app, &slug, first).await;
    assert_eq!(account_notices(&pool, "host_booking_limit").await, 1);

    // Nobody booking is turned away past it.
    let (status, _) = get(&app, &format!("/api/hosts/{slug}/types/call/slots"), None).await;
    assert_eq!(status, StatusCode::OK);
    let path = format!("/api/hosts/{slug}/types/call/bookings");
    let (status, _) = send(&app, Method::POST, &path, None, Some(booking(second, None))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        account_notices(&pool, "host_booking_limit").await,
        1,
        "told once a month"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn questions_need_a_paid_plan_and_stay_saved_without_one(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let mut asking = first_type(&app, &host).await;
    asking["ask_urgency_level"] = json!(true);
    let (status, _) = save_type(&app, &host, &asking).await;
    assert_eq!(status, StatusCode::OK, "the trial includes questions");

    end_trial(&pool, &host).await;
    let (_, page) = get(&app, &format!("/api/hosts/{slug}/types/call"), None).await;
    assert_eq!(page["ask_urgency_level"], false, "not asked on Free");
    let mut saved = first_type(&app, &host).await;
    assert_eq!(saved["ask_urgency_level"], true, "still saved");

    saved["title"] = json!("Intro call");
    let (status, _) = save_type(&app, &host, &saved).await;
    assert_eq!(status, StatusCode::OK, "other changes still save");
    saved["ask_investment_comfort"] = json!(true);
    let (status, _) = save_type(&app, &host, &saved).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    saved["ask_investment_comfort"] = json!(false);
    saved["ask_urgency_level"] = json!(false);
    let (status, _) = save_type(&app, &host, &saved).await;
    assert_eq!(status, StatusCode::OK, "turning one off is fine");
}

/// Gives the host Business: everything Pro has, and the Business tools.
async fn give_business(pool: &PgPool, host: &Host) {
    give_plan(pool, host, "business").await;
}

/// Gives the host `plan` for free, as an admin would.
async fn give_plan(pool: &PgPool, host: &Host, plan: &str) {
    sqlx::query_scalar!(
        "UPDATE subscriptions SET plan = $2, status = 'comped' WHERE user_unid = $1 RETURNING 1",
        host.unid,
        plan,
    )
    .fetch_one(pool)
    .await
    .expect("the host has a plan row");
}

/// A second kind of call at `/book/<slug>/<type_slug>`.
async fn new_type(app: &Router, host: &Host, type_slug: &str, minutes: Option<i32>) -> StatusCode {
    let mut kind = first_type(app, host).await;
    kind["slug"] = json!(type_slug);
    kind["title"] = json!(format!("The {type_slug}"));
    kind["duration_minutes"] = json!(minutes);
    send(
        app,
        Method::POST,
        "/api/host/event-types",
        Some(&host.cookie),
        Some(kind),
    )
    .await
    .0
}

/// The open times of one kind of call, as start and length.
async fn type_slots(app: &Router, slug: &str, type_slug: &str) -> Vec<(DateTime<Utc>, i64)> {
    let (status, slots) = get(
        app,
        &format!("/api/hosts/{slug}/types/{type_slug}/slots"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{type_slug}");
    slots
        .as_array()
        .expect("slots are a list")
        .iter()
        .map(|slot| {
            let start = serde_json::from_value(slot["start"].clone()).expect("slot has a start");
            (start, slot["minutes"].as_i64().expect("slot has a length"))
        })
        .collect()
}

#[sqlx::test(migrations = "../migrations")]
async fn a_type_with_its_own_length_steps_through_the_same_hours(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_in_tbilisi(&app, 0, &[("09:00:00", "12:00:00", 60)]).await;
    give_business(&pool, &host).await;
    assert_eq!(
        new_type(&app, &host, "quick", Some(30)).await,
        StatusCode::CREATED
    );
    let day = |slots: &[(DateTime<Utc>, i64)]| -> Vec<(DateTime<Utc>, i64)> {
        slots
            .iter()
            .filter(|(start, _)| start.date_naive() == tbilisi(3, 9, 0).date_naive())
            .copied()
            .collect()
    };

    let hourly = day(&type_slots(&app, &slug, "call").await);
    let quick = day(&type_slots(&app, &slug, "quick").await);
    assert_eq!(
        hourly.iter().map(|slot| slot.1).collect::<Vec<_>>(),
        [60, 60, 60]
    );
    assert_eq!(quick.len(), 6, "every half hour from 09:00 to 11:30");
    assert!(quick.iter().all(|slot| slot.1 == 30));

    // A 30-minute call at 10:00 closes the hour-long 10:00 call too.
    let path = format!("/api/hosts/{slug}/types/quick/bookings");
    let (status, booked) = send(
        &app,
        Method::POST,
        &path,
        None,
        Some(booking(tbilisi(3, 10, 0), None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(booked["minutes"], 30);
    let starts: Vec<DateTime<Utc>> = day(&type_slots(&app, &slug, "call").await)
        .into_iter()
        .map(|slot| slot.0)
        .collect();
    assert_eq!(starts, [tbilisi(3, 9, 0), tbilisi(3, 11, 0)]);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_plan_decides_how_many_types_the_link_offers(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    end_trial(&pool, &host).await;
    assert_eq!(
        new_type(&app, &host, "deep-dive", Some(60)).await,
        StatusCode::FORBIDDEN,
        "Free books one kind of call"
    );

    give_plan(&pool, &host, "pro").await;
    assert_eq!(
        new_type(&app, &host, "deep-dive", Some(60)).await,
        StatusCode::CREATED,
        "Pro books several"
    );
    assert_eq!(
        new_type(&app, &host, "deep-dive", None).await,
        StatusCode::CONFLICT
    );
    let (status, page) = get(&app, &format!("/api/hosts/{slug}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["host_name"], "Nino Tbilisi");
    let listed: Vec<&str> = page["types"]
        .as_array()
        .expect("a list of types")
        .iter()
        .filter_map(|kind| kind["slug"].as_str())
        .collect();
    assert_eq!(listed, ["call", "deep-dive"]);

    // Back on Free: the newer type stays saved but is not offered.
    end_trial(&pool, &host).await;
    sqlx::query_scalar!(
        "DELETE FROM subscriptions WHERE user_unid = $1 RETURNING 1",
        host.unid
    )
    .fetch_one(&pool)
    .await
    .expect("the plan row is gone");
    let (_, page) = get(&app, &format!("/api/hosts/{slug}"), None).await;
    assert_eq!(page["types"].as_array().map(Vec::len), Some(1));
    let (status, deep) = get(&app, &format!("/api/hosts/{slug}/types/deep-dive"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(deep["accepting_bookings"], false);
    let (status, _) = get(
        &app,
        &format!("/api/hosts/{slug}/types/deep-dive/slots"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, types) = get(&app, "/api/host/event-types", Some(&host.cookie)).await;
    let bookable: Vec<bool> = types
        .as_array()
        .expect("a list of types")
        .iter()
        .map(|kind| kind["bookable"] == true)
        .collect();
    assert_eq!(bookable, [true, false]);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_call_outlives_its_type_and_the_last_type_stays(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    give_business(&pool, &host).await;
    assert_eq!(
        new_type(&app, &host, "intro", None).await,
        StatusCode::CREATED
    );
    let (_, types) = get(&app, "/api/host/event-types", Some(&host.cookie)).await;
    let intro = types[1]["unid"].as_str().expect("the new type").to_owned();
    let start = type_slots(&app, &slug, "intro").await[0].0;
    let path = format!("/api/hosts/{slug}/types/intro/bookings");
    let (status, booked) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
    assert_eq!(status, StatusCode::CREATED);
    let token = booked["manage_token"]
        .as_str()
        .expect("a manage link")
        .to_owned();
    let (_, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(call["event_slug"], "intro");

    let delete = |unid: String| {
        let app = app.clone();
        let cookie = host.cookie.clone();
        async move {
            send(
                &app,
                Method::DELETE,
                &format!("/api/host/event-types/{unid}"),
                Some(&cookie),
                None,
            )
            .await
            .0
        }
    };
    assert_eq!(delete(intro).await, StatusCode::NO_CONTENT);
    let (status, call) = get(&app, &format!("/api/manage/{token}"), None).await;
    assert_eq!(status, StatusCode::OK, "the call is still there");
    assert_eq!(
        call["event_slug"], "call",
        "read with the host's first type"
    );

    let first = first_type(&app, &host).await["unid"]
        .as_str()
        .expect("an id")
        .to_owned();
    assert_eq!(delete(first).await, StatusCode::CONFLICT);
}

/// The Calendly account the fake answers `FAKE_TOKEN` with: two calls, two
/// weekly stretches and a day off next week, in Berlin.
fn calendly_account() -> CalendlyAccount {
    let kind = |name: &str, slug: &str, minutes: i32| ApiEventType {
        name: Some(name.to_owned()),
        slug: Some(slug.to_owned()),
        duration: minutes,
        description_plain: None,
        active: true,
        kind: "solo".to_owned(),
        pooling_type: None,
    };
    let weekly = |wday: &str, from: &str, to: &str| ApiRule {
        kind: "wday".to_owned(),
        wday: Some(wday.to_owned()),
        date: None,
        intervals: vec![ApiInterval {
            from: from.to_owned(),
            to: to.to_owned(),
        }],
    };
    CalendlyAccount {
        user: ApiUser {
            uri: "https://api.calendly.com/users/ME".to_owned(),
            timezone: Some("Europe/Berlin".to_owned()),
        },
        event_types: vec![
            kind("Intro call", "intro", 30),
            kind("Deep dive", "deep-dive", 60),
        ],
        schedule: Some(ApiSchedule {
            default: true,
            timezone: "Europe/Berlin".to_owned(),
            rules: vec![
                weekly("monday", "09:00", "12:00"),
                weekly("wednesday", "14:00", "17:00"),
                ApiRule {
                    kind: "date".to_owned(),
                    wday: None,
                    date: Some((Utc::now() + Duration::days(7)).date_naive()),
                    intervals: vec![],
                },
            ],
        }),
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_calendly_setup_comes_in_and_the_first_link_keeps_working(pool: PgPool) {
    let app = app(&pool);
    let (host, _) = host_with_hours(&app, "nino@example.com").await;
    let preview_path = "/api/host/import/calendly/preview";

    let (status, _) = send(
        &app,
        Method::POST,
        preview_path,
        Some(&host.cookie),
        Some(json!({ "token": "wrong" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a token Calendly refuses");

    let (status, preview) = send(
        &app,
        Method::POST,
        preview_path,
        Some(&host.cookie),
        Some(json!({ "token": FAKE_TOKEN })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["timezone"], "Europe/Berlin");
    assert_eq!(preview["event_types"].as_array().map(Vec::len), Some(2));
    assert_eq!(preview["weekly"].as_array().map(Vec::len), Some(2));
    assert_eq!(preview["dates"].as_array().map(Vec::len), Some(1));

    let import = json!({
        "timezone": preview["timezone"],
        "event_types": preview["event_types"],
        "weekly": preview["weekly"],
        "dates": preview["dates"],
    });
    let (status, saved) = send(
        &app,
        Method::POST,
        "/api/host/import/calendly",
        Some(&host.cookie),
        Some(import.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["event_types"], 2, "the Pro trial brings both calls");
    assert_eq!(saved["event_types_left_out"], 0);

    let (_, settings) = get(&app, "/api/host/settings", Some(&host.cookie)).await;
    assert_eq!(settings["timezone"], "Europe/Berlin");
    let (_, hours) = get(&app, "/api/availability", Some(&host.cookie)).await;
    let hours: Vec<(i64, &str, &str, i64)> = hours
        .as_array()
        .expect("a list of hours")
        .iter()
        .map(|rule| {
            (
                rule["weekday"].as_i64().unwrap_or_default(),
                rule["start_time"].as_str().unwrap_or_default(),
                rule["end_time"].as_str().unwrap_or_default(),
                rule["slot_minutes"].as_i64().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        hours,
        [
            (0, "09:00:00", "12:00:00", 30),
            (2, "14:00:00", "17:00:00", 30)
        ],
        "the old hours are replaced, cut into the first call's length"
    );
    let (_, types) = get(&app, "/api/host/event-types", Some(&host.cookie)).await;
    let types: Vec<(&str, &str)> = types
        .as_array()
        .expect("a list of types")
        .iter()
        .map(|kind| {
            (
                kind["slug"].as_str().unwrap_or_default(),
                kind["title"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        types,
        [("call", "Intro call"), ("deep-dive", "Deep dive")],
        "the first call keeps its link"
    );

    // On Free, the second kind of call stays behind.
    let (other, _) = host_with_hours(&app, "ana@example.com").await;
    end_trial(&pool, &other).await;
    let (status, saved) = send(
        &app,
        Method::POST,
        "/api/host/import/calendly",
        Some(&other.cookie),
        Some(import),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["event_types"], 1);
    assert_eq!(saved["event_types_left_out"], 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn the_business_tools_are_closed_to_pro(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let start = *slots(&app, &slug).await.first().expect("open times");
    let (unid, _) = book(&app, &slug, start).await;
    for path in ["/api/host/leads/unfinished", "/api/host/insights"] {
        let (status, _) = get(&app, path, Some(&host.cookie)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path} on the Pro trial");
    }
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/bookings/{unid}/no-show"),
        Some(&host.cookie),
        Some(json!({ "no_show": true })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(
        !waiting_emails(&pool, unid)
            .await
            .iter()
            .any(|line| line.starts_with("host_unconfirmed")),
        "Pro hosts get no unconfirmed heads-up"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_business_host_sends_their_times_once_to_someone_who_left(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    give_business(&pool, &host).await;
    let (_, page) = get(&app, &format!("/api/hosts/{slug}/types/call"), None).await;
    assert_eq!(
        page["follow_up_notice"], true,
        "the form says it may happen"
    );

    let lead_unid = Uuid::new_v4();
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/types/call/leads/{lead_unid}"),
        None,
        Some(json!({ "first_name": "Jo", "email": "jo@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let unfinished = || async {
        let (status, list) = get(&app, "/api/host/leads/unfinished", Some(&host.cookie)).await;
        assert_eq!(status, StatusCode::OK);
        list.as_array().map_or(0, Vec::len)
    };
    assert_eq!(unfinished().await, 0, "someone typing now has not left");

    sqlx::query_scalar!(
        "UPDATE leads SET updated_at = now() - interval '2 hours' WHERE unid = $1 RETURNING 1",
        lead_unid,
    )
    .fetch_one(&pool)
    .await
    .expect("the lead is older");
    assert_eq!(unfinished().await, 1);

    let path = format!("/api/host/leads/{lead_unid}/follow-up");
    let (status, _) = send(&app, Method::POST, &path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(account_notices(&pool, "lead_follow_up").await, 1);
    let (status, _) = send(&app, Method::POST, &path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "only once");
    assert_eq!(unfinished().await, 0, "followed up, so off the list");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_business_host_knows_who_comes_and_who_did_not(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    give_business(&pool, &host).await;
    let start = *slots(&app, &slug).await.last().expect("open times");
    let (unid, token) = book(&app, &slug, start).await;
    assert!(
        waiting_emails(&pool, unid)
            .await
            .contains(&"host_unconfirmed -> nino@example.com".to_owned()),
        "the host hears 2 hours ahead unless the invitee confirms"
    );

    let (status, call) = send(
        &app,
        Method::POST,
        &format!("/api/manage/{token}/attend"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(call["attendance_confirmed"], true);

    let no_show = |marked: bool| {
        let app = app.clone();
        let cookie = host.cookie.clone();
        async move {
            send(
                &app,
                Method::POST,
                &format!("/api/bookings/{unid}/no-show"),
                Some(&cookie),
                Some(json!({ "no_show": marked })),
            )
            .await
        }
    };
    assert_eq!(no_show(true).await.0, StatusCode::CONFLICT, "not over yet");

    // The call took place yesterday.
    sqlx::query_scalar!(
        "UPDATE bookings SET slot_start = now() - interval '1 day', slot_end = now() - interval '23 hours'
         WHERE unid = $1 RETURNING 1",
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("the call is in the past");
    let (status, marked) = no_show(true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(marked["no_show_at"].is_string());

    let (status, week) = get(&app, "/api/host/insights", Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(week["booked"], 1);
    assert_eq!(week["held"], 1);
    assert_eq!(week["confirmed"], 1);
    assert_eq!(week["no_shows"], 1);

    let (_, unmarked) = no_show(false).await;
    assert!(unmarked["no_show_at"].is_null(), "a mistake can be undone");
}

#[sqlx::test(migrations = "../migrations")]
async fn an_accent_colour_needs_pro_and_stays_saved_without_it(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let mut current = settings(&app, &host).await;
    assert_eq!(current["accent_color"], "#111111");

    current["accent_color"] = json!("red");
    let (status, _) = save_settings(&app, &host, &current).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    current["accent_color"] = json!("#7C3AED");
    let (status, saved) = save_settings(&app, &host, &current).await;
    assert_eq!(status, StatusCode::OK, "the trial includes it");
    assert_eq!(saved["accent_color"], "#7c3aed");
    let (_, page) = get(&app, &format!("/api/hosts/{slug}/types/call"), None).await;
    assert_eq!(page["accent_color"], "#7c3aed");
    let (_, link) = get(&app, &format!("/api/hosts/{slug}"), None).await;
    assert_eq!(link["accent_color"], "#7c3aed");

    end_trial(&pool, &host).await;
    let (_, page) = get(&app, &format!("/api/hosts/{slug}/types/call"), None).await;
    assert_eq!(page["accent_color"], Value::Null, "not shown on Free");
    let mut kept = settings(&app, &host).await;
    assert_eq!(kept["accent_color"], "#7c3aed", "still saved");

    kept["is_active"] = json!(false);
    let (status, _) = save_settings(&app, &host, &kept).await;
    assert_eq!(status, StatusCode::OK, "other changes still save");
    kept["accent_color"] = json!("#0f766e");
    let (status, _) = save_settings(&app, &host, &kept).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "a new colour needs Pro");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_trial_host_sees_what_free_would_take_away(pool: PgPool) {
    let app = app(&pool);
    let (host, _) = host_with_hours(&app, "nino@example.com").await;
    let mut kind = first_type(&app, &host).await;
    kind["ask_urgency_level"] = json!(true);
    let (status, _) = save_type(&app, &host, &kind).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        new_type(&app, &host, "deep-dive", Some(60)).await,
        StatusCode::CREATED
    );

    let (status, me) = get(&app, "/api/billing/me", Some(&host.cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let losses: Vec<&str> = me["trial_losses"]
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(losses[0].starts_with("2 kinds of call"), "{losses:?}");
    assert!(losses.contains(&"Your own form questions stop being asked."));

    end_trial(&pool, &host).await;
    let (_, me) = get(&app, "/api/billing/me", Some(&host.cookie)).await;
    assert_eq!(
        me["trial_losses"],
        json!([]),
        "nothing to warn about once on Free"
    );
}
