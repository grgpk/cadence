use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

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
    pub qualification_status: Option<String>,
    pub form_submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/SubmitLeadRequest.ts")]
pub struct SubmitLeadRequest {
    pub source_page: Option<String>,
}
