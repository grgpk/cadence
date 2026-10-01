use sqlx::PgPool;
use tracing::Level;
use tracing_subscriber::Layer;

use crate::domain::bug_reports::models::AdminBugType;
use crate::domain::bug_reports::service::{BugReportsService, NewBugReport};

pub struct BugReportLayer {
    pub pool: PgPool,
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            value.clone_into(&mut self.message);
        }
    }
}

impl<S: tracing::Subscriber> Layer<S> for BugReportLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if *event.metadata().level() != Level::ERROR {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        if visitor.message.is_empty() {
            return;
        }

        let report = NewBugReport {
            bug_type: AdminBugType::Server,
            message: visitor.message,
            stack_trace: None,
            user_login: None,
            url: None,
            user_agent: None,
        };
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = BugReportsService::add(&pool, &report).await;
        });
    }
}
