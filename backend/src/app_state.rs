use sqlx::PgPool;

use crate::domain::google_calendar::calendar::HostCalendar;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// The calendar hosts connect: Google in production, in memory in tests.
    pub calendar: HostCalendar,
}
