use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// One kind of call a host offers, as the host edits it. Each `ask_*` flag
/// is an independent switch for one form question.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/EventType.ts")]
pub struct EventType {
    pub unid: Uuid,
    /// The type's part of the link: `/book/<host>/<slug>`.
    pub slug: String,
    pub title: String,
    pub description: String,
    /// The call's length; `None` follows the slot length of the host's hours.
    pub duration_minutes: Option<i32>,
    /// Free time kept before and after every booked call.
    pub buffer_minutes: i32,
    /// Shortest time between now and a call a visitor can book.
    pub minimum_notice_hours: i32,
    /// How many days ahead a visitor can book.
    pub booking_window_days: i32,
    pub ask_currently_working_on: bool,
    pub ask_how_heard_about_us: bool,
    pub ask_what_stopping_you: bool,
    pub ask_investment_comfort: bool,
    pub ask_urgency_level: bool,
    /// Switched off by the host: saved, not offered.
    pub is_active: bool,
    /// Offered on the host's link now. An active type past the host's plan
    /// stays saved but is not.
    pub bookable: bool,
}

/// A kind of call as the host saves it.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/EventTypeInput.ts")]
pub struct EventTypeInput {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub duration_minutes: Option<i32>,
    pub buffer_minutes: i32,
    pub minimum_notice_hours: i32,
    pub booking_window_days: i32,
    pub ask_currently_working_on: bool,
    pub ask_how_heard_about_us: bool,
    pub ask_what_stopping_you: bool,
    pub ask_investment_comfort: bool,
    pub ask_urgency_level: bool,
    pub is_active: bool,
}

impl EventTypeInput {
    /// The booking form questions it asks, in one order.
    pub const fn asks(&self) -> [bool; 5] {
        [
            self.ask_currently_working_on,
            self.ask_how_heard_about_us,
            self.ask_what_stopping_you,
            self.ask_investment_comfort,
            self.ask_urgency_level,
        ]
    }
}

impl EventType {
    pub const fn asks(&self) -> [bool; 5] {
        [
            self.ask_currently_working_on,
            self.ask_how_heard_about_us,
            self.ask_what_stopping_you,
            self.ask_investment_comfort,
            self.ask_urgency_level,
        ]
    }
}
