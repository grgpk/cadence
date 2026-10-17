use chrono::Utc;
use reqwest::Url;
use sqlx::{PgConnection, PgPool};
use time_tz::timezones;
use uuid::Uuid;

use super::{
    db,
    models::{
        AdminHost, BookingPage, ConferencingApp, ConferencingUpdate, DEFAULT_ACCENT, HostPage,
        HostSettings,
    },
    preview,
};
use crate::{
    domain::{
        auth::models::UserRecord,
        billing::{plans::Entitlements, service as billing},
        emails::service as emails,
        event_types::service as event_types,
    },
    error::{AppError, AppResult},
    validation::is_link_slug,
};

const SLUG_MIN: usize = 3;
const SLUG_MAX: usize = 40;
const MEETING_LINK_MAX: usize = 500;
const SLUG_ATTEMPTS: usize = 5;
const FALLBACK_SLUG: &str = "host";
/// Hours saved before a host picked a timezone were entered as UTC.
const DEFAULT_TIMEZONE: &str = "UTC";

/// Returns the host's settings, creating the row (their link and their first
/// kind of call) on first use.
pub async fn get_or_create(pool: &PgPool, user: &UserRecord) -> AppResult<HostSettings> {
    let settings = find_or_insert(pool, user).await?;
    event_types::ensure_first(pool, user.unid).await?;
    Ok(settings)
}

async fn find_or_insert(pool: &PgPool, user: &UserRecord) -> AppResult<HostSettings> {
    if let Some(settings) = db::find(pool, user.unid).await? {
        return Ok(settings);
    }
    let base = base_slug(user);
    for attempt in 0..SLUG_ATTEMPTS {
        let slug = if attempt == 0 {
            base.clone()
        } else {
            with_suffix(&base)
        };
        match db::insert(pool, user.unid, &slug, DEFAULT_TIMEZONE).await {
            Ok(Some(settings)) => return Ok(settings),
            // Another request created the row first.
            Ok(None) => break,
            Err(error) if is_unique_violation(&error) => {}
            Err(error) => return Err(error.into()),
        }
    }
    db::find(pool, user.unid).await?.ok_or_else(|| {
        AppError::Conflict("a booking link could not be created, try again".to_owned())
    })
}

pub async fn update(
    pool: &PgPool,
    host_unid: Uuid,
    settings: HostSettings,
) -> AppResult<HostSettings> {
    update_in(pool, &mut *pool.acquire().await?, host_unid, settings).await
}

/// Saves the settings through `connection`, as part of a larger change.
pub async fn update_in(
    pool: &PgPool,
    connection: &mut PgConnection,
    host_unid: Uuid,
    mut settings: HostSettings,
) -> AppResult<HostSettings> {
    settings.public_slug = settings.public_slug.trim().to_lowercase();
    settings.accent_color = settings.accent_color.trim().to_lowercase();
    validate(&settings)?;
    // A colour already saved stays saved on any plan; picking a new one is Pro.
    let saved = db::find(&mut *connection, host_unid)
        .await?
        .ok_or(AppError::NotFound)?;
    if settings.accent_color != saved.accent_color {
        let entitlements = billing::entitlements_for(pool, host_unid, Utc::now()).await?;
        if !entitlements.custom_accent {
            return Err(AppError::PlanRequired("your accent colour"));
        }
    }
    match db::update(&mut *connection, host_unid, &settings).await {
        Ok(Some(saved)) => Ok(saved),
        Ok(None) => Err(AppError::NotFound),
        Err(error) if is_unique_violation(&error) => {
            Err(AppError::Conflict("link is already taken".to_owned()))
        }
        Err(error) => Err(error.into()),
    }
}

/// The host's IANA timezone; UTC before they picked one.
pub async fn timezone(pool: &PgPool, host_unid: Uuid) -> AppResult<String> {
    Ok(db::find(pool, host_unid)
        .await?
        .map_or_else(|| DEFAULT_TIMEZONE.to_owned(), |settings| settings.timezone))
}

/// Picks where the host's calls happen.
///
/// A custom link needs the link. Google Meet may be picked before a calendar
/// is connected: until then, and whenever Google makes no Meet, calls use the
/// host's own link.
pub async fn set_conferencing(
    pool: &PgPool,
    user: &UserRecord,
    update: ConferencingUpdate,
) -> AppResult<HostSettings> {
    let meeting_link = clean_meeting_link(update.meeting_link)?;
    if update.app == ConferencingApp::CustomLink && meeting_link.is_none() {
        return Err(AppError::BadRequest(
            "a custom link needs a meeting link".to_owned(),
        ));
    }
    // The row exists once the host has opened their settings; make sure of it.
    get_or_create(pool, user).await?;
    db::set_conferencing(pool, user.unid, update.app, meeting_link.as_deref())
        .await?
        .ok_or(AppError::NotFound)
}

