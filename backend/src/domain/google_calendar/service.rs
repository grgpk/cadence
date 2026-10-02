use chrono::{DateTime as ChronoDateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use time_tz::{OffsetDateTimeExt, PrimitiveDateTimeExt, timezones};
use uuid::Uuid;

use crate::domain::bookings::db as bookings_db;
use crate::domain::google_calendar::client::GoogleCalendarClient;
use crate::domain::google_calendar::db::GoogleCalendarDb;
use crate::domain::google_calendar::environment::Environment;
use crate::domain::google_calendar::models::{
    AvailableSlotsByDate, BookSlotRequest, BookingResult, CalendarEvent, GoogleTimeSlot,
    GoogleTokenResponse,
};
use crate::domain::google_calendar::timezone::{
    REFERENCE_TIMEZONE, SLOT_TIMES_REFERENCE, convert_time_between_tz,
};

const SLOT_DURATION_MINUTES: i64 = 30;
const SLOT_BUFFER_MINUTES: i64 = 15;
const MIN_BOOKING_HOURS_BUFFER: i64 = 4;

pub fn google_oauth_config() -> Result<(String, String), anyhow::Error> {
    Ok((
        required_env(&["GOOGLE_CLIENT_ID", "CLIENT_ID_GOOGLE"])?,
        required_env(&[
            "GOOGLE_REDIRECT_URI",
            "GOOGLE_REDIRECT_URI_V2__PROD",
            "GOOGLE_REDIRECT_URI_V2__LOCAL",
        ])?,
    ))
}

fn google_client_secret() -> Result<String, anyhow::Error> {
    if Environment::detect() == Environment::Local {
        first_env(&["GOOGLE_CLIENT_SECRET", "LOCAL__CLIENT_SECRET_GOOGLE"])
    } else {
        first_env(&["GOOGLE_CLIENT_SECRET", "CLIENT_SECRET_GOOGLE"])
    }
}

fn required_env(names: &[&str]) -> Result<String, anyhow::Error> {
    first_env(names)
}

fn first_env(names: &[&str]) -> Result<String, anyhow::Error> {
    names
        .iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| anyhow::anyhow!("missing Google Calendar environment variable"))
}

pub async fn google_calendar_login(
    pool: &PgPool,
    host_unid: Uuid,
    code: &str,
) -> Result<(), anyhow::Error> {
    let (client_id, redirect_uri) = google_oauth_config()?;
    let client_secret = google_client_secret()?;
    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("token exchange failed: {error}"))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::error!(body = %body, "Google Calendar token exchange failed");
        return Err(anyhow::anyhow!("Google Calendar token exchange failed"));
    }

    let token_data: GoogleTokenResponse = response
        .json()
        .await
        .map_err(|error| anyhow::anyhow!("failed to parse Google token: {error}"))?;
    let refresh_token = token_data
        .refresh_token
        .ok_or_else(|| anyhow::anyhow!("no refresh token received, reconnect Google Calendar"))?;
    let expires_at = Utc::now()
        + chrono::Duration::seconds(
            i64::try_from(token_data.expires_in)
                .map_err(|_| anyhow::anyhow!("invalid Google token expiry"))?,
        );

    GoogleCalendarDb::save(
        pool,
        host_unid,
        &token_data.access_token,
        &refresh_token,
        expires_at,
    )
    .await?;
    Ok(())
}

