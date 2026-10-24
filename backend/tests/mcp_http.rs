//! The MCP server over HTTP: access tokens, who gets in, and what an agent's
//! tool calls do to a host's calls, hours and emails.

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
        google_calendar::calendar::{HostCalendar, InMemoryCalendar},
    },
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const TOKENS: &str = "/api/host/access-tokens";
const MCP: &str = "/api/mcp";

struct Host {
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

/// One JSON-RPC message to the MCP server, as an agent sends it.
async fn rpc(
    app: &Router,
    token: Option<&str>,
    method: &str,
    params: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri(MCP)
        // Every HTTP/1.1 client sends one, and the MCP transport needs it.
        .header(header::HOST, "bookcadence.app")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let request = request
        .body(Body::from(body.to_string()))
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

/// A tool's answer: whether it was a tool error, and its JSON or message.
async fn call(app: &Router, token: &str, tool: &str, arguments: Value) -> (bool, Value) {
    let (status, reply) = rpc(
        app,
        Some(token),
        "tools/call",
        json!({ "name": tool, "arguments": arguments }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reply}");
    let result = &reply["result"];
    assert!(result.is_object(), "{tool} answered {reply}");
    let text = result["content"][0]["text"]
        .as_str()
        .expect("a text answer")
        .to_owned();
    let is_error = result["isError"].as_bool().unwrap_or(false);
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (is_error, value)
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
        cookie: format!("session={token}"),
    }
}

/// A host with hour-long calls from 09:00 to 11:00 every day, and an agent
/// token of theirs.
async fn host_with_token(app: &Router, email: &str) -> (Host, String) {
    let host = register(app, email).await;
    let (status, _) = send(
        app,
        Method::GET,
        "/api/host/settings",
        Some(&host.cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for weekday in 0..7 {
        let hours = json!({
            "weekday": weekday, "start_time": "09:00:00", "end_time": "11:00:00", "slot_minutes": 60,
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
    let (status, created) = send(
        app,
        Method::POST,
        TOKENS,
        Some(&host.cookie),
        Some(json!({ "name": "Claude Code" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let token = created["token"].as_str().expect("a token").to_owned();
    (host, token)
}

async fn event_type(app: &Router, token: &str) -> String {
    let (is_error, types) = call(app, token, "list_event_types", json!({})).await;
    assert!(!is_error);
    types[0]["unid"]
        .as_str()
        .expect("a kind of call")
        .to_owned()
}

async fn open_slots(app: &Router, token: &str, event_type: &str) -> Vec<Value> {
    let (is_error, slots) = call(
        app,
        token,
        "find_open_slots",
        json!({ "event_type_unid": event_type }),
    )
    .await;
    assert!(!is_error, "{slots}");
    slots.as_array().expect("a list").clone()
}

async fn book(app: &Router, token: &str, event_type: &str, slot_start: &Value) -> Value {
    let (is_error, booking) = call(
        app,
        token,
        "create_booking",
        json!({
            "event_type_unid": event_type, "slot_start": slot_start,
            "invitee_name": "Ada Lovelace", "invitee_email": "ada@example.com",
        }),
    )
    .await;
    assert!(!is_error, "{booking}");
    booking
}

async fn waiting_templates(pool: &PgPool, booking_unid: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT template FROM email_queue WHERE booking_unid = $1 AND sent_at IS NULL ORDER BY template",
        booking_unid,
    )
    .fetch_all(pool)
    .await
    .expect("queue is readable")
}

fn unid_of(value: &Value) -> Uuid {
    serde_json::from_value(value["unid"].clone()).expect("an id")
}

#[sqlx::test(migrations = "../migrations")]
async fn tokens_are_shown_once_and_can_be_revoked(pool: PgPool) {
    let app = app(&pool);
    let (host, token) = host_with_token(&app, "tokens@example.com").await;
    assert!(token.starts_with("cad_"));

    let (status, list) = send(&app, Method::GET, TOKENS, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let listed = &list[0];
    assert_eq!(listed["name"], "Claude Code");
    assert_eq!(listed["last_four"], token[token.len() - 4..]);
    assert!(listed.get("token").is_none());
    assert!(!list.to_string().contains(&token));

    let (status, _) = rpc(&app, Some(&token), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let path = format!("{TOKENS}/{}", listed["unid"].as_str().expect("an id"));
    let (status, _) = send(&app, Method::DELETE, &path, Some(&host.cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = rpc(&app, Some(&token), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../migrations")]
async fn the_server_needs_a_live_token(pool: PgPool) {
    let app = app(&pool);
    let (host, _) = host_with_token(&app, "door@example.com").await;
    let session = host.cookie.trim_start_matches("session=").to_owned();

    for token in [None, Some("cad_not-a-token"), Some(session.as_str())] {
        let (status, _) = rpc(&app, token, "tools/list", json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{token:?}");
    }
    // A session cookie is not an agent token either.
    let (status, _) = send(&app, Method::POST, MCP, Some(&host.cookie), Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../migrations")]
async fn an_agent_sees_the_tools(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "agent@example.com").await;

    let (status, init) = rpc(
        &app,
        Some(&token),
        "initialize",
        json!({
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "test", "version": "1" },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(init["result"]["serverInfo"]["name"], "cadence");

    let (status, list) = rpc(&app, Some(&token), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for expected in [
        "list_bookings",
        "get_booking",
        "cancel_booking",
        "reschedule_booking",
        "find_open_slots",
        "create_booking",
        "list_event_types",
        "set_weekly_hours",
        "set_date_override",
        "get_profile",
    ] {
        assert!(names.contains(&expected), "{expected} in {names:?}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn an_agent_books_moves_and_cancels_a_call(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "calls@example.com").await;
    let kind = event_type(&app, &token).await;
    let slots = open_slots(&app, &token, &kind).await;
    assert!(slots.len() >= 2);

    let booking = book(&app, &token, &kind, &slots[0]["start"]).await;
    let booking_unid = unid_of(&booking);
    assert!(booking.get("manage_token").is_none());
    assert!(!waiting_templates(&pool, booking_unid).await.is_empty());

    let (is_error, listed) = call(&app, &token, "list_bookings", json!({})).await;
    assert!(!is_error);
    assert_eq!(listed.as_array().expect("a list").len(), 1);

    // The time it already has, or one already taken, is a tool error the
    // agent can read.
    let (is_error, message) = call(
        &app,
        &token,
        "reschedule_booking",
        json!({ "booking_unid": booking_unid, "slot_start": slots[0]["start"] }),
    )
    .await;
    assert!(is_error, "{message}");

    let (is_error, moved) = call(
        &app,
        &token,
        "reschedule_booking",
        json!({ "booking_unid": booking_unid, "slot_start": slots[1]["start"] }),
    )
    .await;
    assert!(!is_error, "{moved}");
    assert_eq!(moved["slot_start"], slots[1]["start"]);
    assert!(
        waiting_templates(&pool, booking_unid)
            .await
            .iter()
            .any(|template| template.contains("rescheduled")),
    );

    let (is_error, cancelled) = call(
        &app,
        &token,
        "cancel_booking",
        json!({ "booking_unid": booking_unid }),
    )
    .await;
    assert!(!is_error, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");

    let (is_error, listed) = call(
        &app,
        &token,
        "list_bookings",
        json!({ "status": "confirmed" }),
    )
    .await;
    assert!(!is_error);
    assert!(listed.as_array().expect("a list").is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn an_agent_only_reaches_its_own_host(pool: PgPool) {
    let app = app(&pool);
    let (_, own) = host_with_token(&app, "own@example.com").await;
    let (_, other) = host_with_token(&app, "other@example.com").await;
    let kind = event_type(&app, &other).await;
    let slots = open_slots(&app, &other, &kind).await;
    let booking = book(&app, &other, &kind, &slots[0]["start"]).await;

    let (_, listed) = call(&app, &own, "list_bookings", json!({})).await;
    assert!(listed.as_array().expect("a list").is_empty());
    for tool in ["get_booking", "cancel_booking"] {
        let (is_error, _) =
            call(&app, &own, tool, json!({ "booking_unid": booking["unid"] })).await;
        assert!(is_error, "{tool}");
    }
    let (is_error, _) = call(
        &app,
        &own,
        "find_open_slots",
        json!({ "event_type_unid": kind }),
    )
    .await;
    assert!(is_error);
}

#[sqlx::test(migrations = "../migrations")]
async fn new_hours_change_the_open_slots(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "hours@example.com").await;
    let kind = event_type(&app, &token).await;
    let before = open_slots(&app, &token, &kind).await;

    let (is_error, saved) = call(
        &app,
        &token,
        "set_weekly_hours",
        json!({ "rules": [
            { "weekday": 0, "start_time": "13:00:00", "end_time": "14:00:00", "slot_minutes": 30 },
        ] }),
    )
    .await;
    assert!(!is_error, "{saved}");
    assert_eq!(saved.as_array().expect("rules").len(), 1);

    let after = open_slots(&app, &token, &kind).await;
    assert!(after.len() < before.len());
    assert!(after.iter().all(|slot| slot["minutes"] == 30));

    let (is_error, message) = call(
        &app,
        &token,
        "set_weekly_hours",
        json!({ "rules": [
            { "weekday": 9, "start_time": "13:00:00", "end_time": "14:00:00", "slot_minutes": 30 },
        ] }),
    )
    .await;
    assert!(is_error, "{message}");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_feature_past_the_plan_answers_with_an_upgrade_hint(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "pro@example.com").await;
    sqlx::query_scalar!(
        "UPDATE subscriptions SET plan = 'pro', status = 'active', trial_ends_at = NULL RETURNING 1",
    )
    .fetch_all(&pool)
    .await
    .expect("plans are writable");

    let (is_error, message) = call(&app, &token, "get_insights", json!({})).await;
    assert!(is_error);
    assert!(
        message.as_str().expect("a message").contains("billing"),
        "{message}"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_call_says_its_kind_and_the_hosts_clock(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "clock@example.com").await;
    let kind = event_type(&app, &token).await;
    let slots = open_slots(&app, &token, &kind).await;
    assert!(slots[0]["start_local"].is_string(), "{}", slots[0]);
    book(&app, &token, &kind, &slots[0]["start"]).await;
    book(&app, &token, &kind, &slots[1]["start"]).await;

    let (status, reply) = rpc(
        &app,
        Some(&token),
        "tools/call",
        json!({ "name": "list_bookings", "arguments": { "from": slots[1]["start"] } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let calls = reply["result"]["structuredContent"]["items"]
        .as_array()
        .expect("structured calls");
    assert_eq!(calls.len(), 1, "from leaves out the earlier call");
    let call = &calls[0];
    assert_eq!(call["slot_start"], slots[1]["start"]);
    assert_eq!(call["event_type_unid"], kind);
    assert!(call["event_type_title"].is_string());
    assert_eq!(call["slot_start_local"], slots[1]["start_local"]);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_kind_of_call_changes_only_what_is_sent(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "patch@example.com").await;
    let (_, types) = call(&app, &token, "list_event_types", json!({})).await;
    let before = types[0].clone();

    let (is_error, after) = call(
        &app,
        &token,
        "update_event_type",
        json!({ "event_type_unid": before["unid"], "title": "Intro call" }),
    )
    .await;
    assert!(!is_error, "{after}");
    assert_eq!(after["title"], "Intro call");
    for kept in [
        "slug",
        "description",
        "buffer_minutes",
        "questions",
        "is_active",
    ] {
        assert_eq!(after[kept], before[kept], "{kept}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn an_agent_sets_where_calls_happen(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "meet@example.com").await;

    let (is_error, message) = call(
        &app,
        &token,
        "set_conferencing",
        json!({ "app": "custom_link" }),
    )
    .await;
    assert!(is_error, "a custom link needs the link: {message}");

    let link = "https://zoom.us/j/123";
    let (is_error, settings) = call(
        &app,
        &token,
        "set_conferencing",
        json!({ "app": "custom_link", "meeting_link": link }),
    )
    .await;
    assert!(!is_error, "{settings}");
    assert_eq!(settings["conferencing_app"], "custom_link");
    assert_eq!(settings["meeting_link"], link);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_read_only_token_looks_but_does_not_change(pool: PgPool) {
    let app = app(&pool);
    let (host, full) = host_with_token(&app, "reader@example.com").await;
    let kind = event_type(&app, &full).await;
    let slots = open_slots(&app, &full, &kind).await;
    let booking = book(&app, &full, &kind, &slots[0]["start"]).await;

    let (status, created) = send(
        &app,
        Method::POST,
        TOKENS,
        Some(&host.cookie),
        Some(json!({ "name": "Summaries", "scope": "read" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["access_token"]["scope"], "read");
    let read = created["token"].as_str().expect("a token");

    let (is_error, listed) = call(&app, read, "list_bookings", json!({})).await;
    assert!(!is_error);
    assert_eq!(listed.as_array().expect("a list").len(), 1);

    let (is_error, message) = call(
        &app,
        read,
        "cancel_booking",
        json!({ "booking_unid": booking["unid"] }),
    )
    .await;
    assert!(is_error);
    assert!(
        message.as_str().expect("a message").contains("read-only"),
        "{message}"
    );
    let (_, after) = call(
        &app,
        read,
        "get_booking",
        json!({ "booking_unid": booking["unid"] }),
    )
    .await;
    assert_eq!(after["booking"]["status"], "confirmed");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_runaway_agent_is_slowed_down(pool: PgPool) {
    let app = app(&pool);
    let (_, token) = host_with_token(&app, "loop@example.com").await;
    for _ in 0..300 {
        let (status, _) = rpc(&app, Some(&token), "tools/list", json!({})).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, _) = rpc(&app, Some(&token), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}
