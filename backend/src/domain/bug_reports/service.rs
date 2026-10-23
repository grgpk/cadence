// Mirrors app/src/domain/admin/monitor/data/bug_reports_db.rs BugReportsDb.
// Only the write path (`add`) is ported — the paginated admin listing/charts live
// behind the internal ops dashboard, out of scope for app_v2_backend (see
// PLAN_SWITCH_TO_TS_FRONTEND.md `/dashboard` exclusion).

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::bug_reports::models::{AdminBugReport, AdminBugType};

pub struct BugReportsService;

pub struct NewBugReport {
    pub bug_type: AdminBugType,
    pub message: String,
    pub user_login: Option<String>,
    pub stack_trace: Option<String>,
    pub url: Option<String>,
    pub user_agent: Option<String>,
}

struct AdminBugReportRow {
    unid: Uuid,
    bug_type: String,
    similarity_hash: i32,
    message: Option<String>,
    exception_message: Option<String>,
    stack_trace: Option<String>,
    user_login: Option<String>,
    url: Option<String>,
    user_agent: Option<String>,
    application: Option<String>,
    created_at: DateTime<Utc>,
    total_count: Option<i64>,
    similar_count: Option<i64>,
}

impl NewBugReport {
    fn bounded(value: Option<&str>, max_bytes: usize) -> Option<String> {
        let value = value?;
        let mut bounded = value.to_owned();
        if bounded.len() > max_bytes {
            let mut end = max_bytes;
            while end > 0 && !bounded.is_char_boundary(end) {
                end -= 1;
            }
            bounded.truncate(end);
        }
        Some(bounded)
    }

    fn exception_message(&self) -> String {
        let mut truncated = self.message.clone();
        // UTF-8 safe truncation: find a valid char boundary.
        if truncated.len() > 512 {
            let mut end = 512;
            while end > 0 && !truncated.is_char_boundary(end) {
                end -= 1;
            }
            truncated.truncate(end);
        }
        truncated
    }

    /// Groups reports of the same bug. The hash is keyed with a server
    /// secret, so a reporter cannot craft a report that lands in the group of
    /// a real one and hides it.
    fn similarity_hash(&self) -> i32 {
        let key = std::env::var("JWT_SECRET").unwrap_or_default();
        let Ok(mut mac) =
            Hmac::<Sha256>::new_from_slice(format!("bug-report-similarity:{key}").as_bytes())
        else {
            return 0;
        };
        let exception_message = self.exception_message();
        let fields = [
            Some(self.bug_type.as_str()),
            Some(exception_message.as_str()),
            Some(self.message.as_str()),
            self.stack_trace.as_deref(),
        ];
        for field in fields {
            // Length-prefixed, so field boundaries cannot be shifted.
            let bytes = field.map_or(&[][..], str::as_bytes);
            mac.update(&[u8::from(field.is_some())]);
            mac.update(&(bytes.len() as u64).to_be_bytes());
            mac.update(bytes);
        }
        let digest = mac.finalize().into_bytes();
        let mut head = [0; 4];
        head.copy_from_slice(digest.get(..4).unwrap_or(&[0; 4]));
        i32::from_be_bytes(head)
    }
}

impl BugReportsService {
    pub async fn add(pool: &PgPool, report: &NewBugReport) -> Result<Uuid, sqlx::Error> {
        let bug_type = report.bug_type.as_str();
        let message = NewBugReport::bounded(Some(&report.message), 16_384).unwrap_or_default();
        let exception_message = {
            let mut truncated = message.clone();
            if truncated.len() > 512 {
                let mut end = 512;
                while end > 0 && !truncated.is_char_boundary(end) {
                    end -= 1;
                }
                truncated.truncate(end);
            }
            truncated
        };
        let stack_trace = NewBugReport::bounded(report.stack_trace.as_deref(), 16_384);
        let user_login = NewBugReport::bounded(report.user_login.as_deref(), 255);
        let url = NewBugReport::bounded(report.url.as_deref(), 2_048);
        let user_agent = NewBugReport::bounded(report.user_agent.as_deref(), 512);
        let similarity_hash = report.similarity_hash();
        let application = std::env::var("HOSTNAME").ok();

        let unid = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO bug_reports (
                bug_type, similarity_hash, message, exception_message,
                stack_trace, user_login, url, user_agent, application
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING unid",
        )
        .bind(bug_type)
        .bind(similarity_hash)
        .bind(message)
        .bind(exception_message)
        .bind(stack_trace)
        .bind(user_login)
        .bind(url)
        .bind(user_agent)
        .bind(application)
        .fetch_one(pool)
        .await?;

        Ok(unid)
    }

