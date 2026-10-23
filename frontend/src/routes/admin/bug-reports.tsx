import { createFileRoute } from "@tanstack/react-router";

import type { AdminBugReport } from "../../bindings/AdminBugReport";
import { Badge } from "../../components/ui/badge";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { DetailSheet } from "../../domains/admin/components/detail-sheet";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/data-table";
import { APP_NAME } from "../../lib/config";
import { formatDateTime } from "../../lib/format";

export const Route = createFileRoute("/admin/bug-reports")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.bugReports.title} | ${APP_NAME}` }] }),
  component: AdminBugReportsPage,
});

function errorTitle(report: AdminBugReport): string {
  return report.message ?? report.exception_message ?? "No message recorded";
}

const COLUMNS: Column<AdminBugReport>[] = [
  {
    header: "Error",
    className: "max-w-md",
    cell: (report) => (
      <TwoLine
        primary={
          <DetailSheet
            trigger={errorTitle(report)}
            title={errorTitle(report)}
            description={`${report.bug_type}, last seen ${formatDateTime(report.created_at)}`}
            fields={[
              { label: "Page", value: report.url },
              { label: "Exception", value: report.exception_message },
              { label: "Signed in as", value: report.user_login },
              { label: "Browser", value: report.user_agent },
              { label: "Application", value: report.application },
              {
                label: "Times seen",
                value: `${report.similar_count} like this, ${report.total_count} errors in all`,
              },
            ]}
          >
            <div className="grid gap-1 border-t px-4 py-3 text-sm">
              <p className="text-muted-foreground">Stack trace</p>
              {report.stack_trace ? (
                <pre className="overflow-x-auto rounded-md bg-muted p-3 text-xs leading-relaxed">
                  {report.stack_trace}
                </pre>
              ) : (
                <p className="text-muted-foreground">Not recorded</p>
              )}
            </div>
          </DetailSheet>
        }
        secondary={report.url ?? "No URL recorded"}
      />
    ),
  },
  {
    header: "Type",
    cell: (report) => <Badge variant="secondary">{report.bug_type}</Badge>,
  },
  {
    header: "Seen",
    className: "text-right tabular-nums",
    cell: (report) => report.similar_count,
  },
  {
    header: "Last seen",
    className: "text-right text-muted-foreground text-clock text-md",
    cell: (report) => formatDateTime(report.created_at),
  },
];

function AdminBugReportsPage() {
  return (
    <AdminDataPage<AdminBugReport>
      {...ADMIN_PAGES.bugReports}
      emptyTitle="No errors recorded"
      emptyDescription="Errors the application captures are listed here."
      rowKey={(report) => report.unid}
      columns={COLUMNS}
      searchText={(report) =>
        [errorTitle(report), report.url, report.bug_type, report.user_login]
          .filter(Boolean)
          .join(" ")
      }
    />
  );
}
