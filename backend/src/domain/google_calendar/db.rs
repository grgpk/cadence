use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::google_calendar::models::GoogleCalendarConfig;

/// The `provider` of the states made for connecting a calendar.
const OAUTH_PROVIDER: &str = "google_calendar";

pub struct GoogleCalendarDb;

impl GoogleCalendarDb {
    pub async fn get(
        pool: &PgPool,
        host_unid: Uuid,
    ) -> Result<Option<GoogleCalendarConfig>, sqlx::Error> {
        sqlx::query_as!(
            GoogleCalendarConfig,
            "SELECT host_unid, access_token, refresh_token, expires_at, calendar_id
             FROM google_calendar_config WHERE host_unid = $1",
            host_unid
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn save(
        pool: &PgPool,
        host_unid: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!(
            "INSERT INTO google_calendar_config
                (host_unid, access_token, refresh_token, expires_at, calendar_id, updated_at)
             VALUES ($1, $2, $3, $4, 'primary', now())
             ON CONFLICT (host_unid) DO UPDATE SET
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                expires_at = EXCLUDED.expires_at,
                updated_at = now()
             RETURNING 1",
            host_unid,
            access_token,
            refresh_token,
            expires_at,
        )
        .fetch_optional(pool)
        .await
        .map(|_| ())
    }

    pub async fn update_access_token(
        pool: &PgPool,
        host_unid: Uuid,
        access_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!(
            "UPDATE google_calendar_config
             SET access_token = $1, expires_at = $2, updated_at = now()
             WHERE host_unid = $3
             RETURNING 1",
            access_token,
            expires_at,
            host_unid,
        )
        .fetch_optional(pool)
        .await
        .map(|_| ())
    }

    pub async fn delete(pool: &PgPool, host_unid: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!(
            "DELETE FROM google_calendar_config WHERE host_unid = $1 RETURNING 1",
            host_unid
        )
        .fetch_optional(pool)
        .await
        .map(|_| ())
    }

    /// Remembers the `state` a host takes to Google's consent screen.
    pub async fn create_state(
        pool: &PgPool,
        state: &str,
        host_unid: Uuid,
        redirect_uri: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!(
            "INSERT INTO oauth_states (state, user_unid, provider, redirect_uri, expires_at)
             VALUES ($1, $2, $3, $4, $5) RETURNING 1",
            state,
            host_unid,
            OAUTH_PROVIDER,
            redirect_uri,
            expires_at,
        )
        .fetch_one(pool)
        .await
        .map(|_| ())
    }

    /// Spends a `state` and returns the redirect it was made for. `None`
    /// when it is unknown, expired, already used or another host's.
    pub async fn consume_state(
        pool: &PgPool,
        state: &str,
        host_unid: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar!(
            "UPDATE oauth_states SET consumed_at = now()
             WHERE state = $1 AND user_unid = $2 AND provider = $3
               AND consumed_at IS NULL AND expires_at > now()
             RETURNING redirect_uri",
            state,
            host_unid,
            OAUTH_PROVIDER,
        )
        .fetch_optional(pool)
        .await
    }

    /// Drops states that stopped working a day ago or more.
    pub async fn delete_old_states(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!(
            "DELETE FROM oauth_states WHERE expires_at < now() - interval '1 day' RETURNING 1",
        )
        .fetch_all(pool)
        .await
        .map(|_| ())
    }

    pub fn is_expired(expires_at: DateTime<Utc>) -> bool {
        expires_at < Utc::now() + chrono::Duration::minutes(5)
    }
}