    pub async fn list(pool: &PgPool) -> Result<Vec<AdminBugReport>, sqlx::Error> {
        let rows = sqlx::query_as!(
            AdminBugReportRow,
            r#"WITH unique_bugs AS (
                SELECT DISTINCT ON (similarity_hash)
                    unid, bug_type, similarity_hash, message, exception_message,
                    stack_trace, user_login, url, user_agent, application, created_at,
                    -- Window functions run before DISTINCT ON, so this counts the whole group.
                    COUNT(*) OVER (PARTITION BY similarity_hash) AS similar_count
                FROM bug_reports
                ORDER BY similarity_hash, created_at DESC
            )
            SELECT unid, bug_type, similarity_hash, message, exception_message,
                stack_trace, user_login, url, user_agent, application, created_at,
                    COUNT(*) OVER() AS total_count, similar_count
            FROM unique_bugs ORDER BY created_at DESC LIMIT 500"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AdminBugReport {
                unid: row.unid,
                bug_type: row.bug_type,
                similarity_hash: row.similarity_hash,
                message: row.message,
                exception_message: row.exception_message,
                stack_trace: row.stack_trace,
                user_login: row.user_login,
                url: row.url,
                user_agent: row.user_agent,
                application: row.application,
                created_at: row.created_at,
                total_count: row.total_count.unwrap_or_default(),
                similar_count: row.similar_count.unwrap_or_default(),
            })
            .collect())
    }
}

// ============================================================
// CONTRACT TESTS
// ============================================================

#[cfg(test)]
mod contract_tests {
    use sqlx::PgPool;

    use super::{BugReportsService, NewBugReport};
    use crate::domain::bug_reports::models::AdminBugType;

    #[sqlx::test(migrations = "../migrations")]
    async fn add_writes_expected_row_shape(pool: PgPool) -> Result<(), sqlx::Error> {
        let report = NewBugReport {
            bug_type: AdminBugType::JsError,
            message: "TypeError: undefined is not a function".to_owned(),
            user_login: Some("octocat".to_owned()),
            stack_trace: Some("at foo.js:1:1".to_owned()),
            url: Some("https://rustify.rs/v2/calling".to_owned()),
            user_agent: Some("Mozilla/5.0".to_owned()),
        };

        let unid = BugReportsService::add(&pool, &report).await?;

        #[derive(sqlx::FromRow)]
        struct BugRow {
            bug_type: String,
            user_login: Option<String>,
            url: Option<String>,
        }

        let row = sqlx::query_as!(
            BugRow,
            "SELECT bug_type, user_login, url FROM bug_reports WHERE unid = $1",
            unid,
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.bug_type, "JsError");
        assert_eq!(row.user_login, Some("octocat".to_owned()));
        assert_eq!(row.url, Some("https://rustify.rs/v2/calling".to_owned()));
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn add_truncates_exception_message_at_char_boundary(
        pool: PgPool,
    ) -> Result<(), sqlx::Error> {
        let mut message = "a".repeat(510);
        message.push('🔥'); // 4-byte emoji near the 512 boundary
        message.push('x');

        let report = NewBugReport {
            bug_type: AdminBugType::Bug,
            message,
            user_login: None,
            stack_trace: None,
            url: None,
            user_agent: None,
        };

        let unid = BugReportsService::add(&pool, &report).await?;

        let exception_message: Option<String> =
            sqlx::query_scalar("SELECT exception_message FROM bug_reports WHERE unid = $1")
                .bind(unid)
                .fetch_one(&pool)
                .await?;

        let Some(exception_message) = exception_message else {
            return Err(sqlx::Error::RowNotFound);
        };
        assert!(exception_message.len() <= 512);
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn list_shows_each_bug_once_with_how_often_it_happened(
        pool: PgPool,
    ) -> Result<(), sqlx::Error> {
        let report = |message: &str| NewBugReport {
            bug_type: AdminBugType::Bug,
            message: message.to_owned(),
            user_login: None,
            stack_trace: None,
            url: None,
            user_agent: None,
        };
        for message in ["same failure", "same failure", "same failure", "other"] {
            BugReportsService::add(&pool, &report(message)).await?;
        }

        let listed = BugReportsService::list(&pool).await?;

        let mut counts: Vec<i64> = listed.iter().map(|bug| bug.similar_count).collect();
        counts.sort_unstable();
        assert_eq!(counts, vec![1, 3]);
        assert!(listed.iter().all(|bug| bug.total_count == 2));
        Ok(())
    }
}
