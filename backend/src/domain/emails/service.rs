use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use time::{OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description};
use time_tz::{OffsetDateTimeExt, timezones};

use super::{
    db::{self, PendingEmail},
    models::{AdminEmail, EmailState},
};
use crate::{domain::bookings::models::BookingStatus, error::AppResult};

pub const BOOKING_CONFIRMATION_TEMPLATE: &str = "booking_confirmation";
pub const REMINDER_24H_TEMPLATE: &str = "reminder_24h";
pub const REMINDER_2H_TEMPLATE: &str = "reminder_2h";
pub const REMINDER_30MIN_TEMPLATE: &str = "reminder_30min";
/// Sent to the invitee when the host calls a booking off. Carries no links.
pub const BOOKING_CANCELLED_TEMPLATE: &str = "booking_cancelled";
/// Sent to the invitee after they moved their own call.
pub const BOOKING_RESCHEDULED_TEMPLATE: &str = "booking_rescheduled";
/// The `host_*` emails go to the host's account email, on the host's clock.
pub const HOST_NEW_BOOKING_TEMPLATE: &str = "host_new_booking";
pub const HOST_BOOKING_CANCELLED_TEMPLATE: &str = "host_booking_cancelled";
pub const HOST_BOOKING_RESCHEDULED_TEMPLATE: &str = "host_booking_rescheduled";
/// The only email that is not about a call.
pub const PASSWORD_RESET_TEMPLATE: &str = "password_reset";

const DEFAULT_APP_URL: &str = "http://localhost:3000";
/// Frontend routes the emails link to.
const MANAGE_PATH: &str = "/booking";
const HOST_BOOKINGS_PATH: &str = "/dashboard/bookings";
const RESET_PASSWORD_PATH: &str = "/reset-password";
/// The calendar entry's name when the page title and host are unknown.
const CALENDAR_TITLE: &str = "Cadence call";
const UTC: &str = "UTC";

// The app's light theme, as hex for mail clients.
const PAGE: &str = "#F1F3F8";
const CARD: &str = "#FFFFFF";
const INK: &str = "#151B3D";
const MUTED: &str = "#5F6685";
const BORDER: &str = "#DFE2EC";
const SIGNAL: &str = "#F0262D";
const DAWN: &str = "#FFC79A";
const NOON: &str = "#F0B030";
const DUSK: &str = "#5139D6";
const FONT: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif";
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
    } else if attempts >= db::MAX_ATTEMPTS {
        EmailState::Failed
    } else if attempts > 0 {
        EmailState::Retrying
    } else if send_at < now - Duration::minutes(OVERDUE_AFTER_MINUTES) {
        EmailState::Overdue
    } else {
        EmailState::Scheduled
    }
}

/// The public site, without a trailing slash: `APP_URL`, else the first
/// allowed origin, else the local frontend.
pub fn app_url() -> String {
    let first = |name: &str| {
        let value = std::env::var(name).ok()?;
        let url = value.split(',').next()?.trim().trim_end_matches('/');
        (!url.is_empty()).then(|| url.to_owned())
    };
    first("APP_URL")
        .or_else(|| first("CORS_ORIGINS"))
        .unwrap_or_else(|| DEFAULT_APP_URL.to_owned())
}

/// Where an invitee changes their own call.
pub fn manage_url(manage_token: &str) -> String {
    format!("{}{MANAGE_PATH}/{manage_token}", app_url())
}

/// Where a host reads their calls.
pub fn host_bookings_url() -> String {
    format!("{}{HOST_BOOKINGS_PATH}", app_url())
}

/// Where an account owner chooses a new password. The token is hex.
fn reset_password_url(token: &str) -> String {
    format!("{}{RESET_PASSWORD_PATH}?token={token}", app_url())
}

/// Queues the link that lets an account owner choose a new password.
pub async fn queue_password_reset(
    connection: &mut PgConnection,
    email: &str,
    full_name: &str,
    token: &str,
) -> AppResult<()> {
    db::queue(
        connection,
        email,
        PASSWORD_RESET_TEMPLATE,
        json!({ "name": full_name, "reset_url": reset_password_url(token) }),
    )
    .await
    .map_err(Into::into)
}

