use sqlx::PgPool;

use crate::domain::{
    billing::stripe::Payments, calendly::client::Calendly, crypto_billing::gateway::CryptoPayments,
    google_calendar::calendar::HostCalendar,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// The calendar hosts connect: Google in production, in memory in tests.
    pub calendar: HostCalendar,
    /// Stripe, or `None` when its keys are not set: no checkout then.
    pub payments: Option<Payments>,
    /// The crypto provider `CRYPTO_PROVIDER` names, or `None`: no crypto
    /// checkout then.
    pub crypto: Option<CryptoPayments>,
    /// Calendly's API for imports; a fixed account in tests.
    pub calendly: Calendly,
}
