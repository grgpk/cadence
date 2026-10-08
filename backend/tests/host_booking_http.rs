//! Per-host booking links over HTTP: settings, isolation, question switches,
//! and slots that come from the host's own hours.

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
    domain::{auth::service::make_token, leads},
};
use chrono::{DateTime, Timelike, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// Created by the development seed.
const SEED_SLUG: &str = "cadence-host";
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
    // JWT_SECRET lives in the workspace .env, next to DATABASE_URL.
    dotenvy::dotenv().ok();
    build_app(AppState { pool: pool.clone() })
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
    let token = make_token(unid).expect("JWT_SECRET is set for tests");
    Host {
        unid,
        cookie: format!("session={token}"),
    }
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
    let (status, slots) = get(app, &format!("/api/hosts/{slug}/slots"), None).await;
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

#[sqlx::test(migrations = "../migrations")]
async fn public_page_shows_no_ids_and_asks_nothing_by_default(pool: PgPool) {
    let app = app(&pool);

    let (status, page) = get(&app, &format!("/api/hosts/{SEED_SLUG}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.get("host_unid").is_none());
    assert!(page.get("email").is_none());
    assert_eq!(page["host_name"], "Cadence Host");
    for flag in QUESTION_FLAGS {
        assert_eq!(page[flag], false, "{flag}");
    }

    let (status, _) = get(&app, "/api/hosts/CADENCE-HOST", None).await;
    assert_eq!(status, StatusCode::OK, "links are case-insensitive");
    let (status, _) = get(&app, "/api/hosts/nobody-here", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../migrations")]
async fn switched_off_page_looks_unknown(pool: PgPool) {
    let app = app(&pool);
    sqlx::query_scalar!(
        "UPDATE widget_settings SET is_active = false WHERE public_slug = $1 RETURNING 1",
        SEED_SLUG,
    )
    .fetch_one(&pool)
    .await
    .expect("seed host exists");

    for path in ["", "/slots"] {
        let (status, _) = get(&app, &format!("/api/hosts/{SEED_SLUG}{path}"), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
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

    let (status, page) = get(&app, "/api/hosts/ada-lovelace", None).await;
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
        ("title", json!("  ")),
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
    changed["title"] = json!("Talk to Ada");
    changed["ask_urgency_level"] = json!(true);
    let (status, saved) = save_settings(&app, &ada, &changed).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved, changed);

    let (_, page) = get(&app, "/api/hosts/ada", None).await;
    assert_eq!(page["title"], "Talk to Ada");
    assert_eq!(page["ask_urgency_level"], true);
    let (status, _) = get(&app, "/api/hosts/ada-lovelace", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the old link is gone");
    assert_eq!(settings(&app, &grace).await, grace_settings);
}

#[sqlx::test(migrations = "../migrations")]
async fn answers_are_kept_only_for_questions_the_host_asks(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "Ada Lovelace", "ada@example.com").await;
    let mut host_settings = settings(&app, &host).await;
    let lead_unid = Uuid::new_v4();
    let path = format!("/api/hosts/ada-lovelace/leads/{lead_unid}");
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

    host_settings["ask_investment_comfort"] = json!(true);
    let (status, _) = save_settings(&app, &host, &host_settings).await;
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
        &format!("/api/hosts/ada-lovelace/leads/{lead_unid}"),
        None,
        Some(json!({ "first_name": "Linus", "email": "linus@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for suffix in ["", "/submit"] {
        let (status, _) = send(
            &app,
            Method::POST,
            &format!("/api/hosts/grace-hopper/leads/{lead_unid}{suffix}"),
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
        &format!("/api/hosts/{}/slots", self::slug(&idle_settings)),
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
    let path = format!("/api/hosts/{slug}/bookings");
    let open = slots(&app, &slug).await;
    let start = open[0];

    let (status, _) = send(
        &app,
        Method::POST,
        &path,
        None,
        Some(booking(start + chrono::Duration::minutes(7), None)),
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

    let too_long = format!("https://example.com/{}", "a".repeat(500));
    for bad in ["http://zoom.us/j/1", "not a url", too_long.as_str()] {
        let mut changed = saved.clone();
        changed["meeting_link"] = json!(bad);
        let (status, _) = save_settings(&app, &host, &changed).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }

    let mut changed = saved.clone();
    changed["meeting_link"] = json!("  https://zoom.us/j/123456789 ");
    let (status, with_link) = save_settings(&app, &host, &changed).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(with_link["meeting_link"], "https://zoom.us/j/123456789");

    let (_, page) = get(&app, "/api/hosts/ada-lovelace", None).await;
    assert_eq!(page["has_meeting_link"], true);
    assert!(page.get("meeting_link").is_none());
    assert!(!page.to_string().contains("zoom.us"));

    changed["meeting_link"] = json!("");
    let (status, cleared) = save_settings(&app, &host, &changed).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cleared["meeting_link"], Value::Null);
    let (_, page) = get(&app, "/api/hosts/ada-lovelace", None).await;
    assert_eq!(page["has_meeting_link"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_booking_without_google_uses_the_hosts_meeting_link(pool: PgPool) {
    const LINK: &str = "https://zoom.us/j/123456789";
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let mut host_settings = settings(&app, &host).await;
    host_settings["meeting_link"] = json!(LINK);
    let (status, _) = save_settings(&app, &host, &host_settings).await;
    assert_eq!(status, StatusCode::OK);

    let start = slots(&app, &slug).await[0];
    let (status, booked) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/bookings"),
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
