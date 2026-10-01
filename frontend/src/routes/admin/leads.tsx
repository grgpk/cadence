import { createFileRoute } from "@tanstack/react-router";

import type { Lead } from "../../bindings/Lead";
import {
  AdminDataPage,
  formatAdminDate,
} from "../../domains/admin/components/admin-data-page";

export const Route = createFileRoute("/admin/leads")({
  component: AdminLeadsPage,
});

function AdminLeadsPage() {
  return (
    <AdminDataPage<Lead>
      title="Leads"
      description="All coaching leads captured by Cadence."
      endpoint="/api/admin/leads"
      emptyMessage="No leads found."
      itemKey={(lead) => lead.unid}
      renderItem={(lead) => (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-4">
          <div>
            <p className="font-medium">
              {[lead.first_name, lead.last_name].filter(Boolean).join(" ") ||
                "Unnamed lead"}
            </p>
            <p className="text-sm text-muted-foreground">{lead.email ?? "No email"}</p>
          </div>
          <div className="text-right text-sm text-muted-foreground">
            <p>{lead.qualification_status ?? "Unqualified"}</p>
            <p>{formatAdminDate(lead.created_at)}</p>
          </div>
        </div>
      )}
    />
  );
}
