use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description};
use time_tz::{OffsetDateTimeExt, timezones};

const UTC: &str = "UTC";

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
