use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::event_types::models::Question;

/// How ready a lead is to buy, read from their investment answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LeadQualification.ts")]
pub enum LeadQualification {
    Qualified,
    NotSure,
    Disqualified,
}

/// The visitor's answer to one of the type's questions.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LeadAnswer.ts")]
pub struct LeadAnswer {
    pub question: Question,
    pub answer: String,
}

/// The booking form as far as the visitor has filled it in. A field left
/// out keeps what was saved; an answer left out keeps its saved one.
#[derive(Debug, Default, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LeadUpdate.ts")]
#[serde(default)]
pub struct LeadUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub country_code: Option<String>,
    pub answers: Vec<LeadAnswer>,
    /// The visit the form was opened in, once the page has one.
    pub calling_visit_unid: Option<Uuid>,
}

/// A lead as stored, before its answers are read.
pub struct LeadRow {
    pub unid: Uuid,
    pub host_unid: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub country_code: Option<String>,
    pub source_page: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub form_submitted_at: Option<DateTime<Utc>>,
}

/// A lead with its person, answers and the visit it came from.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Lead.ts")]
pub struct Lead {
    pub unid: Uuid,
    pub host_unid: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub country_code: Option<String>,
    pub answers: Vec<LeadAnswer>,
    pub qualification_status: Option<LeadQualification>,
    pub source_page: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub form_submitted_at: Option<DateTime<Utc>>,
}

/// Someone who started booking with the host, gave an email and left without
/// a call. Business hosts can send them their times once.
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/UnfinishedLead.ts")]
pub struct UnfinishedLead {
    pub unid: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    /// When they last typed on the form.
    pub left_at: DateTime<Utc>,
}
