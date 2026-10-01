use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/CallingVisitRequest.ts"
)]
pub struct CallingVisitRequest {
    pub source_page: Option<String>,
    pub referer: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/CallingVisitEngagement.ts"
)]
pub struct CallingVisitEngagement {
    pub unid: Uuid,
    pub duration_seconds: Option<i32>,
    pub form_started: Option<bool>,
    pub form_submitted: Option<bool>,
    pub booked_call: Option<bool>,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/CallingVisit.ts")]
pub struct CallingVisit {
    pub unid: Uuid,
    pub source_page: Option<String>,
    pub created_at: DateTime<Utc>,
}
