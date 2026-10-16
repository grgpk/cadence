use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::auth::models::UserStatus;
use crate::text_column::text_column;

/// The values of the `widget_conferencing_app` check: where every call of a
/// host happens.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/ConferencingApp.ts")]
pub enum ConferencingApp {
    /// A Meet made with the Google Calendar event.
    GoogleMeet,
    /// The host's own `meeting_link`.
    CustomLink,
}

impl ConferencingApp {
    /// The value as the `conferencing_app` column stores it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GoogleMeet => "google_meet",
            Self::CustomLink => "custom_link",
        }
    }
}

text_column!(ConferencingApp: GoogleMeet, CustomLink);

/// What the host picks on the Integrations page.
#[derive(Debug, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/ConferencingUpdate.ts"
)]
pub struct ConferencingUpdate {
    pub app: ConferencingApp,
    pub meeting_link: Option<String>,
}

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
    /// Read only here; written through `ConferencingUpdate`.
    pub meeting_link: Option<String>,
    /// Read only here; written through `ConferencingUpdate`.
    pub conferencing_app: ConferencingApp,
    pub ask_currently_working_on: bool,
    pub ask_how_heard_about_us: bool,
    pub ask_what_stopping_you: bool,
    pub ask_investment_comfort: bool,
    pub ask_urgency_level: bool,
    /// Shortest time between now and a call a visitor can book.
    pub minimum_notice_hours: i32,
    /// How many days ahead a visitor can book.
    pub booking_window_days: i32,
    /// Free time kept before and after every booked call.
    pub buffer_minutes: i32,
    /// `false` pauses the page: it can still be read, but not booked.
    pub is_active: bool,
}

/// What a visitor's browser may know about a booking page.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/BookingPage.ts")]
pub struct BookingPage {
    #[serde(skip)]
    #[ts(skip)]
    pub host_unid: Uuid,
    /// Where the host is told about their calls.
    #[serde(skip)]
    #[ts(skip)]
    pub host_email: String,
    #[serde(skip)]
    #[ts(skip)]
    pub public_slug: String,
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
    pub conferencing_app: ConferencingApp,
    /// The link itself is only given to someone who has booked.
    #[serde(skip)]
    #[ts(skip)]
    pub meeting_link: Option<String>,
    pub has_meeting_link: bool,
    /// Slots are already cut to the notice period, so the page does not need it.
    #[serde(skip)]
    #[ts(skip)]
    pub minimum_notice_hours: i32,
    pub booking_window_days: i32,
    /// Slots are already cut to the buffer, so the page does not need it.
    #[serde(skip)]
    #[ts(skip)]
    pub buffer_minutes: i32,
    /// `false` while the host has paused the page.
    pub accepting_bookings: bool,
    /// The host's plan has no bookings left this month. Calls already booked
    /// can still move; new ones wait for next month.
    pub full: bool,
    /// The Cadence logo under the card. Paid plans leave it off.
    pub show_branding: bool,
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
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    /// `None` until the host first opens their dashboard.
    pub public_slug: Option<String>,
    pub page_active: bool,
    pub has_hours: bool,
    pub calendar_connected: bool,
    pub has_meeting_link: bool,
}
