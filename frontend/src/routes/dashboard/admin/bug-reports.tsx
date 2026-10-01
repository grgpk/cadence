import { createFileRoute } from "@tanstack/react-router";

import type { AdminBugReport } from "../../../bindings/AdminBugReport";
import {
  AdminDataPage,
  formatAdminDate,
} from "../../../domains/admin/components/admin-data-page";

export const Route = createFileRoute("/dashboard/admin/bug-reports")({
  component: AdminBugReportsPage,
});

function AdminBugReportsPage() {
  return (
    <AdminDataPage<AdminBugReport>
      title="Bug Reports"
      description="Application errors captured by Cadence."
      endpoint="/api/admin/bug-reports"
      emptyMessage="No bug reports found."
      itemKey={(report) => report.unid}
      renderItem={(report) => (
        <div className="rounded-lg border p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <p className="font-medium">{report.bugtype}</p>
            <p className="text-sm text-muted-foreground">
              {formatAdminDate(report.created)}
            </p>
          </div>
          <p className="mt-2 text-sm text-muted-foreground">
            {report.message ?? report.exceptionmessage ?? "No message"}
          </p>
        </div>
      )}
    />
  );
}
