use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::auth::models::UserStatus;

/// A host's own booking page settings. Read and written by the host only.
/// Each `ask_*` flag is an independent switch for one form question.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/HostSettings.ts")]
pub struct HostSettings {
    pub public_slug: String,
    pub title: String,
    pub description: String,
    pub timezone: String,
    /// Where calls happen when Google Calendar does not supply a Meet link.
    pub meeting_link: Option<String>,
    pub ask_currently_working_on: bool,
    pub ask_how_heard_about_us: bool,
    pub ask_what_stopping_you: bool,
    pub ask_investment_comfort: bool,
    pub ask_urgency_level: bool,
}

/// What a visitor's browser may know about a booking page.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/BookingPage.ts")]
pub struct BookingPage {
    #[serde(skip)]
    #[ts(skip)]
    pub host_unid: Uuid,
    pub host_name: String,
    pub title: String,
    pub description: String,
    pub timezone: String,
    pub ask_currently_working_on: bool,
    pub ask_how_heard_about_us: bool,
    pub ask_what_stopping_you: bool,
    pub ask_investment_comfort: bool,
    pub ask_urgency_level: bool,
    pub calendar_connected: bool,
    /// The link itself is only given to someone who has booked.
    #[serde(skip)]
    #[ts(skip)]
    pub meeting_link: Option<String>,
    pub has_meeting_link: bool,
}

/// One host as an admin sees them: who they are and how far their booking
/// page is set up.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/AdminHost.ts")]
pub struct AdminHost {
    pub unid: Uuid,
    pub full_name: String,
    pub email: String,
    #[ts(as = "UserStatus")]
    pub status: String,
    pub created_at: DateTime<Utc>,
    /// `None` until the host first opens their dashboard.
    pub public_slug: Option<String>,
    pub page_active: bool,
    pub has_hours: bool,
    pub calendar_connected: bool,
    pub has_meeting_link: bool,
}
