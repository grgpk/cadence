use std::collections::HashSet;

use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use sqlx::{PgConnection, PgPool};

use super::{
    client::{Calendly, CalendlyError},
    models::{
        ApiEventType, ApiRule, CalendlyAccount, CalendlyImport, CalendlyImported, CalendlyPreview,
        ImportedDate, ImportedEventType, ImportedHours, ImportedRange, SkippedEventType,
    },
};
use crate::{
    domain::{
        auth::models::UserRecord,
        availability::{
            models::{AvailabilityRuleInput, HoursRange},
            service as availability,
        },
        billing::service as billing,
        event_types::{models::EventTypeInput, service as event_types},
        host_settings::service as host_settings,
    },
    error::{AppError, AppResult},
};

const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const SLUG_MIN: usize = 2;
const SLUG_MAX: usize = 40;
const TITLE_MAX: usize = 80;
const DESCRIPTION_MAX: usize = 300;
const DURATION_MINUTES: (i32, i32) = (5, 240);
/// Hours are cut into slots this long when no imported call gives a length.
const DEFAULT_SLOT_MINUTES: i32 = 30;
/// Dates further ahead than this cannot have their own hours.
const DATES_AHEAD_DAYS: i64 = 366;
/// One date override per day of that year, at most.
const DATES_MAX: usize = 366;

/// Reads the Calendly account behind `token` and shows what would come in.
pub async fn preview(
    calendly: &Calendly,
    pool: &PgPool,
    user: &UserRecord,
    token: &str,
) -> AppResult<CalendlyPreview> {
    let account = calendly
        .account(token.trim())
        .await
        .map_err(|error| match error {
            CalendlyError::Unauthorized => AppError::BadRequest(
                "Calendly did not accept that token. Check it has read access to users, event types and availability.".to_owned(),
            ),
            CalendlyError::Request(_) => {
                AppError::Unavailable("Calendly did not answer. Try again in a minute.".to_owned())
            }
        })?;
    let entitlements = billing::entitlements_for(pool, user.unid, Utc::now()).await?;
    Ok(preview_of(
        &account,
        Utc::now().date_naive(),
        entitlements.max_event_types,
    ))
}

/// The account as Cadence would save it. Pure, so the mapping is tested alone.
pub fn preview_of(
    account: &CalendlyAccount,
    today: NaiveDate,
    types_allowed: Option<u32>,
) -> CalendlyPreview {
    let mut slugs = HashSet::new();
    let mut event_types = Vec::new();
    let mut skipped = Vec::new();
    for kind in &account.event_types {
        match imported_type(kind, &mut slugs) {
            Ok(imported) => event_types.push(imported),
            Err(reason) => skipped.push(SkippedEventType {
                name: kind.name.clone().unwrap_or_else(|| "A call".to_owned()),
                reason: reason.to_owned(),
            }),
        }
    }
    let rules = account
        .schedule
        .as_ref()
        .map(|schedule| schedule.rules.as_slice())
        .unwrap_or_default();
    CalendlyPreview {
        timezone: account
            .schedule
            .as_ref()
            .map(|schedule| schedule.timezone.clone())
            .or_else(|| account.user.timezone.clone()),
        event_types,
        skipped,
        weekly: rules.iter().flat_map(weekly_hours).collect(),
        dates: rules
            .iter()
            .filter_map(|rule| own_date(rule, today))
            .collect(),
        types_allowed,
    }
}

fn imported_type(
    kind: &ApiEventType,
    slugs: &mut HashSet<String>,
) -> Result<ImportedEventType, &'static str> {
    if kind.kind != "solo" || kind.pooling_type.is_some() {
        return Err("Calls with several hosts are not brought in.");
    }
    if !kind.active {
        return Err("It is switched off in Calendly.");
    }
    let title: String = kind
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Call")
        .chars()
        .take(TITLE_MAX)
        .collect();
    let base = kind
        .slug
        .as_deref()
        .and_then(link_slug)
        .or_else(|| link_slug(&title))
        .unwrap_or_else(|| "call".to_owned());
    let slug = unique_slug(&base, slugs);
    Ok(ImportedEventType {
        slug,
        title,
        description: kind
            .description_plain
            .as_deref()
            .unwrap_or_default()
            .trim()
            .chars()
            .take(DESCRIPTION_MAX)
            .collect(),
        duration_minutes: kind.duration.clamp(DURATION_MINUTES.0, DURATION_MINUTES.1),
    })
}

/// Lowercase words joined by single dashes, 2 to 40 characters.
fn link_slug(text: &str) -> Option<String> {
    let mut slug = String::new();
    for character in text.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.chars().take(SLUG_MAX).collect();
    let slug = slug.trim_end_matches('-').to_owned();
    (slug.len() >= SLUG_MIN).then_some(slug)
}

/// `base`, or `base-2`, `base-3`… when it is already used.
fn unique_slug(base: &str, used: &mut HashSet<String>) -> String {
    let mut slug = base.to_owned();
    let mut number = 2;
    while used.contains(&slug) {
        let suffix = format!("-{number}");
        let stem: String = base.chars().take(SLUG_MAX - suffix.len()).collect();
        slug = format!("{}{suffix}", stem.trim_end_matches('-'));
        number += 1;
    }
    used.insert(slug.clone());
    slug
}