/// A host's public link: who they are and the kinds of call they take now.
/// Types past the host's plan are left out; unknown and banned hosts look
/// the same.
pub async fn host_page(pool: &PgPool, slug: &str) -> AppResult<HostPage> {
    let host = db::find_host(pool, &slug.to_lowercase())
        .await?
        .ok_or(AppError::NotFound)?;
    let entitlements = billing::entitlements_for(pool, host.host_unid, Utc::now()).await?;
    let mut types = db::active_types(pool, host.host_unid).await?;
    types.truncate(types_allowed(&entitlements));
    Ok(HostPage {
        host_name: host.host_name,
        accepting_bookings: host.is_active,
        show_branding: !entitlements.hide_branding,
        calendar_connected: host.calendar_connected,
        accent_color: shown_accent(&entitlements, Some(host.accent_color)),
        types,
    })
}

/// The picture a shared link shows, as PNG. A paused page has none.
pub async fn preview_png(pool: &PgPool, slug: &str) -> AppResult<Vec<u8>> {
    let page = host_page(pool, slug).await?;
    if !page.accepting_bookings {
        return Err(AppError::NotFound);
    }
    let site = emails::app_url();
    let site = site
        .strip_prefix("https://")
        .or_else(|| site.strip_prefix("http://"))
        .unwrap_or(&site);
    let card = preview::PreviewCard::of(&page, format!("{site}/book/{}", slug.to_lowercase()));
    // Drawing takes a few milliseconds of CPU: off the async threads.
    tokio::task::spawn_blocking(move || preview::render(&card))
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("link preview did not render")))
}

/// Resolves a public link to one kind of call, for reading. A paused page or
/// type, or one past the host's plan, is shown with `accepting_bookings` off.
pub async fn display_page(pool: &PgPool, slug: &str, type_slug: &str) -> AppResult<BookingPage> {
    let page = db::find_type_page(pool, &slug.to_lowercase(), &type_slug.to_lowercase())
        .await?
        .ok_or(AppError::NotFound)?;
    with_plan(pool, page, true).await
}

/// Resolves a public link to a kind of call that can be booked right now. A
/// paused one looks unknown here. A plan's included bookings never close it.
pub async fn bookable_page(pool: &PgPool, slug: &str, type_slug: &str) -> AppResult<BookingPage> {
    let page = display_page(pool, slug, type_slug).await?;
    if page.accepting_bookings {
        Ok(page)
    } else {
        Err(AppError::NotFound)
    }
}

/// The page an existing call was booked through, paused or not.
pub async fn page_of_booking(pool: &PgPool, booking_unid: Uuid) -> AppResult<BookingPage> {
    let page = db::page_of_booking(pool, booking_unid)
        .await?
        .ok_or(AppError::NotFound)?;
    with_plan(pool, page, false).await
}

/// How many kinds of call the plan books, oldest first.
fn types_allowed(entitlements: &Entitlements) -> usize {
    entitlements.max_event_types.map_or(usize::MAX, |limit| {
        usize::try_from(limit).unwrap_or(usize::MAX)
    })
}

/// The page as the host's plan lets it run.
///
/// Nothing is cleared: questions a plan leaves out are kept, only not asked,
/// and come back with an upgrade. With `new_calls`, a type past the plan
/// takes none; its booked calls still move.
async fn with_plan(
    pool: &PgPool,
    mut page: BookingPage,
    new_calls: bool,
) -> AppResult<BookingPage> {
    let entitlements = billing::entitlements_for(pool, page.host_unid, Utc::now()).await?;
    if !entitlements.custom_questions {
        page.ask_currently_working_on = false;
        page.ask_how_heard_about_us = false;
        page.ask_what_stopping_you = false;
        page.ask_investment_comfort = false;
        page.ask_urgency_level = false;
    }
    if new_calls
        && usize::try_from(page.type_rank).unwrap_or(usize::MAX) >= types_allowed(&entitlements)
    {
        page.accepting_bookings = false;
    }
    page.show_branding = !entitlements.hide_branding;
    page.follow_up_notice = entitlements.win_back;
    page.accent_color = shown_accent(&entitlements, page.accent_color.take());
    Ok(page)
}

/// The host's colour on their page: only on a plan that has it, and only
/// when it is not the default.
fn shown_accent(entitlements: &Entitlements, color: Option<String>) -> Option<String> {
    color.filter(|color| entitlements.custom_accent && color != DEFAULT_ACCENT)
}

/// `#rrggbb`, as the colour picker writes it.
fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|digit| digit.is_ascii_hexdigit())
}

pub async fn list_hosts(pool: &PgPool) -> AppResult<Vec<AdminHost>> {
    db::list_hosts(pool).await.map_err(Into::into)
}

fn validate(settings: &HostSettings) -> AppResult<()> {
    if !is_valid_slug(&settings.public_slug) {
        return Err(AppError::BadRequest(
            "link must be 3 to 40 lowercase letters, numbers or single dashes".to_owned(),
        ));
    }
    if !is_known_timezone(&settings.timezone) {
        return Err(AppError::BadRequest("timezone is invalid".to_owned()));
    }
    if !is_hex_color(&settings.accent_color) {
        return Err(AppError::BadRequest(
            "accent colour must look like #7c3aed".to_owned(),
        ));
    }
    Ok(())
}

