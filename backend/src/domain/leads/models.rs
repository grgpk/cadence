use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// The values of the `leads_qualification_status` check. The fields that carry
/// one stay strings for sqlx; this gives the frontend the closed set.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LeadQualification.ts")]
pub enum LeadQualification {
    Qualified,
    NotSure,
    Disqualified,
}

#[derive(Debug, Default, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/LeadUpdate.ts")]
#[serde(default)]
pub struct LeadUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub country_code: Option<String>,
    pub investment_comfort: Option<String>,
    pub what_stopping_you: Option<String>,
    pub how_heard_about_us: Option<String>,
    pub currently_working_on: Option<String>,
    pub urgency_level: Option<String>,
    pub source_page: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    #[ts(as = "Option<LeadQualification>")]
    pub qualification_status: Option<String>,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
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
    pub investment_comfort: Option<String>,
    pub what_stopping_you: Option<String>,
    pub how_heard_about_us: Option<String>,
    pub currently_working_on: Option<String>,
    pub urgency_level: Option<String>,
    pub source_page: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    #[ts(as = "Option<LeadQualification>")]
    pub qualification_status: Option<String>,
    pub form_submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/SubmitLeadRequest.ts")]
pub struct SubmitLeadRequest {
    pub source_page: Option<String>,
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
