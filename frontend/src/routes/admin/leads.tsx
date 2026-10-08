import { createFileRoute } from "@tanstack/react-router";

import type { Lead } from "../../bindings/Lead";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/DataTable";
import { LEAD_STATUS, StatusBadge } from "../../domains/shared/StatusBadge";
import { formatDateTime } from "../../lib/format";

export const Route = createFileRoute("/admin/leads")({
  component: AdminLeadsPage,
});

/** The widget stores the dial code and the national digits apart; older rows hold a full number. */
function formatPhone(lead: Lead): string {
  const phone = lead.phone ?? "";
  const isDialCode = /^\d+$/.test(lead.country_code ?? "");
  return isDialCode && !phone.startsWith("+") ? `+${lead.country_code} ${phone}` : phone;
}

const COLUMNS: Column<Lead>[] = [
  {
    header: "Lead",
    cell: (lead) => (
      <TwoLine
        primary={
          [lead.first_name, lead.last_name].filter(Boolean).join(" ") || "No name yet"
        }
        secondary={lead.email ?? "No email yet"}
      />
    ),
  },
  {
    header: "Phone",
    className: "tabular-nums",
    cell: (lead) =>
      lead.phone ? (
        formatPhone(lead)
      ) : (
        <span className="text-muted-foreground">None</span>
      ),
  },
  {
    header: "Qualification",
    cell: (lead) => (
      <StatusBadge
        value={lead.qualification_status}
        statuses={LEAD_STATUS}
        fallback="Not assessed"
      />
    ),
  },
  {
    header: "Form sent",
    className: "tabular-nums",
    cell: (lead) =>
      lead.form_submitted_at ? (
        formatDateTime(lead.form_submitted_at)
      ) : (
        <span className="text-muted-foreground">Not sent</span>
      ),
  },
  {
    header: "Created",
    className: "text-muted-foreground tabular-nums",
    cell: (lead) => formatDateTime(lead.created_at),
  },
];

function AdminLeadsPage() {
  return (
    <AdminDataPage<Lead>
      {...ADMIN_PAGES.leads}
      endpoint="/api/admin/leads"
      emptyTitle="No leads yet"
      emptyDescription="A lead appears as soon as someone starts the booking form."
      rowKey={(lead) => lead.unid}
      columns={COLUMNS}
    />
  );
}