/// Whether `name` is an IANA timezone this server can compute with.
pub fn is_known_timezone(name: &str) -> bool {
    timezones::get_by_name(name).is_some()
}

/// Trims the link; an empty one is no link.
fn clean_meeting_link(link: Option<String>) -> AppResult<Option<String>> {
    let link = link
        .map(|link| link.trim().to_owned())
        .filter(|link| !link.is_empty());
    match link {
        Some(link) if !is_valid_meeting_link(&link) => Err(AppError::BadRequest(
            "meeting link must be an https URL".to_owned(),
        )),
        link => Ok(link),
    }
}

fn is_valid_meeting_link(link: &str) -> bool {
    link.len() <= MEETING_LINK_MAX
        && Url::parse(link).is_ok_and(|url| url.scheme() == "https" && url.has_host())
}

fn is_valid_slug(slug: &str) -> bool {
    is_link_slug(slug, SLUG_MIN, SLUG_MAX)
}

/// Lowercase ASCII letters and digits, every other run of characters becomes one dash.
fn slugify(value: &str) -> String {
    let mut slug = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.truncate(SLUG_MAX);
    slug.trim_end_matches('-').to_owned()
}

fn base_slug(user: &UserRecord) -> String {
    let email_name = user.email.split('@').next().unwrap_or_default();
    [user.full_name.as_str(), email_name]
        .into_iter()
        .map(slugify)
        .find(|slug| slug.len() >= SLUG_MIN)
        .unwrap_or_else(|| FALLBACK_SLUG.to_owned())
}

fn with_suffix(base: &str) -> String {
    let random = Uuid::new_v4().simple().to_string();
    let base = &base[..base.len().min(SLUG_MAX - 5)];
    format!("{}-{}", base.trim_end_matches('-'), &random[..4])
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(full_name: &str, email: &str) -> UserRecord {
        UserRecord {
            unid: Uuid::nil(),
            email: email.to_owned(),
            password_hash: String::new(),
            full_name: full_name.to_owned(),
            status: crate::domain::auth::models::UserStatus::Active,
            session_version: 0,
            roles: Vec::new(),
        }
    }

    #[test]
    fn slugify_keeps_letters_and_collapses_the_rest() {
        assert_eq!(slugify("  Ada  Lovelace! "), "ada-lovelace");
        assert_eq!(slugify("O'Brien & Sons"), "o-brien-sons");
        assert_eq!(slugify("გიორგი"), "");
    }

    #[test]
    fn base_slug_falls_back_to_email_then_default() {
        assert_eq!(base_slug(&user("გიორგი", "giorgi@example.com")), "giorgi");
        assert_eq!(base_slug(&user("გ", "x@example.com")), FALLBACK_SLUG);
    }

    #[test]
    fn generated_slugs_are_valid() {
        let long = "a".repeat(80);
        assert!(is_valid_slug(&slugify(&long)));
        assert!(is_valid_slug(&with_suffix(&slugify(&long))));
        assert!(is_valid_slug(&with_suffix("ada")));
    }

    #[test]
    fn meeting_link_must_be_an_https_url() {
        assert!(is_valid_meeting_link("https://zoom.us/j/123456789"));
        for bad in [
            "http://zoom.us/j/1",
            "zoom.us/j/1",
            "javascript:alert(1)",
            "https://",
        ] {
            assert!(!is_valid_meeting_link(bad), "{bad}");
        }
        let long = format!("https://example.com/{}", "a".repeat(MEETING_LINK_MAX));
        assert!(!is_valid_meeting_link(&long));
    }

    #[test]
    fn meeting_link_is_trimmed_and_empty_means_none() {
        assert_eq!(clean_meeting_link(Some("  ".to_owned())).unwrap(), None);
        assert_eq!(
            clean_meeting_link(Some(" https://zoom.us/j/1 ".to_owned())).unwrap(),
            Some("https://zoom.us/j/1".to_owned())
        );
        assert!(clean_meeting_link(Some("http://zoom.us/j/1".to_owned())).is_err());
    }

    #[test]
    fn conferencing_app_matches_its_column_values() {
        assert_eq!(ConferencingApp::GoogleMeet.as_str(), "google_meet");
        assert_eq!(ConferencingApp::CustomLink.as_str(), "custom_link");
        let json = serde_json::to_string(&ConferencingApp::CustomLink).unwrap();
        assert_eq!(json, "\"custom_link\"");
    }

    #[test]
    fn slug_rule_rejects_bad_shapes() {
        for bad in [
            "ab",
            "Ada",
            "ada--l",
            "-ada",
            "ada-",
            "ada lovelace",
            "ada/l",
        ] {
            assert!(!is_valid_slug(bad), "{bad}");
        }
        assert!(is_valid_slug("ada-lovelace-2"));
    }
}
