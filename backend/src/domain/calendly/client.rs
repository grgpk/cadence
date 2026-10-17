//! The Calendly seam: its API with the host's personal access token in
//! production, a fixed account in tests. The token is used for the requests
//! of one preview and is never stored or logged.

use std::{sync::Arc, time::Duration};

use serde::Deserialize;
use thiserror::Error;

use super::models::{ApiEventType, ApiSchedule, ApiUser, CalendlyAccount};

const API: &str = "https://api.calendly.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Event types are read 100 at a time, for at most this many pages.
const EVENT_TYPE_PAGES: usize = 3;
/// The token the fake accepts.
pub const FAKE_TOKEN: &str = "calendly-test-token";

#[derive(Debug, Error)]
pub enum CalendlyError {
    #[error("calendly did not accept the token")]
    Unauthorized,
    #[error("calendly request failed")]
    Request(#[source] reqwest::Error),
}

#[derive(Clone)]
pub enum Calendly {
    Live(reqwest::Client),
    Fake(Arc<CalendlyAccount>),
}

#[derive(Deserialize)]
struct Resource<T> {
    resource: T,
}

#[derive(Deserialize)]
struct Collection<T> {
    collection: Vec<T>,
    pagination: Option<Pagination>,
}

#[derive(Deserialize)]
struct Pagination {
    next_page_token: Option<String>,
}

impl Calendly {
    pub fn live() -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self::Live(client)
    }

    /// Answers `FAKE_TOKEN` with `account`, and refuses any other token.
    pub fn fake(account: CalendlyAccount) -> Self {
        Self::Fake(Arc::new(account))
    }

    /// The account behind `token`: who it is, its active event types and its
    /// default schedule.
    pub async fn account(&self, token: &str) -> Result<CalendlyAccount, CalendlyError> {
        match self {
            Self::Fake(account) if token == FAKE_TOKEN => Ok((**account).clone()),
            Self::Fake(_) => Err(CalendlyError::Unauthorized),
            Self::Live(client) => live_account(client, token).await,
        }
    }
}

async fn live_account(
    client: &reqwest::Client,
    token: &str,
) -> Result<CalendlyAccount, CalendlyError> {
    let user = get::<Resource<ApiUser>>(client, token, "/users/me", &[])
        .await?
        .resource;
    let mut event_types = Vec::new();
    let mut page_token: Option<String> = None;
    for _ in 0..EVENT_TYPE_PAGES {
        let mut query = vec![
            ("user", user.uri.clone()),
            ("active", "true".to_owned()),
            ("count", "100".to_owned()),
        ];
        if let Some(page) = page_token.take() {
            query.push(("page_token", page));
        }
        let page = get::<Collection<ApiEventType>>(client, token, "/event_types", &query).await?;
        event_types.extend(page.collection);
        page_token = page.pagination.and_then(|pages| pages.next_page_token);
        if page_token.is_none() {
            break;
        }
    }
    let schedules = get::<Collection<ApiSchedule>>(
        client,
        token,
        "/user_availability_schedules",
        &[("user", user.uri.clone())],
    )
    .await?
    .collection;
    let schedule = schedules.into_iter().find(|schedule| schedule.default);
    Ok(CalendlyAccount {
        user,
        event_types,
        schedule,
    })
}

async fn get<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    token: &str,
    path: &str,
    query: &[(&str, String)],
) -> Result<T, CalendlyError> {
    let response = client
        .get(format!("{API}{path}"))
        .bearer_auth(token)
        .query(query)
        .send()
        .await
        .map_err(CalendlyError::Request)?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(CalendlyError::Unauthorized);
    }
    response
        .error_for_status()
        .map_err(CalendlyError::Request)?
        .json::<T>()
        .await
        .map_err(CalendlyError::Request)
}
