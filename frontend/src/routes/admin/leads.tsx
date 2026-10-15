import { createFileRoute } from "@tanstack/react-router";

import type { Lead } from "../../bindings/Lead";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { DetailSheet } from "../../domains/admin/components/detail-sheet";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/data-table";
import { LEAD_STATUS, StatusBadge } from "../../domains/shared/status-badge";
import { APP_NAME } from "../../lib/config";
import { formatDateTime } from "../../lib/format";

export const Route = createFileRoute("/admin/leads")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.leads.title} | ${APP_NAME}` }] }),
  component: AdminLeadsPage,
});

/** The widget stores the dial code and the national digits apart; older rows hold a full number. */
function formatPhone(lead: Lead): string {
  const phone = lead.phone ?? "";
  const isDialCode = /^\d+$/.test(lead.country_code ?? "");
  return isDialCode && !phone.startsWith("+") ? `+${lead.country_code} ${phone}` : phone;
}

function leadName(lead: Lead): string {
  return [lead.first_name, lead.last_name].filter(Boolean).join(" ") || "No name yet";
}

const COLUMNS: Column<Lead>[] = [
  {
    header: "Lead",
    cell: (lead) => (
      <TwoLine
        primary={
          <DetailSheet
            trigger={leadName(lead)}
            title={leadName(lead)}
            description={lead.email ?? "No email yet"}
            fields={[
              { label: "Phone", value: lead.phone ? formatPhone(lead) : null },
              { label: "Working on", value: lead.currently_working_on },
              { label: "What is stopping them", value: lead.what_stopping_you },
              { label: "Investment comfort", value: lead.investment_comfort },
              { label: "Urgency", value: lead.urgency_level },
              { label: "Heard about us from", value: lead.how_heard_about_us },
              { label: "Came from page", value: lead.source_page },
              {
                label: "Campaign",
                value:
                  [lead.utm_source, lead.utm_medium, lead.utm_campaign]
                    .filter(Boolean)
                    .join(", ") || null,
              },
              { label: "Created", value: formatDateTime(lead.created_at) },
              { label: "Last changed", value: formatDateTime(lead.updated_at) },
            ]}
          />
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
    className: "text-clock text-[0.9375rem]",
    cell: (lead) =>
      lead.form_submitted_at ? (
        formatDateTime(lead.form_submitted_at)
      ) : (
        <span className="text-muted-foreground">Not sent</span>
      ),
  },
  {
    header: "Created",
    className: "text-muted-foreground text-clock text-[0.9375rem]",
    cell: (lead) => formatDateTime(lead.created_at),
  },
];

function AdminLeadsPage() {
  return (
    <AdminDataPage<Lead>
      {...ADMIN_PAGES.leads}
      emptyTitle="No leads yet"
      emptyDescription="A lead appears as soon as someone starts the booking form."
      rowKey={(lead) => lead.unid}
      columns={COLUMNS}
      searchText={(lead) =>
        [leadName(lead), lead.email, lead.phone, lead.qualification_status]
          .filter(Boolean)
          .join(" ")
      }
    />
  );
}