pub async fn forget_old_sent(pool: &PgPool) -> AppResult<()> {
    db::delete_old_sent(pool).await.map_err(Into::into)
}

/// What the worker does with an email that has come due.
pub enum Delivery {
    Send(RenderedEmail),
    /// The call it is about was cancelled or has started; saying so now
    /// would only confuse.
    Drop,
    UnknownTemplate,
}

pub fn delivery(email: &PendingEmail, now: DateTime<Utc>) -> Delivery {
    let template = email.template.as_str();
    let is_reminder = [
        REMINDER_24H_TEMPLATE,
        REMINDER_2H_TEMPLATE,
        REMINDER_30MIN_TEMPLATE,
    ]
    .contains(&template);
    let is_cancellation =
        [BOOKING_CANCELLED_TEMPLATE, HOST_BOOKING_CANCELLED_TEMPLATE].contains(&template);
    let call_cancelled = email.call_status.as_deref() == Some(BookingStatus::Cancelled.as_str());
    let call_started = email.call_start.is_some_and(|start| start <= now);
    if (call_cancelled && !is_cancellation) || (call_started && is_reminder) {
        return Delivery::Drop;
    }
    render(template, email.payload.as_ref()).map_or(Delivery::UnknownTemplate, Delivery::Send)
}

/// A reset link is a password in waiting: once the email is out, the queue
/// forgets it.
pub fn forgets_payload(template: &str) -> bool {
    template == PASSWORD_RESET_TEMPLATE
}

pub struct RenderedEmail {
    pub subject: String,
    pub text: String,
    pub html: String,
}

