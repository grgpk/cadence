use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

/// The queue has no terminal failure: a failing email retries every minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/EmailState.ts")]
pub enum EmailState {
    Sent,
    Scheduled,
    /// Failed at least once and still waiting.
    Retrying,
    /// Due a while ago and never attempted, so the worker is not picking it up.
    Overdue,
}

/// A queued email as an admin sees it. The payload stays out: it carries the
/// host's private meeting link.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/AdminEmail.ts")]
pub struct AdminEmail {
    pub id: Uuid,
    pub booking_unid: Option<Uuid>,
    pub email: String,
    pub template: String,
    pub send_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub state: EmailState,
}
