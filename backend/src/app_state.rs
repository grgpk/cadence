use sqlx::PgPool;

use crate::domain::{billing::stripe::Payments, google_calendar::calendar::HostCalendar};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// The calendar hosts connect: Google in production, in memory in tests.
    pub calendar: HostCalendar,
    /// Stripe, or `None` when its keys are not set: no checkout then.
    pub payments: Option<Payments>,
}
