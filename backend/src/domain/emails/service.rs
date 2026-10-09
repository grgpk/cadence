use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use time::{OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description};
use time_tz::{OffsetDateTimeExt, timezones};

use super::{
    db,
    models::{AdminEmail, EmailState},
};
use crate::error::AppResult;

/// Sent when a host calls a booking off. Carries no join link.
pub const BOOKING_CANCELLED_TEMPLATE: &str = "booking_cancelled";
const UTC: &str = "UTC";
/// The worker runs every minute, so anything this late is not being picked up.
const OVERDUE_AFTER_MINUTES: i64 = 5;

pub async fn list_recent(pool: &PgPool) -> AppResult<Vec<AdminEmail>> {
    let now = Utc::now();
    Ok(db::list_recent(pool)
        .await?
        .into_iter()
        .map(|row| AdminEmail {
            state: state_of(row.sent_at, row.attempts, row.send_at, now),
            id: row.id,
            booking_unid: row.booking_unid,
            email: row.email,
            template: row.template,
            send_at: row.send_at,
            sent_at: row.sent_at,
            attempts: row.attempts,
            last_error: row.last_error,
            created_at: row.created_at,
        })
        .collect())
}

fn state_of(
    sent_at: Option<DateTime<Utc>>,
    attempts: i32,
    send_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> EmailState {
    if sent_at.is_some() {
        EmailState::Sent
    } else if attempts > 0 {
        EmailState::Retrying
    } else if send_at < now - Duration::minutes(OVERDUE_AFTER_MINUTES) {
        EmailState::Overdue
    } else {
        EmailState::Scheduled
    }
}

pub fn render(template: &str, payload: Option<&Value>) -> Option<(String, String)> {
    let (subject, lead) = match template {
        "booking_confirmation" => (
            "Your Cadence call is confirmed",
            "Your call is confirmed for",
        ),
        "reminder_24h" => (
            "Reminder: your Cadence call is tomorrow",
            "Your Cadence call starts in 24 hours:",
        ),
        "reminder_2h" => (
            "Reminder: your Cadence call starts in 2 hours",
            "Your Cadence call starts in 2 hours:",
        ),
        "reminder_30min" => (
            "Reminder: your Cadence call starts in 30 minutes",
            "Your Cadence call starts in 30 minutes:",
        ),
        BOOKING_CANCELLED_TEMPLATE => (
            "Your Cadence call was cancelled",
            "Your host cancelled the call booked for",
        ),
        _ => return None,
    };
    let text = |key: &str| {
        payload
            .and_then(|value| value.get(key))
            .and_then(Value::as_str)
    };
    let name = text("name").unwrap_or("there");
    let when = local_time(text("slot_start"), text("timezone").unwrap_or(UTC));
    // Emails queued before the host's own link existed used `meet_link`.
    let join = text("meeting_url")
        .or_else(|| text("meet_link"))
        .filter(|_| template != BOOKING_CANCELLED_TEMPLATE)
        .map(|url| format!("\n\nJoin the call: {url}"))
        .unwrap_or_default();

    Some((
        subject.to_owned(),
        format!("Hi {name},\n\n{lead} {when}.{join}\n\nCadence"),
    ))
}

/// "Monday, 12 October 2026 at 09:00 (Europe/Paris)", in the invitee's own
/// timezone. An unknown timezone falls back to UTC and says so.
fn local_time(slot_start: Option<&str>, timezone: &str) -> String {
    let Some(instant) = slot_start.and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
    else {
        return "the time you booked".to_owned();
    };
    let (local, label) = timezones::get_by_name(timezone).map_or_else(
        || (instant.to_offset(time::UtcOffset::UTC), UTC),
        |tz| (instant.to_timezone(tz), timezone),
    );
    let format = format_description!(
        "[weekday], [day padding:none] [month repr:long] [year] at [hour]:[minute]"
    );
    local.format(format).map_or_else(
        |_| "the time you booked".to_owned(),
        |when| format!("{when} ({label})"),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn email_state_follows_the_queue_row() {
        let now = Utc::now();
        let late = now - Duration::minutes(OVERDUE_AFTER_MINUTES + 1);
        assert_eq!(state_of(Some(now), 3, late, now), EmailState::Sent);
        assert_eq!(state_of(None, 1, late, now), EmailState::Retrying);
        assert_eq!(state_of(None, 0, late, now), EmailState::Overdue);
        assert_eq!(state_of(None, 0, now, now), EmailState::Scheduled);
    }

    const TEMPLATES: [&str; 4] = [
        "booking_confirmation",
        "reminder_24h",
        "reminder_2h",
        "reminder_30min",
    ];

    fn body(template: &str, payload: &Value) -> String {
        render(template, Some(payload)).expect("known template").1
    }

    #[test]
    fn every_email_carries_the_call_link() {
        let payload = json!({
            "name": "Ada",
            "slot_start": "2026-10-12T07:00:00Z",
            "timezone": "Europe/Paris",
            "meeting_url": "https://zoom.us/j/123",
        });
        for template in TEMPLATES {
            assert!(
                body(template, &payload).contains("Join the call: https://zoom.us/j/123"),
                "{template}"
            );
        }
        assert_eq!(
            body("booking_confirmation", &payload),
            "Hi Ada,\n\nYour call is confirmed for Monday, 12 October 2026 at 09:00 (Europe/Paris).\n\nJoin the call: https://zoom.us/j/123\n\nCadence"
        );
    }

    #[test]
    fn emails_queued_with_the_old_key_keep_their_link() {
        let payload = json!({ "meet_link": "https://meet.google.com/abc-defg-hij" });
        assert!(body("reminder_2h", &payload).contains("https://meet.google.com/abc-defg-hij"));
    }

    #[test]
    fn no_link_means_no_join_line() {
        for payload in [json!({}), json!({ "meeting_url": null })] {
            assert!(!body("booking_confirmation", &payload).contains("Join"));
        }
    }

    #[test]
    fn a_cancellation_names_the_call_and_has_no_link() {
        let payload = json!({
            "name": "Ada",
            "slot_start": "2026-10-12T07:00:00Z",
            "timezone": "Europe/Paris",
            "meeting_url": "https://zoom.us/j/123",
        });
        let (subject, body) =
            render(BOOKING_CANCELLED_TEMPLATE, Some(&payload)).expect("known template");
        assert_eq!(subject, "Your Cadence call was cancelled");
        assert_eq!(
            body,
            "Hi Ada,\n\nYour host cancelled the call booked for Monday, 12 October 2026 at 09:00 (Europe/Paris).\n\nCadence"
        );
    }

    #[test]
    fn time_is_shown_in_the_invitees_timezone() {
        let at = |timezone: &str| local_time(Some("2026-10-12T07:00:00Z"), timezone);
        assert_eq!(
            at("Europe/Paris"),
            "Monday, 12 October 2026 at 09:00 (Europe/Paris)"
        );
        assert_eq!(
            at("Asia/Tbilisi"),
            "Monday, 12 October 2026 at 11:00 (Asia/Tbilisi)"
        );
        assert_eq!(at("Mars/Olympus"), "Monday, 12 October 2026 at 07:00 (UTC)");
        // Chrono writes sub-second precision; it must still parse.
        assert_eq!(
            local_time(Some("2026-10-12T22:30:00.123456Z"), "Asia/Tbilisi"),
            "Tuesday, 13 October 2026 at 02:30 (Asia/Tbilisi)"
        );
    }

    #[test]
    fn unknown_template_is_not_rendered() {
        assert!(render("newsletter", None).is_none());
    }
}
