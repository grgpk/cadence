use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
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
    #[ts(type = "number")]
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
    pub minutes: i32,
}

/// One stretch of open hours on a date that has its own hours.
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/HoursRange.ts")]
pub struct HoursRange {
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub slot_minutes: i32,
}

/// A date whose hours replace the weekly hours. No ranges: off all day.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/DateOverride.ts")]
pub struct DateOverride {
    pub date: NaiveDate,
    pub ranges: Vec<HoursRange>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/DateOverrideInput.ts")]
pub struct DateOverrideInput {
    pub ranges: Vec<HoursRange>,
}

/// A row of `availability_overrides`. Times are `None` on a day off.
#[derive(Debug, sqlx::FromRow)]
pub struct OverrideRow {
    pub date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub slot_minutes: Option<i32>,
}