/// Everything an email says, before it is laid out as text or HTML.
struct Content {
    headline: String,
    greeting: String,
    /// `None` for an email that is not about a call.
    when: Option<When>,
    /// Who and what the call is, under its time.
    details: Vec<String>,
    /// Label and link, in order.
    buttons: Vec<(&'static str, String)>,
    manage_url: Option<String>,
    /// A quiet last line inside the card.
    note: Option<&'static str>,
    footer: &'static str,
}

/// Payloads queued before host names, lengths and manage links existed carry
/// only `name`, `slot_start`, `timezone` and `meeting_url`; whatever is
/// missing is left out.
pub fn render(template: &str, payload: Option<&Value>) -> Option<RenderedEmail> {
    let text = |key: &str| {
        payload
            .and_then(|value| value.get(key))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
    };
    let link = |key: &str| {
        text(key).filter(|url| url.starts_with("https://") || url.starts_with("http://"))
    };
    if template == PASSWORD_RESET_TEMPLATE {
        return Some(rendered(Content {
            headline: "Reset your Cadence password".to_owned(),
            greeting: greeting(text("name")),
            when: None,
            details: vec!["This link works for one hour and can be used once.".to_owned()],
            buttons: link("reset_url")
                .map(|url| ("Choose a new password", url.to_owned()))
                .into_iter()
                .collect(),
            manage_url: None,
            note: Some(
                "If you did not ask for this, ignore this email; your password stays the same.",
            ),
            footer: "You are getting this because a password reset was requested for your Cadence account.",
        }));
    }
    let host = text("host_name");
    let with_host = host.map(|name| format!(" with {name}")).unwrap_or_default();
    let invitee = text("name");
    let who = invitee.unwrap_or("Someone");
    let (for_host, headline) = match template {
        BOOKING_CONFIRMATION_TEMPLATE => (false, format!("Your call{with_host} is booked")),
        REMINDER_24H_TEMPLATE => (false, format!("Your call{with_host} is tomorrow")),
        REMINDER_2H_TEMPLATE => (false, format!("Your call{with_host} starts in 2 hours")),
        REMINDER_30MIN_TEMPLATE => (false, format!("Your call{with_host} starts in 30 minutes")),
        BOOKING_CANCELLED_TEMPLATE => (false, format!("Your call{with_host} was cancelled")),
        BOOKING_RESCHEDULED_TEMPLATE => (false, format!("Your call{with_host} was moved")),
        HOST_NEW_BOOKING_TEMPLATE => (true, format!("{who} booked a call")),
        HOST_BOOKING_CANCELLED_TEMPLATE => (true, format!("{who} cancelled their call")),
        HOST_BOOKING_RESCHEDULED_TEMPLATE => (true, format!("{who} moved their call")),
        _ => return None,
    };
    let minutes = payload
        .and_then(|value| value.get("minutes"))
        .and_then(Value::as_i64)
        .filter(|minutes| *minutes > 0);
    let when = When::new(text("slot_start"), text("timezone").unwrap_or(UTC), minutes);
    let title = text("title");

    let content = if for_host {
        Content {
            greeting: greeting(host),
            details: [
                invitee.map(|name| match text("invitee_email") {
                    Some(email) => format!("{name} ({email})"),
                    None => name.to_owned(),
                }),
                title.map(str::to_owned),
            ]
            .into_iter()
            .flatten()
            .collect(),
            buttons: link("bookings_url")
                .map(|url| ("Open your bookings", url.to_owned()))
                .into_iter()
                .collect(),
            manage_url: None,
            note: None,
            footer: "You are getting this because someone used your Cadence booking page.",
            headline,
            when: Some(when),
        }
    } else {
        let call = match (title, host) {
            (Some(title), Some(host)) => Some(format!("{title} with {host}")),
            (Some(title), None) => Some(title.to_owned()),
            (None, Some(host)) => Some(format!("With {host}")),
            (None, None) => None,
        };
        // A cancelled call has nothing left to join, save or change.
        let live = template != BOOKING_CANCELLED_TEMPLATE;
        // Emails queued before the host's own link existed used `meet_link`.
        let join = link("meeting_url")
            .or_else(|| link("meet_link"))
            .filter(|_| live);
        let calendar = when
            .calendar_dates
            .as_deref()
            .filter(|_| live)
            .map(|dates| {
                google_calendar_url(call.as_deref().unwrap_or(CALENDAR_TITLE), dates, join)
            });
        Content {
            greeting: greeting(invitee),
            buttons: [
                join.map(|url| ("Join the call", url.to_owned())),
                calendar.map(|url| ("Add to Google Calendar", url)),
            ]
            .into_iter()
            .flatten()
            .collect(),
            manage_url: link("manage_url").filter(|_| live).map(str::to_owned),
            note: None,
            footer: "You are getting this because a call was booked with your email address.",
            details: call.into_iter().collect(),
            headline,
            when: Some(when),
        }
    };

    Some(rendered(content))
}

fn rendered(content: Content) -> RenderedEmail {
    RenderedEmail {
        text: text_body(&content),
        html: html_body(&content),
        subject: content.headline,
    }
}

fn greeting(name: Option<&str>) -> String {
    format!("Hi {},", name.unwrap_or("there"))
}

/// When a call happens, on the reader's own clock. An unknown timezone falls
/// back to UTC and says so.
struct When {
    /// "Monday, 12 October 2026"
    date: String,
    /// "09:00 to 09:30 (Europe/Paris)", or only the start when the length is
    /// unknown. `None` when the time itself could not be read.
    time: Option<String>,
    /// The part-of-day colour of the start.
    accent: &'static str,
    /// Start and end as Google Calendar wants them, when the length is known.
    calendar_dates: Option<String>,
}

impl When {
    fn new(slot_start: Option<&str>, timezone: &str, minutes: Option<i64>) -> Self {
        let unknown = || Self {
            date: "The time you booked".to_owned(),
            time: None,
            accent: DUSK,
            calendar_dates: None,
        };
        let Some(start) = slot_start.and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
        else {
            return unknown();
        };
        let (zone, label) =
            timezones::get_by_name(timezone).map_or((None, UTC), |zone| (Some(zone), timezone));
        let local = |instant: OffsetDateTime| {
            zone.map_or_else(
                || instant.to_offset(time::UtcOffset::UTC),
                |zone| instant.to_timezone(zone),
            )
        };
        let clock =
            |instant: OffsetDateTime| local(instant).format(format_description!("[hour]:[minute]"));
        let end = minutes.map(|minutes| start + time::Duration::minutes(minutes));
        let (Ok(date), Ok(from)) = (
            local(start).format(format_description!(
                "[weekday], [day padding:none] [month repr:long] [year]"
            )),
            clock(start),
        ) else {
            return unknown();
        };
        let range = match end.map(clock) {
            Some(Ok(to)) => format!("{from} to {to}"),
            _ => from,
        };
        let stamp = |instant: OffsetDateTime| {
            instant
                .to_offset(time::UtcOffset::UTC)
                .format(format_description!(
                    "[year][month][day]T[hour][minute][second]Z"
                ))
                .ok()
        };
        Self {
            date,
            time: Some(format!("{range} ({label})")),
            accent: match local(start).hour() {
                0..12 => DAWN,
                12..17 => NOON,
                _ => DUSK,
            },
            calendar_dates: end.and_then(|end| Some(format!("{}/{}", stamp(start)?, stamp(end)?))),
        }
    }
}

fn google_calendar_url(title: &str, dates: &str, location: Option<&str>) -> String {
    let location = location
        .map(|url| format!("&location={}", urlencoding::encode(url)))
        .unwrap_or_default();
    format!(
        "https://calendar.google.com/calendar/render?action=TEMPLATE&text={}&dates={dates}{location}",
        urlencoding::encode(title),
    )
}

fn text_body(content: &Content) -> String {
    let call = content
        .when
        .iter()
        .flat_map(|when| std::iter::once(when.date.as_str()).chain(when.time.as_deref()))
        .chain(content.details.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let buttons = content
        .buttons
        .iter()
        .map(|(label, url)| format!("{label}: {url}"))
        .collect::<Vec<_>>()
        .join("\n");
    let manage = content
        .manage_url
        .as_ref()
        .map(|url| format!("Need to change it? Reschedule or cancel: {url}"));
    [
        Some(content.greeting.clone()),
        Some(format!("{}.", content.headline)),
        Some(call),
        Some(buttons).filter(|buttons| !buttons.is_empty()),
        manage,
        content.note.map(str::to_owned),
        Some("Cadence".to_owned()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n")
}

/// The one layout every email shares: tables and inline styles only, so mail
/// clients cope. Every value is escaped on its way in.
fn html_body(content: &Content) -> String {
    let headline = escape(&content.headline);
    let greeting = escape(&content.greeting);
    let call = content.when.as_ref().map(|when| {
        let time = when.time.as_deref().map(|time| {
            format!(
                r#"<div style="font-size:16px;line-height:24px;color:{INK};">{}</div>"#,
                escape(time)
            )
        });
        format!(
            r#"<tr><td style="padding:0 0 12px;"><table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0"><tr><td style="border-left:4px solid {accent};padding:6px 0 6px 16px;"><div style="font-size:16px;line-height:24px;font-weight:600;color:{INK};">{date}</div>{time}</td></tr></table></td></tr>"#,
            accent = when.accent,
            date = escape(&when.date),
            time = time.unwrap_or_default(),
        )
    });
    let details = content.details.iter().map(|line| {
        format!(
            r#"<tr><td style="padding:0 0 4px;font-size:15px;line-height:22px;color:{MUTED};">{}</td></tr>"#,
            escape(line)
        )
    });
    let buttons = content.buttons.iter().map(|(label, url)| {
        format!(
            r#"<tr><td style="padding:12px 0 0;"><table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr><td bgcolor="{INK}" style="background-color:{INK};border-radius:8px;"><a href="{}" style="display:inline-block;padding:12px 20px;font-size:15px;line-height:20px;font-weight:600;color:{CARD};text-decoration:none;">{label}</a></td></tr></table></td></tr>"#,
            escape(url)
        )
    });
    let manage = content.manage_url.as_ref().map(|url| {
        format!(
            r#"<tr><td style="padding:24px 0 0;font-size:14px;line-height:20px;color:{MUTED};">Need to change it? <a href="{}" style="color:{DUSK};text-decoration:underline;">Reschedule or cancel</a></td></tr>"#,
            escape(url)
        )
    });
    let bar = |height: u8, colour: &str| {
        format!(
            r#"<td valign="bottom" style="padding:0 3px 0 0;"><div style="width:4px;height:{height}px;background-color:{colour};border-radius:2px;font-size:0;line-height:0;">&nbsp;</div></td>"#
        )
    };
    let note = content.note.map(|note| {
        format!(
            r#"<tr><td style="padding:24px 0 0;font-size:14px;line-height:20px;color:{MUTED};">{note}</td></tr>"#
        )
    });
    let rows: String = call
        .into_iter()
        .chain(details)
        .chain(buttons)
        .chain(manage)
        .chain(note)
        .collect();
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{headline}</title>
</head>
<body style="margin:0;padding:0;background-color:{PAGE};">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" bgcolor="{PAGE}" style="background-color:{PAGE};">
<tr><td align="center" style="padding:32px 16px;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width:560px;font-family:{FONT};color:{INK};">
<tr><td style="padding:0 4px 20px;"><table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr>{bars}<td valign="bottom" style="padding:0 0 0 7px;font-size:18px;line-height:18px;font-weight:700;color:{INK};">Cadence</td></tr></table></td></tr>
<tr><td bgcolor="{CARD}" style="background-color:{CARD};border:1px solid {BORDER};border-radius:12px;padding:32px 28px;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
<tr><td style="padding:0 0 8px;font-size:15px;line-height:22px;color:{MUTED};">{greeting}</td></tr>
<tr><td style="padding:0 0 20px;font-size:22px;line-height:30px;font-weight:700;color:{INK};">{headline}</td></tr>
{rows}
</table>
</td></tr>
<tr><td style="padding:20px 4px 0;font-size:12px;line-height:18px;color:{MUTED};">{footer}</td></tr>
</table>
</td></tr>
</table>
</body>
</html>
"#,
        bars = [bar(12, INK), bar(18, SIGNAL), bar(9, INK)].concat(),
        footer = content.footer,
    )
}

/// Makes user input safe inside HTML text and attribute values.
fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_state_follows_the_queue_row() {
        let now = Utc::now();
        let late = now - Duration::minutes(OVERDUE_AFTER_MINUTES + 1);
        assert_eq!(state_of(Some(now), 3, late, now), EmailState::Sent);
        assert_eq!(state_of(None, 1, late, now), EmailState::Retrying);
        assert_eq!(
            state_of(None, db::MAX_ATTEMPTS - 1, late, now),
            EmailState::Retrying
        );
        assert_eq!(
            state_of(None, db::MAX_ATTEMPTS, late, now),
            EmailState::Failed
        );
        assert_eq!(state_of(None, 0, late, now), EmailState::Overdue);
        assert_eq!(state_of(None, 0, now, now), EmailState::Scheduled);
    }

    const INVITEE_TEMPLATES: [&str; 6] = [
        BOOKING_CONFIRMATION_TEMPLATE,
        REMINDER_24H_TEMPLATE,
        REMINDER_2H_TEMPLATE,
        REMINDER_30MIN_TEMPLATE,
        BOOKING_CANCELLED_TEMPLATE,
        BOOKING_RESCHEDULED_TEMPLATE,
    ];
    const HOST_TEMPLATES: [&str; 3] = [
        HOST_NEW_BOOKING_TEMPLATE,
        HOST_BOOKING_CANCELLED_TEMPLATE,
        HOST_BOOKING_RESCHEDULED_TEMPLATE,
    ];
    const HOSTILE: &str = "<b>Mia</b> & co";
    const HOSTILE_ESCAPED: &str = "&lt;b&gt;Mia&lt;/b&gt; &amp; co";

    fn email(template: &str, payload: &Value) -> RenderedEmail {
        render(template, Some(payload)).expect("known template")
    }

    /// 09:00 to 09:30 in Paris, the way the booking service queues it.
    fn invitee_payload(name: &str) -> Value {
        json!({
            "name": name,
            "slot_start": "2026-10-12T07:00:00Z",
            "timezone": "Europe/Paris",
            "meeting_url": "https://zoom.us/j/123?pwd=a&b=c",
            "host_name": name,
            "title": "Intro call",
            "minutes": 30,
            "manage_url": "https://cadence.example/booking/abc123",
        })
    }

    /// The same call on the host's clock in Tbilisi.
    fn host_payload(name: &str) -> Value {
        json!({
            "name": name,
            "invitee_email": "mia@example.com",
            "slot_start": "2026-10-12T07:00:00Z",
            "timezone": "Asia/Tbilisi",
            "host_name": "Nino",
            "title": "Intro call",
            "minutes": 30,
            "bookings_url": "https://cadence.example/dashboard/bookings",
        })
    }

    fn reset_payload(name: &str) -> Value {
        json!({
            "name": name,
            "reset_url": "https://cadence.example/reset-password?token=abc&x=1",
        })
    }

    fn headline(template: &str, name: &str) -> String {
        match template {
            PASSWORD_RESET_TEMPLATE => "Reset your Cadence password".to_owned(),
            BOOKING_CONFIRMATION_TEMPLATE => format!("Your call with {name} is booked"),
            REMINDER_24H_TEMPLATE => format!("Your call with {name} is tomorrow"),
            REMINDER_2H_TEMPLATE => format!("Your call with {name} starts in 2 hours"),
            REMINDER_30MIN_TEMPLATE => format!("Your call with {name} starts in 30 minutes"),
            BOOKING_CANCELLED_TEMPLATE => format!("Your call with {name} was cancelled"),
            BOOKING_RESCHEDULED_TEMPLATE => format!("Your call with {name} was moved"),
            HOST_NEW_BOOKING_TEMPLATE => format!("{name} booked a call"),
            HOST_BOOKING_CANCELLED_TEMPLATE => format!("{name} cancelled their call"),
            HOST_BOOKING_RESCHEDULED_TEMPLATE => format!("{name} moved their call"),
            other => panic!("no headline for {other}"),
        }
    }

    #[test]
    fn every_template_renders_with_user_input_escaped() {
        let cases = INVITEE_TEMPLATES
            .map(|template| (template, invitee_payload(HOSTILE)))
            .into_iter()
            .chain(HOST_TEMPLATES.map(|template| (template, host_payload(HOSTILE))))
            .chain([(PASSWORD_RESET_TEMPLATE, reset_payload(HOSTILE))]);
        for (template, payload) in cases {
            let email = email(template, &payload);
            assert_eq!(email.subject, headline(template, HOSTILE), "{template}");
            assert!(email.text.contains(&email.subject), "{template}");
            assert!(
                email.html.contains(&headline(template, HOSTILE_ESCAPED)),
                "{template}"
            );
            // Host emails greet the host; every other one greets `name`.
            assert_eq!(
                email.html.contains(&format!(">Hi {HOSTILE_ESCAPED},</td>")),
                !HOST_TEMPLATES.contains(&template),
                "{template}"
            );
            assert!(!email.html.contains("<b>"), "{template}");
            assert!(!email.html.contains("& co"), "{template}");
        }
    }

    #[test]
    fn an_invitee_email_carries_the_call_and_its_links() {
        let email = email(BOOKING_CONFIRMATION_TEMPLATE, &invitee_payload("Nino"));
        assert_eq!(
            email.text,
            "Hi Nino,\n\nYour call with Nino is booked.\n\nMonday, 12 October 2026\n09:00 to 09:30 (Europe/Paris)\nIntro call with Nino\n\nJoin the call: https://zoom.us/j/123?pwd=a&b=c\nAdd to Google Calendar: https://calendar.google.com/calendar/render?action=TEMPLATE&text=Intro%20call%20with%20Nino&dates=20261012T070000Z/20261012T073000Z&location=https%3A%2F%2Fzoom.us%2Fj%2F123%3Fpwd%3Da%26b%3Dc\n\nNeed to change it? Reschedule or cancel: https://cadence.example/booking/abc123\n\nCadence"
        );
        for part in [
            "09:00 to 09:30 (Europe/Paris)",
            r#"href="https://zoom.us/j/123?pwd=a&amp;b=c""#,
            "Join the call</a>",
            "Add to Google Calendar</a>",
            r#"href="https://cadence.example/booking/abc123""#,
            // 09:00 is morning.
            DAWN,
        ] {
            assert!(email.html.contains(part), "{part}");
        }
        for template in INVITEE_TEMPLATES {
            let html = self::email(template, &invitee_payload("Nino")).html;
            let live = template != BOOKING_CANCELLED_TEMPLATE;
            for part in [
                "Join the call",
                "Add to Google Calendar",
                "Reschedule or cancel",
            ] {
                assert_eq!(html.contains(part), live, "{template}: {part}");
            }
        }
    }

    #[test]
    fn a_host_email_names_the_invitee_on_the_hosts_clock() {
        for template in HOST_TEMPLATES {
            let email = email(template, &host_payload("Mia Chen"));
            for part in [
                "Mia Chen (mia@example.com)",
                "11:00 to 11:30 (Asia/Tbilisi)",
                "https://cadence.example/dashboard/bookings",
            ] {
                assert!(email.text.contains(part), "{template}: {part}");
                assert!(email.html.contains(part), "{template}: {part}");
            }
            assert!(email.text.starts_with("Hi Nino,"), "{template}");
            assert!(email.html.contains(">Hi Nino,</td>"), "{template}");
            assert!(email.html.contains("Open your bookings</a>"), "{template}");
            assert!(!email.html.contains("Reschedule or cancel"), "{template}");
        }
    }

    #[test]
    fn a_password_reset_email_has_one_link_and_no_call() {
        let email = email(PASSWORD_RESET_TEMPLATE, &reset_payload("Nino"));
        assert_eq!(email.subject, "Reset your Cadence password");
        assert_eq!(
            email.text,
            "Hi Nino,\n\nReset your Cadence password.\n\nThis link works for one hour and can be used once.\n\nChoose a new password: https://cadence.example/reset-password?token=abc&x=1\n\nIf you did not ask for this, ignore this email; your password stays the same.\n\nCadence"
        );
        for part in [
            "This link works for one hour and can be used once.",
            r#"href="https://cadence.example/reset-password?token=abc&amp;x=1""#,
            "Choose a new password</a>",
            "If you did not ask for this, ignore this email; your password stays the same.",
        ] {
            assert!(email.html.contains(part), "{part}");
        }
        assert_eq!(email.html.matches("<a ").count(), 1);
        for part in [
            "border-left:4px solid",
            "Join the call",
            "Reschedule or cancel",
        ] {
            assert!(!email.html.contains(part), "{part}");
        }
        // A link that is not a web address is left out, like every other link.
        let unsafe_link = json!({ "reset_url": "javascript:alert(1)" });
        assert!(
            !self::email(PASSWORD_RESET_TEMPLATE, &unsafe_link)
                .html
                .contains("<a ")
        );
    }

    #[test]
    fn a_payload_queued_before_the_new_fields_still_renders() {
        let payload = json!({
            "name": "Ada",
            "slot_start": "2026-10-12T16:00:00Z",
            "timezone": "Europe/Paris",
            "meeting_url": "https://zoom.us/j/123",
        });
        for template in INVITEE_TEMPLATES {
            let email = email(template, &payload);
            assert!(email.subject.starts_with("Your call "), "{template}");
            assert!(!email.subject.contains("with"), "{template}");
            assert!(email.text.contains("18:00 (Europe/Paris)"), "{template}");
            assert!(email.html.contains("18:00 (Europe/Paris)"), "{template}");
            // 18:00 is evening; the length is unknown, so nothing to add to a calendar.
            assert!(
                email
                    .html
                    .contains(&format!("border-left:4px solid {DUSK}"))
            );
            assert!(!email.html.contains("Add to Google Calendar"), "{template}");
            assert!(!email.html.contains("Reschedule or cancel"), "{template}");
        }
        assert_eq!(
            email(BOOKING_CONFIRMATION_TEMPLATE, &payload).text,
            "Hi Ada,\n\nYour call is booked.\n\nMonday, 12 October 2026\n18:00 (Europe/Paris)\n\nJoin the call: https://zoom.us/j/123\n\nCadence"
        );
    }

    #[test]
    fn emails_queued_with_the_old_key_keep_their_link() {
        let payload = json!({ "meet_link": "https://meet.google.com/abc-defg-hij" });
        let email = email(REMINDER_2H_TEMPLATE, &payload);
        assert!(email.text.contains("https://meet.google.com/abc-defg-hij"));
        assert!(email.html.contains("https://meet.google.com/abc-defg-hij"));
    }

    #[test]
    fn no_link_means_no_join_line() {
        for payload in [
            json!({}),
            json!({ "meeting_url": null }),
            json!({ "meeting_url": "javascript:alert(1)" }),
        ] {
            let email = email(BOOKING_CONFIRMATION_TEMPLATE, &payload);
            assert!(!email.text.contains("Join"));
            assert!(!email.html.contains("Join"));
        }
    }

    #[test]
    fn time_is_shown_on_the_readers_clock() {
        let at = |slot_start: &str, timezone: &str| {
            let when = When::new(Some(slot_start), timezone, Some(45));
            (when.date, when.time.unwrap_or_default(), when.accent)
        };
        let morning = "2026-10-12T07:00:00Z";
        assert_eq!(
            at(morning, "Europe/Paris"),
            (
                "Monday, 12 October 2026".to_owned(),
                "09:00 to 09:45 (Europe/Paris)".to_owned(),
                DAWN
            )
        );
        assert_eq!(
            at(morning, "Asia/Tbilisi").1,
            "11:00 to 11:45 (Asia/Tbilisi)"
        );
        assert_eq!(at(morning, "Mars/Olympus").1, "07:00 to 07:45 (UTC)");
        assert_eq!(at("2026-10-12T10:00:00Z", "Europe/Paris").2, NOON);
        assert_eq!(at("2026-10-12T14:59:00Z", "Europe/Paris").2, NOON);
        assert_eq!(at("2026-10-12T15:00:00Z", "Europe/Paris").2, DUSK);
        // Chrono writes sub-second precision; it must still parse.
        assert_eq!(
            at("2026-10-12T22:30:00.123456Z", "Asia/Tbilisi"),
            (
                "Tuesday, 13 October 2026".to_owned(),
                "02:30 to 03:15 (Asia/Tbilisi)".to_owned(),
                DAWN
            )
        );
        assert!(When::new(Some("soon"), UTC, Some(30)).time.is_none());
    }

    #[test]
    fn unknown_template_is_not_rendered() {
        assert!(render("newsletter", None).is_none());
    }

    /// Writes one HTML file per template for looking at in a browser:
    /// `CADENCE_EMAIL_PREVIEW_DIR=/some/dir cargo test -p cadence-backend --lib email_previews -- --ignored`
    #[test]
    #[ignore = "writes files; needs CADENCE_EMAIL_PREVIEW_DIR"]
    fn email_previews() {
        let directory = std::path::PathBuf::from(
            std::env::var("CADENCE_EMAIL_PREVIEW_DIR").expect("CADENCE_EMAIL_PREVIEW_DIR is set"),
        );
        std::fs::create_dir_all(&directory).expect("preview directory can be created");
        // Morning, afternoon and evening in Paris, so every accent shows.
        let starts = [
            "2026-10-12T07:00:00Z",
            "2026-10-13T11:00:00Z",
            "2026-10-14T16:30:00Z",
        ];
        let mut invitee = invitee_payload("Nino Beridze");
        invitee["name"] = json!("Mia Chen");
        let cases = INVITEE_TEMPLATES
            .map(|template| (template, invitee.clone()))
            .into_iter()
            .chain(HOST_TEMPLATES.map(|template| (template, host_payload("Mia Chen"))))
            .chain([(PASSWORD_RESET_TEMPLATE, reset_payload("Nino Beridze"))]);
        for (index, (template, mut payload)) in cases.enumerate() {
            payload["slot_start"] = json!(starts[index % starts.len()]);
            std::fs::write(
                directory.join(format!("{template}.html")),
                email(template, &payload).html,
            )
            .expect("preview can be written");
        }
    }
}
