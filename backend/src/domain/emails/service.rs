use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use time::{OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description};
use time_tz::{OffsetDateTimeExt, timezones};

use super::{
    db::{self, PendingEmail},
    models::{AdminEmail, EmailState},
};
use crate::{
    domain::{
        billing::plans::Reminder,
        bookings::models::{Booking, BookingStatus},
        host_settings::models::BookingPage,
    },
    error::AppResult,
};

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
/// The account emails: not about a call.
pub const PASSWORD_RESET_TEMPLATE: &str = "password_reset";
pub const TRIAL_ENDING_TEMPLATE: &str = "trial_ending";
/// For hosts from before plans existed: their trial started with the deploy.
/// Queued only by migration 0022.
pub const TRIAL_STARTED_TEMPLATE: &str = "trial_started";
pub const HOST_BOOKING_LIMIT_TEMPLATE: &str = "host_booking_limit";
pub const PAYMENT_FAILED_TEMPLATE: &str = "payment_failed";
/// Business: the invitee has not said they are coming, 2 hours before.
pub const HOST_UNCONFIRMED_TEMPLATE: &str = "host_unconfirmed";
/// Business: a host's one "here are my times" to someone who left a booking.
pub const LEAD_FOLLOW_UP_TEMPLATE: &str = "lead_follow_up";
/// Business: the week's numbers, on Monday morning.
pub const WEEKLY_INSIGHTS_TEMPLATE: &str = "weekly_insights";

const DEFAULT_APP_URL: &str = "http://localhost:3000";
/// The header logo travels inside each email, because mail clients cannot
/// fetch images from a local or not-yet-deployed site.
pub const LOGO_CID: &str = "cadence-logo";
pub const LOGO_PNG: &[u8] = include_bytes!("logo.png");
/// Frontend routes the emails link to.
const MANAGE_PATH: &str = "/booking";
const HOST_BOOKINGS_PATH: &str = "/dashboard/bookings";
const RESET_PASSWORD_PATH: &str = "/reset-password";
const BILLING_PATH: &str = "/dashboard/billing";
const DASHBOARD_PATH: &str = "/dashboard";
/// The calendar entry's name when the page title and host are unknown.
const CALENDAR_TITLE: &str = "Cadence call";
const UTC: &str = "UTC";

// The app's light theme, as hex for mail clients.
const PAGE: &str = "#F1F3F8";
const CARD: &str = "#FFFFFF";
const INK: &str = "#151B3D";
const MUTED: &str = "#5F6685";
const BORDER: &str = "#DFE2EC";
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
    let payload = payload_of(&ResetEmail {
        name: Some(full_name.to_owned()),
        reset_url: Some(reset_password_url(token)),
    })?;
    db::queue(
        connection,
        None,
        email,
        PASSWORD_RESET_TEMPLATE,
        Utc::now(),
        payload,
    )
    .await
    .map_err(Into::into)
}

/// What a host is told about their plan.
#[derive(Clone, Copy)]
pub enum AccountNotice {
    /// A few days before the Pro trial ends.
    TrialEnding,
    /// The Free plan's bookings for the month are used up.
    BookingLimit,
    /// A renewal failed; Stripe is retrying it.
    PaymentFailed,
}

impl AccountNotice {
    pub const fn template(self) -> &'static str {
        match self {
            Self::TrialEnding => TRIAL_ENDING_TEMPLATE,
            Self::BookingLimit => HOST_BOOKING_LIMIT_TEMPLATE,
            Self::PaymentFailed => PAYMENT_FAILED_TEMPLATE,
        }
    }
}

/// Queues a notice about the host's plan, to go out at `send_at`.
pub async fn queue_account_notice(
    connection: &mut PgConnection,
    notice: AccountNotice,
    email: &str,
    full_name: &str,
    send_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let payload = payload_of(&AccountEmail {
        name: Some(full_name.to_owned()),
        billing_url: Some(format!("{}{BILLING_PATH}", app_url())),
        losing: None,
    })?;
    db::queue(connection, None, email, notice.template(), send_at, payload).await
}

/// Takes back a notice that has not gone out, such as the trial ending for
/// a host who has since paid.
pub async fn drop_account_notice(
    connection: &mut PgConnection,
    notice: AccountNotice,
    email: &str,
) -> Result<(), sqlx::Error> {
    db::drop_unsent_to(connection, email, notice.template()).await
}

