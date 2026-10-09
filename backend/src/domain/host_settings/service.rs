use reqwest::Url;
use sqlx::PgPool;
use time_tz::timezones;
use uuid::Uuid;

use super::{
    db,
    models::{AdminHost, BookingPage, HostSettings},
};
use crate::{
    domain::auth::models::UserRecord,
    error::{AppError, AppResult},
};

const SLUG_MIN: usize = 3;
const SLUG_MAX: usize = 40;
const TITLE_MAX: usize = 80;
const DESCRIPTION_MAX: usize = 300;
const MEETING_LINK_MAX: usize = 500;
const NOTICE_HOURS_MAX: i32 = 168;
const WINDOW_DAYS_MAX: i32 = 90;
const SLUG_ATTEMPTS: usize = 5;
const FALLBACK_SLUG: &str = "host";
/// Hours saved before a host picked a timezone were entered as UTC.
const DEFAULT_TIMEZONE: &str = "UTC";

/// Returns the host's settings, creating the row (and their link) on first use.
pub async fn get_or_create(pool: &PgPool, user: &UserRecord) -> AppResult<HostSettings> {
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
    mut settings: HostSettings,
) -> AppResult<HostSettings> {
    settings.public_slug = settings.public_slug.trim().to_lowercase();
    settings.title = settings.title.trim().to_owned();
    settings.description = settings.description.trim().to_owned();
    settings.meeting_link = settings
        .meeting_link
        .map(|link| link.trim().to_owned())
        .filter(|link| !link.is_empty());
    validate(&settings)?;
    match db::update(pool, host_unid, &settings).await {
        Ok(Some(saved)) => Ok(saved),
        Ok(None) => Err(AppError::NotFound),
        Err(error) if is_unique_violation(&error) => {
            Err(AppError::Conflict("link is already taken".to_owned()))
        }
        Err(error) => Err(error.into()),
    }
}

/// Resolves a public link to its page for reading. A paused page is shown
/// with `accepting_bookings` off; unknown and banned hosts look the same.
pub async fn display_page(pool: &PgPool, slug: &str) -> AppResult<BookingPage> {
    db::find_page(pool, Some(&slug.to_lowercase()), None)
        .await?
        .ok_or(AppError::NotFound)
}

/// Resolves a public link to a page that can be booked right now. A paused
/// page looks unknown here.
pub async fn bookable_page(pool: &PgPool, slug: &str) -> AppResult<BookingPage> {
    let page = display_page(pool, slug).await?;
    if page.accepting_bookings {
        Ok(page)
    } else {
        Err(AppError::NotFound)
    }
}

/// The page an existing call was booked through, paused or not.
pub async fn page_of_host(pool: &PgPool, host_unid: Uuid) -> AppResult<BookingPage> {
    db::find_page(pool, None, Some(host_unid))
        .await?
        .ok_or(AppError::NotFound)
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
    if settings.title.is_empty() || settings.title.chars().count() > TITLE_MAX {
        return Err(AppError::BadRequest(
            "title must be 1 to 80 characters".to_owned(),
        ));
    }
    if settings.description.chars().count() > DESCRIPTION_MAX {
        return Err(AppError::BadRequest(
            "description must be at most 300 characters".to_owned(),
        ));
    }
    if timezones::get_by_name(&settings.timezone).is_none() {
        return Err(AppError::BadRequest("timezone is invalid".to_owned()));
    }
    if let Some(link) = &settings.meeting_link
        && !is_valid_meeting_link(link)
    {
        return Err(AppError::BadRequest(
            "meeting link must be an https URL".to_owned(),
        ));
    }
    if !(0..=NOTICE_HOURS_MAX).contains(&settings.minimum_notice_hours) {
        return Err(AppError::BadRequest(
            "minimum notice must be 0 to 168 hours".to_owned(),
        ));
    }
    if !(1..=WINDOW_DAYS_MAX).contains(&settings.booking_window_days) {
        return Err(AppError::BadRequest(
            "booking window must be 1 to 90 days".to_owned(),
        ));
    }
    Ok(())
}

fn is_valid_meeting_link(link: &str) -> bool {
    link.len() <= MEETING_LINK_MAX
        && Url::parse(link).is_ok_and(|url| url.scheme() == "https" && url.has_host())
}

fn is_valid_slug(slug: &str) -> bool {
    (SLUG_MIN..=SLUG_MAX).contains(&slug.len())
        && slug.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
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
            status: String::new(),
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