pub async fn disconnect_google_calendar(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<(), anyhow::Error> {
    GoogleCalendarDb::delete(pool, host_unid).await?;
    Ok(())
}

pub async fn get_google_access_token(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<String, anyhow::Error> {
    let config = GoogleCalendarDb::get(pool, host_unid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Google Calendar not connected"))?;
    if !GoogleCalendarDb::is_expired(config.expires_at) {
        return Ok(config.access_token);
    }

    let (client_id, _) = google_oauth_config()?;
    let client_secret = google_client_secret()?;
    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", config.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("token refresh failed: {error}"))?;

    if !response.status().is_success() {
        tracing::error!(status = %response.status(), "Google access token refresh failed");
        return Err(anyhow::anyhow!(
            "token refresh failed, reconnect Google Calendar"
        ));
    }

    #[derive(serde::Deserialize)]
    struct RefreshResponse {
        access_token: String,
        expires_in: u64,
    }

    let refresh: RefreshResponse = response.json().await?;
    let expires_at = Utc::now()
        + chrono::Duration::seconds(
            i64::try_from(refresh.expires_in)
                .map_err(|_| anyhow::anyhow!("invalid Google token expiry"))?,
        );
    GoogleCalendarDb::update_access_token(pool, host_unid, &refresh.access_token, expires_at)
        .await?;
    Ok(refresh.access_token)
}

pub async fn get_calendar_events(
    pool: &PgPool,
    host_unid: Uuid,
) -> Result<Vec<CalendarEvent>, anyhow::Error> {
    let access_token = get_google_access_token(pool, host_unid).await?;
    GoogleCalendarClient::new(access_token)
        .get_upcoming_events(7)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub async fn get_available_slots(
    pool: &PgPool,
    host_unid: Uuid,
    date: &str,
    timezone: &str,
) -> Result<Vec<GoogleTimeSlot>, anyhow::Error> {
    let parsed_date = parse_date(date)?;
    validate_timezone(timezone)?;
    validate_booking_window(parsed_date, timezone, OffsetDateTime::now_utc())?;
    let access_token = get_google_access_token(pool, host_unid).await?;
    let events = GoogleCalendarClient::new(access_token)
        .get_events_for_day(date, REFERENCE_TIMEZONE)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(build_available_slots(
        &events,
        date,
        parsed_date,
        timezone,
        OffsetDateTime::now_utc(),
    ))
}

pub async fn get_available_slots_batch(
    pool: &PgPool,
    host_unid: Uuid,
    dates: &[String],
    timezone: &str,
) -> Result<Vec<AvailableSlotsByDate>, anyhow::Error> {
    validate_timezone(timezone)?;
    let parsed_dates = validate_batch_dates(dates)?;
    if parsed_dates.is_empty() {
        return Ok(Vec::new());
    }

    let start_date = parsed_dates
        .iter()
        .min_by_key(|(_, date)| *date)
        .map(|(date, _)| date.as_str())
        .unwrap_or_default();
    let end_date = parsed_dates
        .iter()
        .max_by_key(|(_, date)| *date)
        .map(|(date, _)| date.as_str())
        .unwrap_or_default();
    let access_token = get_google_access_token(pool, host_unid).await?;
    let events = GoogleCalendarClient::new(access_token)
        .get_events_between(start_date, end_date, REFERENCE_TIMEZONE)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let now = OffsetDateTime::now_utc();

    Ok(parsed_dates
        .into_iter()
        .map(|(date, parsed_date)| AvailableSlotsByDate {
            slots: build_available_slots(&events, &date, parsed_date, timezone, now),
            date,
        })
        .collect())
}

pub async fn book_slot(
    pool: &PgPool,
    host_unid: Uuid,
    request: BookSlotRequest,
) -> Result<BookingResult, anyhow::Error> {
    validate_booking(&request)?;
    let parsed_date = parse_date(&request.date)?;
    let parsed_time = parse_time(&request.time)?;
    validate_timezone(&request.timezone)?;
    let now = OffsetDateTime::now_utc();
    validate_booking_window(parsed_date, &request.timezone, now)?;
    if is_slot_too_soon(&request.date, &request.time, &request.timezone, now) {
        return Err(anyhow::anyhow!(
            "Cannot book slots within {MIN_BOOKING_HOURS_BUFFER} hours of current time"
        ));
    }

    let reference = convert_time_between_tz(
        &request.time,
        parsed_date,
        &request.timezone,
        REFERENCE_TIMEZONE,
    )
    .ok_or_else(|| anyhow::anyhow!("failed to convert time for availability check"))?;
    let reference_date = match reference.day_offset {
        -1 => parsed_date.previous_day().unwrap_or(parsed_date),
        1 => parsed_date.next_day().unwrap_or(parsed_date),
        _ => parsed_date,
    };
    let reference_date_string = format_date(reference_date);
    let access_token = get_google_access_token(pool, host_unid).await?;
    let client = GoogleCalendarClient::new(access_token);
    let events = client
        .get_events_for_day(&reference_date_string, REFERENCE_TIMEZONE)
        .await
        .map_err(|error| anyhow::anyhow!("failed to check availability: {error}"))?;
    if is_slot_busy(&events, &reference_date_string, &reference.time) {
        return Err(anyhow::anyhow!(
            "This slot is no longer available. Please select another time"
        ));
    }

    let result = client
        .create_event_with_meet(
            &request.date,
            &request.time,
            SLOT_DURATION_MINUTES,
            &request.timezone,
            &format!("Cadence call with {}", request.attendee_name),
            &format!(
                "Booking by {} ({})",
                request.attendee_name, request.attendee_email
            ),
            &request.attendee_email,
            &request.attendee_name,
        )
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    let slot_start = local_datetime_to_utc(parsed_date, parsed_time, &request.timezone)?;
    let mut transaction = pool.begin().await?;
    let booking = match bookings_db::create(
        &mut transaction,
        host_unid,
        Some(request.lead_id),
        None,
        slot_start,
        &request.timezone,
        &request.attendee_name,
        &request.attendee_email,
        Some(&result.event_id),
        result.meet_link.as_deref(),
    )
    .await
    {
        Ok(value) => value,
        Err(error) if is_unique_violation(&error) => {
            return Err(anyhow::anyhow!("slot already booked"));
        }
        Err(error) => return Err(error.into()),
    };

    let payload = json!({
        "name": booking.invitee_name,
        "slot_start": booking.slot_start,
        "timezone": booking.timezone,
        "meet_link": result.meet_link,
    });
    bookings_db::queue_email(
        &mut transaction,
        booking.unid,
        &booking.invitee_email,
        "booking_confirmation",
        Utc::now(),
        payload.clone(),
    )
    .await?;
    for (template, delay) in [
        ("reminder_24h", chrono::Duration::hours(-24)),
        ("reminder_2h", chrono::Duration::hours(-2)),
        ("reminder_30min", chrono::Duration::minutes(-30)),
    ] {
        let send_at = booking.slot_start + delay;
        if send_at > Utc::now() {
            bookings_db::queue_email(
                &mut transaction,
                booking.unid,
                &booking.invitee_email,
                template,
                send_at,
                payload.clone(),
            )
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(result)
}

fn validate_booking(request: &BookSlotRequest) -> Result<(), anyhow::Error> {
    if request.attendee_name.trim().is_empty()
        || !request.attendee_email.contains('@')
        || request.timezone.trim().is_empty()
    {
        return Err(anyhow::anyhow!("booking data is invalid"));
    }
    Ok(())
}

fn validate_timezone(timezone: &str) -> Result<(), anyhow::Error> {
    if timezones::get_by_name(timezone).is_none() {
        return Err(anyhow::anyhow!("invalid timezone"));
    }
    Ok(())
}

fn validate_booking_window(
    date: Date,
    timezone: &str,
    now: OffsetDateTime,
) -> Result<(), anyhow::Error> {
    let tz = timezones::get_by_name(timezone).ok_or_else(|| anyhow::anyhow!("invalid timezone"))?;
    let today = now.to_timezone(tz).date();
    let window_end = today + Duration::days(7);
    if date < today || date >= window_end {
        return Err(anyhow::anyhow!("date is outside the 7-day booking window"));
    }
    Ok(())
}

fn parse_date(value: &str) -> Result<Date, anyhow::Error> {
    Date::parse(value, format_description!("[year]-[month]-[day]"))
        .map_err(|error| anyhow::anyhow!("invalid date: {error}"))
}

fn parse_time(value: &str) -> Result<Time, anyhow::Error> {
    Time::parse(value, format_description!("[hour]:[minute]"))
        .map_err(|error| anyhow::anyhow!("invalid time: {error}"))
}

fn format_date(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

fn build_available_slots(
    events: &[CalendarEvent],
    date: &str,
    parsed_date: Date,
    timezone: &str,
    now: OffsetDateTime,
) -> Vec<GoogleTimeSlot> {
    SLOT_TIMES_REFERENCE
        .iter()
        .filter_map(|slot_time| {
            let converted =
                convert_time_between_tz(slot_time, parsed_date, REFERENCE_TIMEZONE, timezone)?;
            let user_date = match converted.day_offset {
                -1 => parsed_date.previous_day().unwrap_or(parsed_date),
                1 => parsed_date.next_day().unwrap_or(parsed_date),
                _ => parsed_date,
            };
            let user_date_string = format_date(user_date);
            Some(GoogleTimeSlot {
                time: converted.time.clone(),
                available: !is_slot_busy(events, date, slot_time)
                    && !is_slot_too_soon(&user_date_string, &converted.time, timezone, now),
                day_offset: converted.day_offset,
            })
        })
        .collect()
}

fn validate_batch_dates(dates: &[String]) -> Result<Vec<(String, Date)>, anyhow::Error> {
    if dates.len() > 14 {
        return Err(anyhow::anyhow!("at most 14 dates can be requested"));
    }
    dates
        .iter()
        .map(|date| Ok((date.clone(), parse_date(date)?)))
        .collect()
}

fn local_datetime_to_utc(
    date: Date,
    time: Time,
    timezone: &str,
) -> Result<ChronoDateTime<Utc>, anyhow::Error> {
    let tz = timezones::get_by_name(timezone).ok_or_else(|| anyhow::anyhow!("invalid timezone"))?;
    let local = PrimitiveDateTime::new(date, time)
        .assume_timezone(tz)
        .take_first()
        .ok_or_else(|| anyhow::anyhow!("invalid local time"))?;
    let utc = local.to_offset(UtcOffset::UTC);
    ChronoDateTime::<Utc>::from_timestamp(utc.unix_timestamp(), utc.nanosecond())
        .ok_or_else(|| anyhow::anyhow!("invalid UTC booking time"))
}

fn is_slot_too_soon(slot_date: &str, slot_time: &str, timezone: &str, now: OffsetDateTime) -> bool {
    let Ok(date) = parse_date(slot_date) else {
        return true;
    };
    let Ok(time) = parse_time(slot_time) else {
        return true;
    };
    let Some(tz) = timezones::get_by_name(timezone) else {
        return true;
    };
    let local = PrimitiveDateTime::new(date, time);
    let Some(slot) = local.assume_timezone(tz).take_first() else {
        return true;
    };
    slot.to_offset(UtcOffset::UTC) < now + Duration::hours(MIN_BOOKING_HOURS_BUFFER)
}

fn is_slot_busy(events: &[CalendarEvent], date: &str, slot_time: &str) -> bool {
    let slot_start = format!("{date}T{slot_time}:00");
    let slot_end_buffered = add_minutes(&slot_start, SLOT_DURATION_MINUTES + SLOT_BUFFER_MINUTES);
    let empty = String::new();

    for event in events {
        let event_start = event
            .start
            .as_ref()
            .and_then(|value| value.date_time.as_ref().or(value.date.as_ref()))
            .unwrap_or(&empty);
        let event_end = event
            .end
            .as_ref()
            .and_then(|value| value.date_time.as_ref().or(value.date.as_ref()))
            .unwrap_or(&empty);
        if !event_start.contains('T') {
            if date >= event_start.as_str() && date < event_end.as_str() {
                return true;
            }
            continue;
        }
        let normalized_start = normalize_datetime(event_start);
        let normalized_end = add_minutes(&normalize_datetime(event_end), SLOT_BUFFER_MINUTES);
        if slot_start < normalized_end && slot_end_buffered > normalized_start {
            return true;
        }
    }
    false
}

fn add_minutes(datetime: &str, minutes: i64) -> String {
    let Some((date, time)) = datetime.split_once('T') else {
        return datetime.to_owned();
    };
    let parts = time.split(':').collect::<Vec<_>>();
    let hour = parts
        .first()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let minute = parts
        .get(1)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let total = hour * 60 + minute + minutes;
    if total >= 24 * 60 {
        let Ok(parsed) = parse_date(date) else {
            return datetime.to_owned();
        };
        let next = parsed.next_day().unwrap_or(parsed);
        return format!(
            "{}T{:02}:{:02}:00",
            format_date(next),
            (total / 60) % 24,
            total % 60
        );
    }
    format!("{date}T{:02}:{:02}:00", total / 60, total % 60)
}

fn normalize_datetime(datetime: &str) -> String {
    let Some((date, time)) = datetime.split_once('T') else {
        return datetime.to_owned();
    };
    let time = time
        .split_once('+')
        .map_or(time, |(value, _)| value)
        .rsplit_once('-')
        .filter(|(_, offset)| offset.len() == 2 || offset.len() == 4)
        .map_or(time, |(value, _)| value)
        .trim_end_matches('Z');
    format!("{date}T{time}")
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}