/// Who called a call off decides who still has to hear about it.
#[derive(Clone, Copy)]
pub enum CancelledBy {
    Host,
    Invitee,
}

/// A call was booked: the invitee gets the confirmation and the reminders
/// still ahead, and the host hears of it.
///
/// With `attendance`, the day-before reminder asks the invitee to confirm,
/// and the host hears 2 hours ahead if they have not.
pub async fn booked(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    reminders: &[Reminder],
    attendance: bool,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    queue_for_invitee(
        connection,
        booking,
        page,
        BOOKING_CONFIRMATION_TEMPLATE,
        reminders,
        attendance,
        now,
    )
    .await?;
    queue_for_host(connection, booking, page, HOST_NEW_BOOKING_TEMPLATE, now).await?;
    queue_unconfirmed(connection, booking, page, attendance, now).await
}

/// A call moved: the reminders for its old time go, and both sides hear of
/// the new one.
pub async fn rescheduled(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    reminders: &[Reminder],
    attendance: bool,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    db::drop_unsent(connection, booking.unid).await?;
    queue_for_invitee(
        connection,
        booking,
        page,
        BOOKING_RESCHEDULED_TEMPLATE,
        reminders,
        attendance,
        now,
    )
    .await?;
    queue_for_host(
        connection,
        booking,
        page,
        HOST_BOOKING_RESCHEDULED_TEMPLATE,
        now,
    )
    .await?;
    queue_unconfirmed(connection, booking, page, attendance, now).await
}

/// How long before a call the host hears that it is still unconfirmed.
const UNCONFIRMED_NOTICE: Duration = Duration::hours(2);

/// Only the day-before reminder asks the invitee to confirm.
///
/// A call booked or moved to less than a day ahead was never asked, so its
/// host is not told the invitee "has not confirmed".
fn asked_to_confirm(slot_start: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    slot_start - Duration::hours(24) >= now
}

/// The host's heads-up for a call nobody has confirmed. The worker drops it
/// if the invitee confirms first.
async fn queue_unconfirmed(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    attendance: bool,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let send_at = booking.slot_start - UNCONFIRMED_NOTICE;
    if !attendance || !asked_to_confirm(booking.slot_start, now) || send_at < now {
        return Ok(());
    }
    let payload = payload_of(&HostEmail::of(booking, page))?;
    db::queue(
        connection,
        Some(booking.unid),
        &page.host_email,
        HOST_UNCONFIRMED_TEMPLATE,
        send_at,
        payload,
    )
    .await
}

/// A host's one email to someone who started booking and left: their link,
/// and an invitation to pick a time.
pub async fn queue_lead_follow_up(
    connection: &mut PgConnection,
    email: &str,
    name: Option<&str>,
    host_name: &str,
    booking_url: &str,
) -> Result<(), sqlx::Error> {
    let payload = payload_of(&FollowUpEmail {
        name: name.map(str::to_owned),
        host_name: Some(host_name.to_owned()),
        booking_url: Some(booking_url.to_owned()),
    })?;
    db::queue(
        connection,
        None,
        email,
        LEAD_FOLLOW_UP_TEMPLATE,
        Utc::now(),
        payload,
    )
    .await
}

/// The week's numbers for a Business host, one line each.
pub async fn queue_weekly_insights(
    connection: &mut PgConnection,
    email: &str,
    name: &str,
    lines: Vec<String>,
) -> Result<(), sqlx::Error> {
    let payload = payload_of(&InsightsEmail {
        name: Some(name.to_owned()),
        lines: Some(lines),
        dashboard_url: Some(format!("{}{DASHBOARD_PATH}", app_url())),
    })?;
    db::queue(
        connection,
        None,
        email,
        WEEKLY_INSIGHTS_TEMPLATE,
        Utc::now(),
        payload,
    )
    .await
}

/// A call was called off: nothing still waiting goes out, and the side that
/// did not cancel is told.
pub async fn cancelled(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    by: CancelledBy,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    db::drop_unsent(connection, booking.unid).await?;
    match by {
        CancelledBy::Host => {
            let payload = payload_of(&InviteeEmail::of(booking, page))?;
            db::queue(
                connection,
                Some(booking.unid),
                &booking.invitee_email,
                BOOKING_CANCELLED_TEMPLATE,
                now,
                payload,
            )
            .await
        }
        CancelledBy::Invitee => {
            queue_for_host(
                connection,
                booking,
                page,
                HOST_BOOKING_CANCELLED_TEMPLATE,
                now,
            )
            .await
        }
    }
}