fn weekly_hours(rule: &ApiRule) -> Vec<ImportedHours> {
    let Some(weekday) = rule
        .wday
        .as_deref()
        .filter(|_| rule.kind == "wday")
        .and_then(|day| WEEKDAYS.iter().position(|name| *name == day))
    else {
        return Vec::new();
    };
    ranges(rule)
        .into_iter()
        .map(|range| ImportedHours {
            weekday: i32::try_from(weekday).unwrap_or_default(),
            start_time: range.start_time,
            end_time: range.end_time,
        })
        .collect()
}

fn own_date(rule: &ApiRule, today: NaiveDate) -> Option<ImportedDate> {
    let date = rule.date.filter(|_| rule.kind == "date")?;
    let in_range = date >= today && date <= today + Duration::days(DATES_AHEAD_DAYS);
    in_range.then(|| ImportedDate {
        date,
        ranges: ranges(rule),
    })
}

fn ranges(rule: &ApiRule) -> Vec<ImportedRange> {
    rule.intervals
        .iter()
        .filter_map(|interval| {
            let start_time = clock(&interval.from)?;
            let end_time = clock(&interval.to)?;
            (start_time < end_time).then_some(ImportedRange {
                start_time,
                end_time,
            })
        })
        .collect()
}

/// "08:30" as a time. Calendly's end of day, "24:00", is the last minute.
fn clock(text: &str) -> Option<NaiveTime> {
    if text == "24:00" {
        return NaiveTime::from_hms_opt(23, 59, 0);
    }
    NaiveTime::parse_from_str(text, "%H:%M").ok()
}

/// Saves the reviewed import.
///
/// The timezone, the weekly hours (replacing the old ones), the dates with
/// their own hours and the kinds of call. The first kind of call fills the
/// host's first one, so its link keeps working.
pub async fn apply(
    pool: &PgPool,
    user: &UserRecord,
    import: CalendlyImport,
) -> AppResult<CalendlyImported> {
    if let Some(timezone) = &import.timezone
        && !host_settings::is_known_timezone(timezone)
    {
        return Err(AppError::BadRequest("unknown timezone".to_owned()));
    }
    if import.dates.len() > DATES_MAX {
        return Err(AppError::BadRequest(format!(
            "an import brings at most {DATES_MAX} dates"
        )));
    }
    let slot_minutes = import
        .event_types
        .first()
        .map_or(DEFAULT_SLOT_MINUTES, |kind| {
            kind.duration_minutes
                .clamp(DURATION_MINUTES.0, DURATION_MINUTES.1)
        });

    // All of it or none: a failure halfway leaves the account as it was.
    let mut transaction = pool.begin().await?;
    if let Some(timezone) = import.timezone {
        let mut settings = host_settings::get_or_create(pool, user).await?;
        settings.timezone = timezone;
        host_settings::update_in(pool, &mut transaction, user.unid, settings).await?;
    }
    let weekly_hours = u32::try_from(import.weekly.len()).unwrap_or(u32::MAX);
    if !import.weekly.is_empty() {
        let rules = import
            .weekly
            .into_iter()
            .map(|hours| AvailabilityRuleInput {
                weekday: hours.weekday,
                start_time: hours.start_time,
                end_time: hours.end_time,
                slot_minutes,
            })
            .collect();
        availability::replace_weekly_in(&mut transaction, user.unid, rules).await?;
    }
    let dates = u32::try_from(import.dates.len()).unwrap_or(u32::MAX);
    for date in import.dates {
        let ranges = date
            .ranges
            .into_iter()
            .map(|range| HoursRange {
                start_time: range.start_time,
                end_time: range.end_time,
                slot_minutes,
            })
            .collect();
        availability::save_override_in(&mut transaction, user.unid, date.date, ranges).await?;
    }
    let (event_types, event_types_left_out) =
        save_types(pool, &mut transaction, user, import.event_types).await?;
    transaction.commit().await?;
    Ok(CalendlyImported {
        event_types,
        event_types_left_out,
        weekly_hours,
        dates,
    })
}

