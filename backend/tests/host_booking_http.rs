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
    domain::{auth::service::make_token, emails, leads},
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

/// Books `start` as Ada and returns the call's id and its manage link token.
async fn book(app: &Router, slug: &str, start: DateTime<Utc>) -> (Uuid, String) {
    let (status, booked) = send(
        app,
        Method::POST,
        &format!("/api/hosts/{slug}/bookings"),
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
async fn a_switched_off_page_is_readable_and_a_banned_host_is_not(pool: PgPool) {
    let app = app(&pool);
    let page_path = format!("/api/hosts/{SEED_SLUG}");
    let (_, page) = get(&app, &page_path, None).await;
    assert_eq!(page["accepting_bookings"], true);

    sqlx::query_scalar!(
        "UPDATE widget_settings SET is_active = false WHERE public_slug = $1 RETURNING 1",
        SEED_SLUG,
    )
    .fetch_one(&pool)
    .await
    .expect("seed host exists");
    let (status, page) = get(&app, &page_path, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["accepting_bookings"], false);
    assert_eq!(page["host_name"], "Cadence Host");
    let (status, _) = get(&app, &format!("{page_path}/slots"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    sqlx::query_scalar!(
        "UPDATE users SET status = 'Banned' FROM widget_settings w
         WHERE w.host_unid = users.unid AND w.public_slug = $1 RETURNING 1",
        SEED_SLUG,
    )
    .fetch_one(&pool)
    .await
    .expect("seed host exists");
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
async fn slots_stay_inside_the_hosts_window_and_notice(pool: PgPool) {
    let app = app(&pool);
    let (host, slug) = host_with_hours(&app, "nino@example.com").await;
    let mut rules = settings(&app, &host).await;
    assert_eq!(rules["minimum_notice_hours"], 4);
    assert_eq!(rules["booking_window_days"], 7);
    assert_eq!(rules["is_active"], true);

    for (field, bad) in [
        ("minimum_notice_hours", -1),
        ("minimum_notice_hours", 169),
        ("booking_window_days", 0),
        ("booking_window_days", 91),
    ] {
        let mut changed = rules.clone();
        changed[field] = json!(bad);
        let (status, _) = save_settings(&app, &host, &changed).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field} = {bad}");
    }

    rules["booking_window_days"] = json!(2);
    let (status, saved) = save_settings(&app, &host, &rules).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved, rules);
    let near = slots(&app, &slug).await;
    // One call a day, from 4 hours out to 2 days out.
    assert!((1..=2).contains(&near.len()), "{} slots", near.len());
    for start in &near {
        assert!(*start < Utc::now() + chrono::Duration::days(2), "{start}");
    }
    let (_, page) = get(&app, &format!("/api/hosts/{slug}"), None).await;
    assert_eq!(page["booking_window_days"], 2);
    assert!(page.get("minimum_notice_hours").is_none());

    rules["booking_window_days"] = json!(7);
    rules["minimum_notice_hours"] = json!(72);
    let (status, _) = save_settings(&app, &host, &rules).await;
    assert_eq!(status, StatusCode::OK);
    let far = slots(&app, &slug).await;
    assert!((3..=4).contains(&far.len()), "{} slots", far.len());
    for start in &far {
        assert!(*start > Utc::now() + chrono::Duration::hours(71), "{start}");
        assert!(!near.contains(start), "{start}");
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

    let (status, page) = get(&app, &format!("/api/hosts/{slug}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["accepting_bookings"], false);
    assert_eq!(page["host_name"], "Nino Tbilisi");
    let (status, _) = get(&app, &format!("/api/hosts/{slug}/slots"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let path = format!("/api/hosts/{slug}/bookings");
    let (status, _) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/leads/{}", Uuid::new_v4()),
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
    let mut asking = settings(&app, &host).await;
    asking["ask_urgency_level"] = json!(true);
    let (status, _) = save_settings(&app, &host, &asking).await;
    assert_eq!(status, StatusCode::OK);

    let lead_unid = Uuid::new_v4();
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/hosts/{slug}/leads/{lead_unid}"),
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
    let path = format!("/api/hosts/{slug}/bookings");
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
    let book_path = format!("/api/hosts/{slug}/bookings");
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
            "can_change": true,
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
    assert_eq!(
        waiting_emails(&pool, unid).await,
        [
            "booking_confirmation -> ada@example.com",
            "host_new_booking -> nino@example.com",
            "reminder_24h -> ada@example.com",
            "reminder_2h -> ada@example.com",
            "reminder_30min -> ada@example.com",
        ]
    );
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
        (new + chrono::Duration::minutes(7), "never offered"),
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

    // The old emails are replaced: both sides are told and the reminders
    // follow the new time.
    assert_eq!(
        waiting_emails(&pool, unid).await,
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
        unid,
    )
    .fetch_one(&pool)
    .await
    .expect("queue is readable");
    assert_eq!(reminder, new - chrono::Duration::minutes(30));
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
    let seeded = hosts
        .iter()
        .find(|host| host["public_slug"] == json!(SEED_SLUG))
        .expect("the seeded host is listed");
    assert_eq!(seeded["has_hours"], json!(true));

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

    // The development seed has due emails of its own.
    let due = emails::db::pending(&pool).await.expect("queue is readable");
    assert!(due.iter().all(|row| row.email != "failed@example.com"));
    let last_try = due
        .iter()
        .find(|row| row.email == "last-try@example.com")
        .expect("an email with one attempt left is still due");

    emails::db::mark_sent(&pool, last_try.id)
        .await
        .expect("row is marked");
    let clean = sqlx::query_scalar!(
        r#"SELECT sent_at IS NOT NULL AND last_error IS NULL AS "clean!"
           FROM email_queue WHERE id = $1"#,
        last_try.id,
    )
    .fetch_one(&pool)
    .await
    .expect("row is readable");
    assert!(clean, "sent, and the old error is gone");
    let due = emails::db::pending(&pool).await.expect("queue is readable");
    assert!(due.iter().all(|row| row.email != "last-try@example.com"));
}

/// The open times of a host with half-hour calls, as "HH:MM" on the host's
/// clock in Tbilisi (UTC+4) for the day `day` falls on.
async fn half_hours_on(app: &Router, slug: &str, day: DateTime<Utc>) -> Vec<String> {
    let (status, slots) = get(app, &format!("/api/hosts/{slug}/slots"), None).await;
    assert_eq!(status, StatusCode::OK);
    slots
        .as_array()
        .expect("slots are a list")
        .iter()
        .map(|slot| {
            serde_json::from_value::<DateTime<Utc>>(slot["start"].clone())
                .expect("slot has a start")
        })
        .filter(|start| start.date_naive() == day.date_naive())
        .map(|start| {
            (start + chrono::Duration::hours(4))
                .format("%H:%M")
                .to_string()
        })
        .collect()
}

#[sqlx::test(migrations = "../migrations")]
async fn a_buffer_keeps_time_free_around_every_booked_call(pool: PgPool) {
    let app = app(&pool);
    let host = register(&app, "Nino Tbilisi", "nino@example.com").await;
    let mut host_settings = settings(&app, &host).await;
    assert_eq!(host_settings["buffer_minutes"], 0, "no buffer until chosen");
    host_settings["timezone"] = json!("Asia/Tbilisi");
    let (status, _) = save_settings(&app, &host, &host_settings).await;
    assert_eq!(status, StatusCode::OK);
    for weekday in 0..7 {
        let (status, _) = send(
            &app,
            Method::POST,
            "/api/availability",
            Some(&host.cookie),
            Some(json!({
                "weekday": weekday,
                "start_time": "09:00:00",
                "end_time": "12:00:00",
                "slot_minutes": 30,
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let slug = slug(&host_settings).to_owned();
    let path = format!("/api/hosts/{slug}/bookings");
    let (_, page) = get(&app, &format!("/api/hosts/{slug}"), None).await;
    assert!(page.get("buffer_minutes").is_none());

    // 10:00 in Tbilisi, three days out: a whole day inside the window.
    let ten = (Utc::now() + chrono::Duration::days(3))
        .date_naive()
        .and_hms_opt(6, 0, 0)
        .expect("06:00 exists")
        .and_utc();
    let half_hour = chrono::Duration::minutes(30);
    let all = ["09:00", "09:30", "10:00", "10:30", "11:00", "11:30"];
    assert_eq!(half_hours_on(&app, &slug, ten).await, all);

    let (_, token) = book(&app, &slug, ten).await;
    assert_eq!(
        half_hours_on(&app, &slug, ten).await,
        ["09:00", "09:30", "10:30", "11:00", "11:30"],
        "without a buffer only the booked time closes"
    );

    for bad in [-1, 121] {
        host_settings["buffer_minutes"] = json!(bad);
        let (status, _) = save_settings(&app, &host, &host_settings).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
    host_settings["buffer_minutes"] = json!(15);
    let (status, saved) = save_settings(&app, &host, &host_settings).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["buffer_minutes"], 15);
    assert_eq!(
        half_hours_on(&app, &slug, ten).await,
        ["09:00", "11:00", "11:30"],
        "the calls right before and after would leave no gap"
    );
    for start in [ten - half_hour, ten + half_hour] {
        let (status, _) = send(&app, Method::POST, &path, None, Some(booking(start, None))).await;
        assert_eq!(status, StatusCode::CONFLICT, "{start}");
    }

    // The call's own time does not stop it from moving half an hour.
    let (status, moved) = send(
        &app,
        Method::POST,
        &format!("/api/manage/{token}/reschedule"),
        None,
        Some(json!({ "slot_start": ten + half_hour })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved["slot_start"], json!(ten + half_hour));
    assert_eq!(
        half_hours_on(&app, &slug, ten).await,
        ["09:00", "09:30", "11:30"]
    );

    // A second call still has to keep the gap from the first.
    let (other, other_token) = book(&app, &slug, ten - half_hour).await;
    assert_eq!(half_hours_on(&app, &slug, ten).await, ["11:30"]);
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/manage/{other_token}/reschedule"),
        None,
        Some(json!({ "slot_start": ten })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "too close to the other call");
    let (_, mine) = get(&app, &format!("/api/bookings/{other}"), Some(&host.cookie)).await;
    assert_eq!(mine["booking"]["slot_start"], json!(ten - half_hour));
}