/// A reminder's template and how long before the call it goes out.
const fn reminder_email(reminder: Reminder) -> (&'static str, Duration) {
    match reminder {
        Reminder::Before24h => (REMINDER_24H_TEMPLATE, Duration::hours(24)),
        Reminder::Before2h => (REMINDER_2H_TEMPLATE, Duration::hours(2)),
        Reminder::Before30m => (REMINDER_30MIN_TEMPLATE, Duration::minutes(30)),
    }
}

/// The notice the invitee gets now, and the reminders that are still ahead.
async fn queue_for_invitee(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    notice: &str,
    reminders: &[Reminder],
    attendance: bool,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let mut email = InviteeEmail::of(booking, page);
    if attendance {
        email.attend_url = Some(format!("{}?attend=yes", manage_url(&booking.manage_token)));
    }
    let payload = payload_of(&email)?;
    let start = booking.slot_start;
    let reminders = reminders.iter().map(|reminder| {
        let (template, before) = reminder_email(*reminder);
        (template, start - before)
    });
    for (template, send_at) in std::iter::once((notice, now)).chain(reminders) {
        if send_at >= now {
            db::queue(
                connection,
                Some(booking.unid),
                &booking.invitee_email,
                template,
                send_at,
                payload.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

async fn queue_for_host(
    connection: &mut PgConnection,
    booking: &Booking,
    page: &BookingPage,
    template: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let payload = payload_of(&HostEmail::of(booking, page))?;
    db::queue(
        connection,
        Some(booking.unid),
        &page.host_email,
        template,
        now,
        payload,
    )
    .await
}

fn payload_of(email: &impl Serialize) -> Result<Value, sqlx::Error> {
    serde_json::to_value(email).map_err(|error| sqlx::Error::Encode(Box::new(error)))
}

/// What an email to the invitee says, in the invitee's own timezone.
///
/// The same type writes the queue row and reads it back. Every field is
/// optional and read on its own: rows queued before a field existed, or
/// with a value that no longer reads, still render with what they have.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct InviteeEmail {
    #[serde(deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    slot_start: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "lenient")]
    timezone: Option<String>,
    #[serde(deserialize_with = "lenient")]
    meeting_url: Option<String>,
    /// Rows queued before the host's own link existed named it this.
    #[serde(deserialize_with = "lenient", skip_serializing)]
    meet_link: Option<String>,
    #[serde(deserialize_with = "lenient")]
    host_name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    title: Option<String>,
    #[serde(deserialize_with = "lenient")]
    minutes: Option<i64>,
    #[serde(deserialize_with = "lenient")]
    manage_url: Option<String>,
    /// Business: confirms the invitee is coming, from the day-before reminder.
    #[serde(deserialize_with = "lenient")]
    attend_url: Option<String>,
}

impl InviteeEmail {
    fn of(booking: &Booking, page: &BookingPage) -> Self {
        Self {
            name: Some(booking.invitee_name.clone()),
            slot_start: Some(booking.slot_start),
            timezone: Some(booking.timezone.clone()),
            meeting_url: booking.meeting_url.clone(),
            meet_link: None,
            host_name: Some(page.host_name.clone()),
            title: Some(page.title.clone()),
            minutes: Some(i64::from(booking.minutes)),
            manage_url: Some(manage_url(&booking.manage_token)),
            attend_url: None,
        }
    }
}

/// What an email to the host says, in the host's own timezone.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct HostEmail {
    /// The invitee's name.
    #[serde(deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    invitee_email: Option<String>,
    #[serde(deserialize_with = "lenient")]
    slot_start: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "lenient")]
    timezone: Option<String>,
    #[serde(deserialize_with = "lenient")]
    host_name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    title: Option<String>,
    #[serde(deserialize_with = "lenient")]
    minutes: Option<i64>,
    #[serde(deserialize_with = "lenient")]
    bookings_url: Option<String>,
}

