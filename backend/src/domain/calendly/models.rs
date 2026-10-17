use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// What Calendly's API answers, as far as an import reads it.

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ApiUser {
    pub uri: String,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiEventType {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub duration: i32,
    pub description_plain: Option<String>,
    pub active: bool,
    /// `solo` or `group`.
    pub kind: String,
    /// `round_robin`, `collective` or `multi_pool` for team events.
    pub pooling_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiSchedule {
    pub default: bool,
    pub timezone: String,
    pub rules: Vec<ApiRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRule {
    /// `wday` for a weekday, `date` for one date.
    #[serde(rename = "type")]
    pub kind: String,
    pub wday: Option<String>,
    pub date: Option<NaiveDate>,
    /// Empty: no bookings that day.
    pub intervals: Vec<ApiInterval>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiInterval {
    /// "08:30", in the schedule's timezone.
    pub from: String,
    pub to: String,
}

/// Everything an import reads from one Calendly account.
#[derive(Debug, Clone, Default)]
pub struct CalendlyAccount {
    pub user: ApiUser,
    pub event_types: Vec<ApiEventType>,
    /// The schedule Calendly uses by default, if there is one.
    pub schedule: Option<ApiSchedule>,
}

// What Cadence shows and saves.

/// The personal access token, used for one preview and never stored.
#[derive(Debug, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/CalendlyTokenRequest.ts"
)]
pub struct CalendlyTokenRequest {
    pub token: String,
}

/// A Calendly event type as a Cadence kind of call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ImportedEventType.ts")]
pub struct ImportedEventType {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub duration_minutes: i32,
}

/// A Calendly event type the import leaves out, and why.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/SkippedEventType.ts")]
pub struct SkippedEventType {
    pub name: String,
    pub reason: String,
}

/// Open hours on one weekday; 0 is Monday.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ImportedHours.ts")]
pub struct ImportedHours {
    pub weekday: i32,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
}

/// A date with its own hours. No ranges: off all day.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ImportedDate.ts")]
pub struct ImportedDate {
    pub date: NaiveDate,
    pub ranges: Vec<ImportedRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ImportedRange.ts")]
pub struct ImportedRange {
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
}

/// What an import would bring in, for the host to review.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CalendlyPreview.ts")]
pub struct CalendlyPreview {
    /// The schedule's timezone, or the account's.
    pub timezone: Option<String>,
    pub event_types: Vec<ImportedEventType>,
    pub skipped: Vec<SkippedEventType>,
    pub weekly: Vec<ImportedHours>,
    pub dates: Vec<ImportedDate>,
    /// How many kinds of call the host's plan offers; `None` is unlimited.
    pub types_allowed: Option<u32>,
}

/// The reviewed import, as the host chose to bring it in.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CalendlyImport.ts")]
pub struct CalendlyImport {
    pub timezone: Option<String>,
    pub event_types: Vec<ImportedEventType>,
    pub weekly: Vec<ImportedHours>,
    pub dates: Vec<ImportedDate>,
}

/// What the import saved.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/CalendlyImported.ts")]
pub struct CalendlyImported {
    pub event_types: u32,
    /// Picked but past the host's plan, so not brought in.
    pub event_types_left_out: u32,
    pub weekly_hours: u32,
    pub dates: u32,
}
