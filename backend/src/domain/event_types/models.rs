use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::text_column::text_column;

/// A booking form question a host can turn on for a kind of call. Values
/// follow the `event_type_questions` and `lead_answers` checks.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../frontend/src/bindings/Question.ts")]
pub enum Question {
    CurrentlyWorkingOn,
    HowHeardAboutUs,
    WhatStoppingYou,
    InvestmentComfort,
    UrgencyLevel,
}

impl Question {
    /// The value as the `question` columns store it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentlyWorkingOn => "currently_working_on",
            Self::HowHeardAboutUs => "how_heard_about_us",
            Self::WhatStoppingYou => "what_stopping_you",
            Self::InvestmentComfort => "investment_comfort",
            Self::UrgencyLevel => "urgency_level",
        }
    }
}

text_column!(Question: CurrentlyWorkingOn, HowHeardAboutUs, WhatStoppingYou, InvestmentComfort, UrgencyLevel);

/// One kind of call a host offers, as the host edits it.
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
    /// The booking form questions it asks.
    pub questions: Vec<Question>,
    /// Switched off by the host: saved, not offered.
    pub is_active: bool,
    /// Offered on the host's link now. An active type past the host's plan
    /// stays saved but is not.
    pub bookable: bool,
}

/// A kind of call as the host saves it.
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
    pub questions: Vec<Question>,
    pub is_active: bool,
}
