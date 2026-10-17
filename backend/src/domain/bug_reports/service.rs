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
    id: i64,
    unid: Uuid,
    bugtype: String,
    similarityhash: i32,
    message: Option<String>,
    exceptionmessage: Option<String>,
    stacktrace: Option<String>,
    userlogin: Option<String>,
    url: Option<String>,
    useragent: Option<String>,
    application: Option<String>,
    created: DateTime<Utc>,
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
    pub async fn add(pool: &PgPool, report: &NewBugReport) -> Result<i64, sqlx::Error> {
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

        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO bug_reports (
                bugtype, similarityhash, message, exceptionmessage,
                stacktrace, userlogin, url, useragent, application
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id",
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

        Ok(id)
    }

    pub async fn list(pool: &PgPool) -> Result<Vec<AdminBugReport>, sqlx::Error> {
        let rows = sqlx::query_as!(
            AdminBugReportRow,
            r#"WITH unique_bugs AS (
                SELECT DISTINCT ON (similarityhash)
                    id, unid, bugtype, similarityhash, message, exceptionmessage,
                    stacktrace, userlogin, url, useragent, application, created
                FROM bug_reports
                ORDER BY similarityhash, created DESC
            ), counted AS (
                SELECT u.*,
                    (SELECT COUNT(*) FROM bug_reports b2 WHERE b2.similarityhash = u.similarityhash) AS similar_count
                FROM unique_bugs u
            )
            SELECT id, unid, bugtype, similarityhash, message, exceptionmessage,
                stacktrace, userlogin, url, useragent, application, created,
                    COUNT(*) OVER() AS total_count, similar_count
            FROM counted ORDER BY created DESC LIMIT 500"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AdminBugReport {
                id: row.id,
                unid: row.unid,
                bugtype: row.bugtype,
                similarityhash: row.similarityhash,
                message: row.message,
                exceptionmessage: row.exceptionmessage,
                stacktrace: row.stacktrace,
                userlogin: row.userlogin,
                url: row.url,
                useragent: row.useragent,
                application: row.application,
                created: row.created,
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

        let id = BugReportsService::add(&pool, &report).await?;
        assert!(id > 0);

        #[derive(sqlx::FromRow)]
        struct BugRow {
            bugtype: String,
            userlogin: Option<String>,
            url: Option<String>,
        }

        let row = sqlx::query_as!(
            BugRow,
            "SELECT bugtype, userlogin, url FROM bug_reports WHERE id = $1",
            id,
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.bugtype, "JsError");
        assert_eq!(row.userlogin, Some("octocat".to_owned()));
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

        let id = BugReportsService::add(&pool, &report).await?;

        let exception_message: Option<String> =
            sqlx::query_scalar("SELECT exceptionmessage FROM bug_reports WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await?;

        let Some(exception_message) = exception_message else {
            return Err(sqlx::Error::RowNotFound);
        };
        assert!(exception_message.len() <= 512);
        Ok(())
    }
}
