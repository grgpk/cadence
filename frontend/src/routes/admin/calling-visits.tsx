import { createFileRoute } from "@tanstack/react-router";

import type { CallingVisit } from "../../bindings/CallingVisit";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import type { Column } from "../../domains/shared/data-table";
import { APP_NAME } from "../../lib/config";
import { formatDateTime } from "../../lib/format";

export const Route = createFileRoute("/admin/calling-visits")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.callingVisits.title} | ${APP_NAME}` }] }),
  component: AdminCallingVisitsPage,
});

const COLUMNS: Column<CallingVisit>[] = [
  {
    header: "Page",
    className: "font-medium",
    cell: (visit) =>
      visit.source_page ?? <span className="text-muted-foreground">Unknown page</span>,
  },
  {
    header: "Visited",
    className: "text-right text-muted-foreground text-clock text-[0.9375rem]",
    cell: (visit) => formatDateTime(visit.created_at),
  },
];

function AdminCallingVisitsPage() {
  return (
    <AdminDataPage<CallingVisit>
      {...ADMIN_PAGES.callingVisits}
      emptyTitle="No visits yet"
      emptyDescription="Each visit to the booking page is recorded here."
      rowKey={(visit) => visit.unid}
      columns={COLUMNS}
    />
  );
}
