import { createFileRoute } from "@tanstack/react-router";

import type { CallingVisit } from "../../../bindings/CallingVisit";
import {
  AdminDataPage,
  formatAdminDate,
} from "../../../domains/admin/components/admin-data-page";

export const Route = createFileRoute("/dashboard/admin/calling-visits")({
  component: AdminCallingVisitsPage,
});

function AdminCallingVisitsPage() {
  return (
    <AdminDataPage<CallingVisit>
      title="Calling Visits"
      description="Call funnel visits and booking intent."
      endpoint="/api/admin/calling-visits"
      emptyMessage="No calling visits found."
      itemKey={(visit) => visit.unid}
      renderItem={(visit) => (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-4">
          <p className="font-medium">{visit.source_page ?? "Unknown source"}</p>
          <p className="text-sm text-muted-foreground">
            {formatAdminDate(visit.created_at)}
          </p>
        </div>
      )}
    />
  );
}
