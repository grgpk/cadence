use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::auth::models::UserStatus;
use crate::domain::event_types::models::Question;
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

/// What belongs to the host on their booking page, whatever kind of call is
/// booked. Read and written by the host only; each kind of call is an
/// event type.
#[derive(Debug, Deserialize, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/HostSettings.ts")]
pub struct HostSettings {
    pub public_slug: String,
    pub timezone: String,
    /// Where calls happen when Google Calendar does not supply a Meet link.
    /// Read only here; written through `ConferencingUpdate`.
    pub meeting_link: Option<String>,
    /// Read only here; written through `ConferencingUpdate`.
    pub conferencing_app: ConferencingApp,
    /// `false` pauses the page: it can still be read, but not booked.
    pub is_active: bool,
    /// `#rrggbb` for buttons and the picked time on the booking page. Pro.
    #[serde(default = "default_accent")]
    pub accent_color: String,
}

/// The colour a page has until its host picks one: the ink of the app.
pub const DEFAULT_ACCENT: &str = "#111111";

fn default_accent() -> String {
    DEFAULT_ACCENT.to_owned()
}

/// What a visitor's browser may know about a booking page.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/BookingPage.ts")]
pub struct BookingPage {
    #[serde(skip)]
    #[ts(skip)]
    pub host_unid: Uuid,
    /// The kind of call this page books.
    #[serde(skip)]
    #[ts(skip)]
    pub event_type_unid: Uuid,
    /// The type's part of the link: `/book/<host>/<event_slug>`.
    pub event_slug: String,
    /// The type's own call length; `None` follows the host's hours.
    pub duration_minutes: Option<i32>,
    /// Active types older than this one. A plan with a single type books
    /// rank 0 only.
    #[serde(skip)]
    #[ts(skip)]
    pub type_rank: i64,
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
    /// The booking form questions this type asks.
    pub questions: Vec<Question>,
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
    /// The Cadence logo under the card. Paid plans leave it off.
    pub show_branding: bool,
    /// Business: the form says the host may email once if the visitor leaves
    /// without booking.
    pub follow_up_notice: bool,
    /// The host's own accent colour, when their plan shows it.
    pub accent_color: Option<String>,
}

/// A kind of call as a visitor picks it on the host's page.
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/EventTypeCard.ts")]
pub struct EventTypeCard {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub duration_minutes: Option<i32>,
}

/// A host's public link: who they are and the kinds of call they take now.
/// Each flag is an independent fact about the host.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/HostPage.ts")]
pub struct HostPage {
    pub host_name: String,
    /// `false` while the host has paused the page.
    pub accepting_bookings: bool,
    pub show_branding: bool,
    pub calendar_connected: bool,
    /// The host's own accent colour, when their plan shows it.
    pub accent_color: Option<String>,
    /// Bookable kinds of call, oldest first. Types past the host's plan are
    /// left out.
    pub types: Vec<EventTypeCard>,
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
