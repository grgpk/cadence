use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/AvailabilityRuleInput.ts"
)]
pub struct AvailabilityRuleInput {
    pub weekday: i32,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub slot_minutes: i32,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export, export_to = "../../frontend/src/bindings/AvailabilityRule.ts")]
pub struct AvailabilityRule {
    pub id: i64,
    pub host_unid: Uuid,
    pub weekday: i32,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub slot_minutes: i32,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/TimeSlot.ts")]
pub struct TimeSlot {
    pub start: DateTime<Utc>,
    pub available: bool,
}
