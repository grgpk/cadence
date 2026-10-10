use sqlx::PgPool;

use super::db;
use crate::error::{AppError, AppResult};

/// How many requests one subject (a client address, an email, or both) may
/// make for one action inside a fixed window.
pub struct Limit {
    action: &'static str,
    max_requests: i32,
    window_minutes: u32,
}

impl Limit {
    const fn new(action: &'static str, max_requests: i32, window_minutes: u32) -> Self {
        Self {
            action,
            max_requests,
            window_minutes,
        }
    }
}

pub const LOGIN_PER_ADDRESS: Limit = Limit::new("login_address", 10, 10);
/// One email from one address. Counting the email alone would let anyone
/// lock its owner out by guessing. Lower than the address limit, so it is
/// the one that stops guesses at a single account.
pub const LOGIN_PER_EMAIL_AND_ADDRESS: Limit = Limit::new("login_email_address", 5, 10);
pub const REGISTER_PER_ADDRESS: Limit = Limit::new("register_address", 5, 60);
pub const FORGOT_PASSWORD_PER_ADDRESS: Limit = Limit::new("forgot_password_address", 5, 60);
pub const FORGOT_PASSWORD_PER_EMAIL: Limit = Limit::new("forgot_password_email", 3, 60);
pub const RESET_PASSWORD_PER_ADDRESS: Limit = Limit::new("reset_password_address", 10, 60);
pub const BOOKING_PER_ADDRESS: Limit = Limit::new("booking_address", 10, 60);
/// The form saves on every blur, so one visitor makes many of these.
pub const LEAD_PER_ADDRESS: Limit = Limit::new("lead_address", 60, 10);
/// Reading, cancelling and moving a call share one budget: it is what
/// guessing manage links would spend.
pub const MANAGE_PER_ADDRESS: Limit = Limit::new("manage_address", 60, 10);
/// The booking page reads open times again as the visitor moves between weeks.
pub const SLOTS_PER_ADDRESS: Limit = Limit::new("slots_address", 120, 10);
/// Recording a visit and updating it share one budget.
pub const CALLING_VISIT_PER_ADDRESS: Limit = Limit::new("calling_visit_address", 60, 10);
pub const BUG_REPORT_PER_ADDRESS: Limit = Limit::new("bug_report_address", 30, 10);

/// Longer subjects are cut, so a made-up header cannot outgrow the index.
const SUBJECT_MAX: usize = 254;

/// Counts this attempt, whether or not it goes on to succeed, and refuses it
/// once the subject is over the limit.
pub async fn check(pool: &PgPool, limit: &Limit, subject: &str) -> AppResult<()> {
    let subject: String = subject.chars().take(SUBJECT_MAX).collect();
    let window_seconds = f64::from(limit.window_minutes * 60);
    if db::hit(pool, limit.action, &subject, window_seconds).await? > limit.max_requests {
        return Err(AppError::TooManyRequests);
    }
    Ok(())
}

pub async fn forget_old(pool: &PgPool) -> AppResult<()> {
    db::delete_old(pool).await.map_err(Into::into)
}