impl HostEmail {
    fn of(booking: &Booking, page: &BookingPage) -> Self {
        Self {
            name: Some(booking.invitee_name.clone()),
            invitee_email: Some(booking.invitee_email.clone()),
            slot_start: Some(booking.slot_start),
            timezone: Some(page.timezone.clone()),
            host_name: Some(page.host_name.clone()),
            title: Some(page.title.clone()),
            minutes: Some(i64::from(booking.minutes)),
            bookings_url: Some(host_bookings_url()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ResetEmail {
    #[serde(deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    reset_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct FollowUpEmail {
    #[serde(deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    host_name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    booking_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct InsightsEmail {
    #[serde(deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    lines: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient")]
    dashboard_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct AccountEmail {
    #[serde(deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(deserialize_with = "lenient")]
    billing_url: Option<String>,
    /// The trial email: what this host would lose on Free, filled in when it
    /// is sent. Rows without it say it in general.
    #[serde(deserialize_with = "lenient", skip_serializing_if = "Option::is_none")]
    losing: Option<Vec<String>>,
}

/// The trial email's payload with what the host would lose, read when it
/// goes out rather than at sign-up.
pub fn with_trial_losses(payload: Option<&Value>, losses: Vec<String>) -> Option<Value> {
    let mut email: AccountEmail = read(payload);
    email.losing = Some(losses);
    with_billing_link(email)
}

/// An account email's payload with the billing link, which rows queued by a
/// migration leave out: the app's address is not known there.
pub fn with_billing_url(payload: Option<&Value>) -> Option<Value> {
    with_billing_link(read(payload))
}

fn with_billing_link(mut email: AccountEmail) -> Option<Value> {
    email
        .billing_url
        .get_or_insert_with(|| format!("{}{BILLING_PATH}", app_url()));
    payload_of(&email).ok()
}

/// A field that does not read as its type counts as missing, instead of
/// making the whole email unreadable.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(Option::<Value>::deserialize(deserializer)?
        .and_then(|value| serde_json::from_value(value).ok()))
}

/// A stored payload as `T`. Anything that is not an object reads as empty.
fn read<T: DeserializeOwned + Default>(payload: Option<&Value>) -> T {
    payload
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

/// A field with something in it.
fn text(value: Option<&String>) -> Option<&str> {
    value
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

/// A field that holds a web link.
fn link(value: Option<&String>) -> Option<&str> {
    text(value).filter(|url| url.starts_with("https://") || url.starts_with("http://"))
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
        HOST_UNCONFIRMED_TEMPLATE,
    ]
    .contains(&template);
    // The invitee confirmed after the heads-up was queued.
    if template == HOST_UNCONFIRMED_TEMPLATE && email.call_confirmed == Some(true) {
        return Delivery::Drop;
    }
    let is_cancellation =
        [BOOKING_CANCELLED_TEMPLATE, HOST_BOOKING_CANCELLED_TEMPLATE].contains(&template);
    let call_cancelled = email.call_status == Some(BookingStatus::Cancelled);
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
    let content = match template {
        PASSWORD_RESET_TEMPLATE => reset_content(&read(payload)),
        TRIAL_STARTED_TEMPLATE
        | TRIAL_ENDING_TEMPLATE
        | HOST_BOOKING_LIMIT_TEMPLATE
        | PAYMENT_FAILED_TEMPLATE => account_content(template, &read(payload)),
        HOST_NEW_BOOKING_TEMPLATE
        | HOST_BOOKING_CANCELLED_TEMPLATE
        | HOST_BOOKING_RESCHEDULED_TEMPLATE
        | HOST_UNCONFIRMED_TEMPLATE => host_content(template, &read(payload)),
        LEAD_FOLLOW_UP_TEMPLATE => follow_up_content(&read(payload)),
        WEEKLY_INSIGHTS_TEMPLATE => insights_content(&read(payload)),
        BOOKING_CONFIRMATION_TEMPLATE
        | REMINDER_24H_TEMPLATE
        | REMINDER_2H_TEMPLATE
        | REMINDER_30MIN_TEMPLATE
        | BOOKING_CANCELLED_TEMPLATE
        | BOOKING_RESCHEDULED_TEMPLATE => invitee_content(template, &read(payload)),
        _ => return None,
    };
    Some(rendered(content))
}

fn follow_up_content(email: &FollowUpEmail) -> Content {
    let host = text(email.host_name.as_ref()).unwrap_or("your host");
    Content {
        headline: format!("Still want to talk with {host}?"),
        greeting: greeting(text(email.name.as_ref())),
        when: None,
        details: vec![format!(
            "You started booking a call with {host} but did not pick a time. Their open times are one click away."
        )],
        buttons: link(email.booking_url.as_ref())
            .map(|url| ("Pick a time", url.to_owned()))
            .into_iter()
            .collect(),
        manage_url: None,
        note: Some("This is the only email about it. Nothing more is sent if you do not book."),
        footer: "You are getting this because you started booking on a Cadence booking page.",
    }
}

fn insights_content(email: &InsightsEmail) -> Content {
    Content {
        headline: "Your week in bookings".to_owned(),
        greeting: greeting(text(email.name.as_ref())),
        when: None,
        details: email.lines.clone().unwrap_or_default(),
        buttons: link(email.dashboard_url.as_ref())
            .map(|url| ("Open your dashboard", url.to_owned()))
            .into_iter()
            .collect(),
        manage_url: None,
        note: None,
        footer: "You are getting this because you have Cadence Business.",
    }
}

fn reset_content(email: &ResetEmail) -> Content {
    Content {
        headline: "Reset your Cadence password".to_owned(),
        greeting: greeting(text(email.name.as_ref())),
        when: None,
        details: vec!["This link works for one hour and can be used once.".to_owned()],
        buttons: link(email.reset_url.as_ref())
            .map(|url| ("Choose a new password", url.to_owned()))
            .into_iter()
            .collect(),
        manage_url: None,
        note: Some("If you did not ask for this, ignore this email; your password stays the same."),
        footer: "You are getting this because a password reset was requested for your Cadence account.",
    }
}

fn account_content(template: &str, email: &AccountEmail) -> Content {
    let (headline, details, button) = match template {
        TRIAL_STARTED_TEMPLATE => (
            "Your page is on Pro, free for 14 days",
            "Cadence now has plans. So nothing you use changes today, you are on a \
             14-day Pro trial: reminders 2 hours and 30 minutes before, your own form \
             questions and no Cadence logo. Three days before it ends, we will email you \
             what Free would change for you. Pick a plan whenever you like.",
            "See plans",
        ),
        TRIAL_ENDING_TEMPLATE => (
            "Your Pro trial ends in 3 days",
            "After that your page moves to Free: 25 included bookings a month and one \
             reminder. Your settings stay saved. Pick a plan to keep everything as it is.",
            "Pick a plan",
        ),
        HOST_BOOKING_LIMIT_TEMPLATE => (
            "You've had 25 bookings this month",
            "Free includes 25 a month. Your page stays open, so nobody is turned away. \
             Pro adds reminders 2 hours and 30 minutes before, your own form questions \
             and no Cadence logo, from $8 a month.",
            "See plans",
        ),
        _ => (
            "Your Cadence payment failed",
            "We could not charge your card for your plan. Stripe will try again over the \
             next few days. Update your card so your plan keeps running.",
            "Update your card",
        ),
    };
    let losing = email
        .losing
        .as_deref()
        .filter(|lines| template == TRIAL_ENDING_TEMPLATE && !lines.is_empty());
    let details = losing.map_or_else(
        || vec![details.to_owned()],
        |lines| {
            std::iter::once("After that your page moves to Free. What changes for you:".to_owned())
                .chain(lines.iter().cloned())
                .chain(std::iter::once(
                    "Your settings stay saved. Pick a plan to keep everything as it is.".to_owned(),
                ))
                .collect()
        },
    );
    Content {
        headline: headline.to_owned(),
        greeting: greeting(text(email.name.as_ref())),
        when: None,
        details,
        buttons: link(email.billing_url.as_ref())
            .map(|url| (button, url.to_owned()))
            .into_iter()
            .collect(),
        manage_url: None,
        note: None,
        footer: "You are getting this because you have a Cadence account.",
    }
}

/// When a call happens, from what an email payload says about it.
fn when_of(
    slot_start: Option<DateTime<Utc>>,
    timezone: Option<&String>,
    minutes: Option<i64>,
) -> When {
    let start = slot_start.map(|start| start.to_rfc3339());
    When::new(
        start.as_deref(),
        text(timezone).unwrap_or(UTC),
        minutes.filter(|minutes| *minutes > 0),
    )
}

fn host_content(template: &str, email: &HostEmail) -> Content {
    let invitee = text(email.name.as_ref());
    let who = invitee.unwrap_or("Someone");
    let headline = match template {
        HOST_NEW_BOOKING_TEMPLATE => format!("{who} booked a call"),
        HOST_BOOKING_CANCELLED_TEMPLATE => format!("{who} cancelled their call"),
        HOST_UNCONFIRMED_TEMPLATE => format!("{who} has not confirmed yet"),
        _ => format!("{who} moved their call"),
    };
    let heads_up = (template == HOST_UNCONFIRMED_TEMPLATE).then(|| {
        "The call starts in 2 hours and they have not said they are coming. A short note may help.".to_owned()
    });
    Content {
        greeting: greeting(text(email.host_name.as_ref())),
        details: [
            invitee.map(|name| {
                text(email.invitee_email.as_ref())
                    .map_or_else(|| name.to_owned(), |address| format!("{name} ({address})"))
            }),
            text(email.title.as_ref()).map(str::to_owned),
            heads_up,
        ]
        .into_iter()
        .flatten()
        .collect(),
        buttons: link(email.bookings_url.as_ref())
            .map(|url| ("Open your bookings", url.to_owned()))
            .into_iter()
            .collect(),
        manage_url: None,
        note: None,
        footer: "You are getting this because someone used your Cadence booking page.",
        headline,
        when: Some(when_of(
            email.slot_start,
            email.timezone.as_ref(),
            email.minutes,
        )),
    }
}

fn invitee_content(template: &str, email: &InviteeEmail) -> Content {
    let host = text(email.host_name.as_ref());
    let with_host = host.map(|name| format!(" with {name}")).unwrap_or_default();
    let headline = match template {
        BOOKING_CONFIRMATION_TEMPLATE => format!("Your call{with_host} is booked"),
        REMINDER_24H_TEMPLATE => format!("Your call{with_host} is tomorrow"),
        REMINDER_2H_TEMPLATE => format!("Your call{with_host} starts in 2 hours"),
        REMINDER_30MIN_TEMPLATE => format!("Your call{with_host} starts in 30 minutes"),
        BOOKING_CANCELLED_TEMPLATE => format!("Your call{with_host} was cancelled"),
        _ => format!("Your call{with_host} was moved"),
    };
    let when = when_of(email.slot_start, email.timezone.as_ref(), email.minutes);
    let call = match (text(email.title.as_ref()), host) {
        (Some(title), Some(host)) => Some(format!("{title} with {host}")),
        (Some(title), None) => Some(title.to_owned()),
        (None, Some(host)) => Some(format!("With {host}")),
        (None, None) => None,
    };
    // A cancelled call has nothing left to join, save or change.
    let live = template != BOOKING_CANCELLED_TEMPLATE;
    let join = link(email.meeting_url.as_ref())
        .or_else(|| link(email.meet_link.as_ref()))
        .filter(|_| live);
    // Business: the day before, the invitee says whether they are coming.
    let attend =
        link(email.attend_url.as_ref()).filter(|_| live && template == REMINDER_24H_TEMPLATE);
    let calendar = when
        .calendar_dates
        .as_deref()
        .filter(|_| live)
        .map(|dates| google_calendar_url(call.as_deref().unwrap_or(CALENDAR_TITLE), dates, join));
    Content {
        greeting: greeting(text(email.name.as_ref())),
        buttons: [
            attend.map(|url| ("I'm coming", url.to_owned())),
            attend
                .and_then(|_| link(email.manage_url.as_ref()))
                .map(|url| ("I need another time", url.to_owned())),
            join.map(|url| ("Join the call", url.to_owned())),
            calendar.map(|url| ("Add to Google Calendar", url)),
        ]
        .into_iter()
        .flatten()
        .collect(),
        manage_url: link(email.manage_url.as_ref())
            .filter(|_| live)
            .map(str::to_owned),
        note: None,
        footer: "You are getting this because a call was booked with your email address.",
        details: call.into_iter().collect(),
        headline,
        when: Some(when),
    }
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
<tr><td style="padding:0 4px 20px;"><table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr><td valign="middle"><img src="cid:{LOGO_CID}" width="18" height="18" alt="" style="display:block;border:0;"></td><td valign="middle" style="padding:0 0 0 7px;font-size:18px;line-height:18px;font-weight:700;color:{INK};">Cadence</td></tr></table></td></tr>
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
    use serde_json::json;

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
    fn only_a_call_a_day_away_asks_the_invitee_to_confirm() {
        let now = Utc::now();
        assert!(asked_to_confirm(now + Duration::hours(30), now));
        assert!(!asked_to_confirm(now + Duration::hours(10), now));
    }

    #[test]
    fn a_host_from_before_plans_hears_their_trial_started() {
        let payload = with_billing_url(Some(&json!({ "name": "Nino" }))).expect("a payload");
        let started = email(TRIAL_STARTED_TEMPLATE, &payload);
        assert_eq!(started.subject, "Your page is on Pro, free for 14 days");
        assert!(started.text.contains("nothing you use changes today"));
        assert!(started.text.contains("/dashboard/billing"));
    }

    #[test]
    fn the_trial_email_lists_what_this_host_loses_when_it_knows() {
        let general = email(TRIAL_ENDING_TEMPLATE, &json!({ "name": "Nino" }));
        assert!(general.text.contains("25 included bookings a month"));
        let filled = with_trial_losses(
            Some(&json!({ "name": "Nino" })),
            vec!["Your own form questions stop being asked.".to_owned()],
        )
        .expect("a payload");
        let personal = email(TRIAL_ENDING_TEMPLATE, &filled);
        assert!(personal.text.contains("What changes for you"));
        // A row queued without a link still gets one.
        assert!(personal.text.contains("/dashboard/billing"));
        assert!(
            personal
                .text
                .contains("Your own form questions stop being asked.")
        );
    }

    #[test]
    fn account_notices_greet_the_host_and_link_to_billing() {
        let payload =
            json!({ "name": HOSTILE, "billing_url": "https://cadence.example/dashboard/billing" });
        for (template, subject) in [
            (TRIAL_ENDING_TEMPLATE, "Your Pro trial ends in 3 days"),
            (
                HOST_BOOKING_LIMIT_TEMPLATE,
                "You've had 25 bookings this month",
            ),
            (PAYMENT_FAILED_TEMPLATE, "Your Cadence payment failed"),
        ] {
            let email = email(template, &payload);
            assert_eq!(email.subject, subject);
            assert!(
                email.html.contains(&format!(">Hi {HOSTILE_ESCAPED},</td>")),
                "{template}"
            );
            assert!(
                email
                    .text
                    .contains("https://cadence.example/dashboard/billing"),
                "{template}"
            );
            assert!(!email.html.contains("<b>"), "{template}");
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

    fn pending(
        template: &str,
        call_start: Option<DateTime<Utc>>,
        status: Option<BookingStatus>,
    ) -> PendingEmail {
        PendingEmail {
            id: uuid::Uuid::nil(),
            email: "ada@example.com".to_owned(),
            template: template.to_owned(),
            payload: Some(invitee_payload("Nino")),
            call_start,
            call_status: status,
            call_confirmed: None,
        }
    }

    fn sends(email: &PendingEmail, now: DateTime<Utc>) -> bool {
        matches!(delivery(email, now), Delivery::Send(_))
    }

    #[test]
    fn the_day_before_asks_whether_the_invitee_is_coming() {
        let mut payload = invitee_payload("Nino");
        payload["attend_url"] = json!("https://cadence.example/booking/abc123?attend=yes");
        let reminder = email(REMINDER_24H_TEMPLATE, &payload);
        assert!(reminder.html.contains("I&#x27;m coming") || reminder.html.contains("I'm coming"));
        assert!(reminder.text.contains("attend=yes"));
        assert!(reminder.text.contains("I need another time"));
        // Only the day-before reminder asks.
        let later = email(REMINDER_2H_TEMPLATE, &payload);
        assert!(!later.text.contains("attend=yes"));
    }

    #[test]
    fn the_unconfirmed_heads_up_waits_on_the_invitee() {
        let start = Utc::now() + Duration::hours(2);
        let mut heads_up = pending(
            HOST_UNCONFIRMED_TEMPLATE,
            Some(start),
            Some(BookingStatus::Confirmed),
        );
        heads_up.payload = Some(host_payload("Ada"));
        assert!(sends(&heads_up, Utc::now()));
        heads_up.call_confirmed = Some(true);
        assert!(!sends(&heads_up, Utc::now()), "the invitee confirmed");
        heads_up.call_confirmed = Some(false);
        assert!(!sends(&heads_up, start), "the call has started");
    }

    #[test]
    fn a_follow_up_and_the_weekly_numbers_render() {
        let follow_up = email(
            LEAD_FOLLOW_UP_TEMPLATE,
            &json!({ "name": "Jo", "host_name": "Nino", "booking_url": "https://cadence.example/book/nino" }),
        );
        assert_eq!(follow_up.subject, "Still want to talk with Nino?");
        assert!(follow_up.text.contains("https://cadence.example/book/nino"));
        let weekly = email(
            WEEKLY_INSIGHTS_TEMPLATE,
            &json!({ "name": "Nino", "lines": ["12 calls booked", "1 no-show"], "dashboard_url": "https://cadence.example/dashboard" }),
        );
        assert_eq!(weekly.subject, "Your week in bookings");
        assert!(weekly.text.contains("12 calls booked"));
    }

    #[test]
    fn a_cancelled_call_only_sends_word_of_the_cancellation() {
        let now = Utc::now();
        let ahead = Some(now + Duration::days(1));
        let cancelled = Some(BookingStatus::Cancelled);
        for template in [
            BOOKING_CONFIRMATION_TEMPLATE,
            REMINDER_24H_TEMPLATE,
            BOOKING_RESCHEDULED_TEMPLATE,
            HOST_NEW_BOOKING_TEMPLATE,
        ] {
            assert!(
                !sends(&pending(template, ahead, cancelled), now),
                "{template}"
            );
        }
        for template in [BOOKING_CANCELLED_TEMPLATE, HOST_BOOKING_CANCELLED_TEMPLATE] {
            assert!(
                sends(&pending(template, ahead, cancelled), now),
                "{template}"
            );
        }
    }

    #[test]
    fn a_started_call_drops_its_reminders_but_not_its_news() {
        let now = Utc::now();
        let started = Some(now - Duration::minutes(1));
        let confirmed = Some(BookingStatus::Confirmed);
        for template in [
            REMINDER_24H_TEMPLATE,
            REMINDER_2H_TEMPLATE,
            REMINDER_30MIN_TEMPLATE,
        ] {
            assert!(
                !sends(&pending(template, started, confirmed), now),
                "{template}"
            );
        }
        for template in [BOOKING_CONFIRMATION_TEMPLATE, HOST_NEW_BOOKING_TEMPLATE] {
            assert!(
                sends(&pending(template, started, confirmed), now),
                "{template}"
            );
        }
        // Still ahead, a reminder goes out.
        let ahead = Some(now + Duration::minutes(29));
        assert!(sends(
            &pending(REMINDER_30MIN_TEMPLATE, ahead, confirmed),
            now
        ));
    }

    #[test]
    fn an_email_about_no_call_is_sent_and_an_unknown_one_is_reported() {
        let now = Utc::now();
        let mut reset = pending(PASSWORD_RESET_TEMPLATE, None, None);
        reset.payload = Some(reset_payload("Ada"));
        assert!(sends(&reset, now));
        assert!(matches!(
            delivery(&pending("newsletter", None, None), now),
            Delivery::UnknownTemplate
        ));
    }

    #[test]
    fn a_field_that_no_longer_reads_is_left_out_not_fatal() {
        let mut payload = invitee_payload("Nino");
        payload["minutes"] = json!("thirty");
        payload["host_name"] = json!(42);
        let rendered = email(BOOKING_CONFIRMATION_TEMPLATE, &payload);
        assert_eq!(rendered.subject, "Your call is booked");
        assert!(rendered.text.contains("09:00 (Europe/Paris)"));
        // Not an object at all: the email still goes out, with what it can say.
        let rendered = email(BOOKING_CONFIRMATION_TEMPLATE, &json!("garbage"));
        assert!(rendered.text.contains("The time you booked"));
    }

    #[test]
    fn a_payload_written_by_the_queue_reads_back_as_written() {
        let written = InviteeEmail {
            name: Some("Ada".to_owned()),
            slot_start: Some(Utc::now()),
            timezone: Some("Europe/Paris".to_owned()),
            meeting_url: None,
            meet_link: None,
            host_name: Some("Nino".to_owned()),
            title: Some("Intro call".to_owned()),
            minutes: Some(30),
            manage_url: Some("https://cadence.example/booking/abc".to_owned()),
            attend_url: None,
        };
        let stored = payload_of(&written).expect("serialises");
        assert!(
            stored.get("meet_link").is_none(),
            "the old name is only read"
        );
        assert_eq!(stored["meeting_url"], Value::Null);
        let read: InviteeEmail = read(Some(&stored));
        assert_eq!(read.name, written.name);
        assert_eq!(read.minutes, Some(30));
        assert_eq!(read.manage_url, written.manage_url);
    }
}