/// The first imported kind of call fills the host's first one; the rest are
/// added while the plan allows. Returns how many were saved and left out.
async fn save_types(
    pool: &PgPool,
    connection: &mut PgConnection,
    user: &UserRecord,
    imported: Vec<ImportedEventType>,
) -> AppResult<(u32, u32)> {
    let existing = event_types::list_in(pool, connection, user.unid).await?;
    let Some(first) = existing.first() else {
        return Ok((0, 0));
    };
    let mut used: HashSet<String> = existing.iter().map(|kind| kind.slug.clone()).collect();
    let mut saved = 0;
    let mut left_out = 0;
    for (index, kind) in imported.into_iter().enumerate() {
        let is_first = index == 0;
        let input = EventTypeInput {
            // The first keeps its link; the others get theirs, made unique.
            slug: if is_first {
                first.slug.clone()
            } else {
                unique_slug(&kind.slug, &mut used)
            },
            title: kind.title,
            description: kind.description,
            duration_minutes: Some(kind.duration_minutes),
            buffer_minutes: first.buffer_minutes,
            minimum_notice_hours: first.minimum_notice_hours,
            booking_window_days: first.booking_window_days,
            questions: if is_first {
                first.questions.clone()
            } else {
                Vec::new()
            },
            is_active: true,
        };
        let result = if is_first {
            event_types::update_in(pool, connection, user.unid, first.unid, input).await
        } else {
            event_types::create_in(pool, connection, user.unid, input).await
        };
        match result {
            Ok(_) => saved += 1,
            Err(AppError::PlanRequired(_)) => left_out += 1,
            Err(error) => return Err(error),
        }
    }
    Ok((saved, left_out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendly::models::{ApiInterval, ApiSchedule, ApiUser};

    fn kind(name: &str, slug: Option<&str>, minutes: i32) -> ApiEventType {
        ApiEventType {
            name: Some(name.to_owned()),
            slug: slug.map(str::to_owned),
            duration: minutes,
            description_plain: Some("  A first talk.  ".to_owned()),
            active: true,
            kind: "solo".to_owned(),
            pooling_type: None,
        }
    }

    fn rule(
        kind: &str,
        wday: Option<&str>,
        date: Option<NaiveDate>,
        hours: &[(&str, &str)],
    ) -> ApiRule {
        ApiRule {
            kind: kind.to_owned(),
            wday: wday.map(str::to_owned),
            date,
            intervals: hours
                .iter()
                .map(|(from, to)| ApiInterval {
                    from: (*from).to_owned(),
                    to: (*to).to_owned(),
                })
                .collect(),
        }
    }

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("a date")
    }

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("a time")
    }

    fn account() -> CalendlyAccount {
        let mut team = kind("Team sync", Some("team"), 30);
        team.pooling_type = Some("round_robin".to_owned());
        CalendlyAccount {
            user: ApiUser {
                uri: "https://api.calendly.com/users/ME".to_owned(),
                timezone: Some("America/New_York".to_owned()),
            },
            event_types: vec![
                kind("Intro call", Some("intro"), 15),
                kind("Intro call", Some("intro"), 600),
                kind("Strategy & Co!", None, 2),
                team,
            ],
            schedule: Some(ApiSchedule {
                default: true,
                timezone: "Europe/Berlin".to_owned(),
                rules: vec![
                    rule(
                        "wday",
                        Some("monday"),
                        None,
                        &[("09:00", "12:00"), ("13:00", "24:00")],
                    ),
                    rule("wday", Some("sunday"), None, &[]),
                    rule("wday", Some("friday"), None, &[("17:00", "09:00")]),
                    rule("date", None, Some(day("2026-12-24")), &[]),
                    rule("date", None, Some(day("2020-01-01")), &[("09:00", "10:00")]),
                ],
            }),
        }
    }

    #[test]
    fn calendly_types_become_kinds_of_call() {
        let preview = preview_of(&account(), day("2026-10-17"), Some(1));
        let slugs: Vec<&str> = preview
            .event_types
            .iter()
            .map(|kind| kind.slug.as_str())
            .collect();
        assert_eq!(slugs, ["intro", "intro-2", "strategy-co"]);
        let minutes: Vec<i32> = preview
            .event_types
            .iter()
            .map(|kind| kind.duration_minutes)
            .collect();
        assert_eq!(minutes, [15, 240, 5], "lengths are kept within 5 to 240");
        assert_eq!(preview.event_types[0].description, "A first talk.");
        assert_eq!(preview.skipped.len(), 1, "the team call stays behind");
        assert_eq!(preview.types_allowed, Some(1));
    }

    #[test]
    fn the_schedule_becomes_hours_and_dates() {
        let preview = preview_of(&account(), day("2026-10-17"), None);
        assert_eq!(preview.timezone.as_deref(), Some("Europe/Berlin"));
        assert_eq!(
            preview.weekly,
            [
                ImportedHours {
                    weekday: 0,
                    start_time: time(9, 0),
                    end_time: time(12, 0),
                },
                ImportedHours {
                    weekday: 0,
                    start_time: time(13, 0),
                    end_time: time(23, 59),
                },
            ],
            "an empty Sunday and a backwards Friday add nothing"
        );
        assert_eq!(
            preview.dates,
            [ImportedDate {
                date: day("2026-12-24"),
                ranges: vec![],
            }],
            "a past date is left out; a date with no hours is a day off"
        );
    }

    #[test]
    fn slugs_are_link_safe() {
        assert_eq!(
            link_slug("  Deep Dive -- 60 min "),
            Some("deep-dive-60-min".to_owned())
        );
        assert_eq!(link_slug("!"), None);
        let mut used = HashSet::from(["call".to_owned(), "call-2".to_owned()]);
        assert_eq!(unique_slug("call", &mut used), "call-3");
    }
}
