import { createFileRoute } from "@tanstack/react-router";

import type { AdminBugReport } from "../../bindings/AdminBugReport";
import { Badge } from "../../components/ui/badge";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/DataTable";
import { formatDateTime } from "../../lib/format";

export const Route = createFileRoute("/admin/bug-reports")({
  component: AdminBugReportsPage,
});

const COLUMNS: Column<AdminBugReport>[] = [
  {
    header: "Error",
    className: "max-w-md whitespace-normal",
    cell: (report) => (
      <TwoLine
        primary={report.message ?? report.exceptionmessage ?? "No message recorded"}
        secondary={report.url ?? "No URL recorded"}
      />
    ),
  },
  {
    header: "Type",
    cell: (report) => <Badge variant="secondary">{report.bugtype}</Badge>,
  },
  {
    header: "Seen",
    className: "text-right tabular-nums",
    cell: (report) => String(report.similar_count),
  },
  {
    header: "Last seen",
    className: "text-right text-muted-foreground tabular-nums",
    cell: (report) => formatDateTime(report.created),
  },
];

function AdminBugReportsPage() {
  return (
    <AdminDataPage<AdminBugReport>
      {...ADMIN_PAGES.bugReports}
      endpoint="/api/admin/bug-reports"
      emptyTitle="No errors recorded"
      emptyDescription="Errors the application captures are listed here."
      rowKey={(report) => report.unid}
      columns={COLUMNS}
    />
  );
}
