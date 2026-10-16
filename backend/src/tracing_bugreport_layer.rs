use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;
use tracing_subscriber::Layer;

use crate::domain::bug_reports::models::AdminBugType;
use crate::domain::bug_reports::service::{BugReportsService, NewBugReport};

/// Reports waiting to be written. Past this, new ones are dropped: when errors
/// pour in, often because the database itself is failing, more writes to it
/// would only add load.
const QUEUE_MAX: usize = 64;

/// Turns every error event into a server bug report, written by one task.
pub struct BugReportLayer {
    reports: mpsc::Sender<NewBugReport>,
}

impl BugReportLayer {
    /// Starts the one task that writes reports. Needs a running Tokio runtime.
    pub fn start(pool: PgPool) -> Self {
        let (reports, mut queue) = mpsc::channel::<NewBugReport>(QUEUE_MAX);
        tokio::spawn(async move {
            while let Some(report) = queue.recv().await {
                // A report that cannot be written is not reported again.
                let _ = BugReportsService::add(&pool, &report).await;
            }
        });
        Self { reports }
    }

    /// Queues a report without waiting; `false` when it was dropped.
    fn offer(&self, report: NewBugReport) -> bool {
        self.reports.try_send(report).is_ok()
    }
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

const fn server_report(message: String) -> NewBugReport {
    NewBugReport {
        bug_type: AdminBugType::Server,
        message,
        stack_trace: None,
        user_login: None,
        url: None,
        user_agent: None,
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
        self.offer(server_report(visitor.message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_queue_drops_reports_instead_of_waiting_or_spawning() {
        let (reports, _queue) = mpsc::channel(1);
        let layer = BugReportLayer { reports };
        assert!(layer.offer(server_report("first".to_owned())));
        assert!(!layer.offer(server_report("second".to_owned())));
    }

    #[test]
    fn a_closed_queue_drops_reports_too() {
        let (reports, queue) = mpsc::channel(1);
        drop(queue);
        let layer = BugReportLayer { reports };
        assert!(!layer.offer(server_report("lost".to_owned())));
    }
}
