import { createFileRoute } from "@tanstack/react-router";

import type { AdminEmail } from "../../bindings/AdminEmail";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/data-table";
import { EMAIL_STATUS, StatusBadge } from "../../domains/shared/status-badge";
import { APP_NAME } from "../../lib/config";
import { formatDateTime } from "../../lib/format";
import { searchFromUrl } from "../../lib/search";

export const Route = createFileRoute("/admin/emails")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.emails.title} | ${APP_NAME}` }] }),
  validateSearch: searchFromUrl,
  component: AdminEmailsPage,
});

/** "booking_confirmation" reads as "Booking confirmation". */
function templateName(template: string): string {
  const words = template.replaceAll("_", " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

const COLUMNS: Column<AdminEmail>[] = [
  {
    header: "Email",
    cell: (email) => (
      <TwoLine primary={templateName(email.template)} secondary={email.email} />
    ),
  },
  {
    header: "State",
    cell: (email) => <StatusBadge value={email.state} statuses={EMAIL_STATUS} />,
  },
  {
    header: "Tries",
    className: "text-right tabular-nums",
    cell: (email) => email.attempts,
  },
  {
    header: "Last error",
    className: "max-w-sm",
    cell: (email) =>
      email.last_error ? (
        <p className="truncate" title={email.last_error}>
          {email.last_error}
        </p>
      ) : (
        <span className="text-muted-foreground">None</span>
      ),
  },
  {
    header: "Sent or due",
    className: "text-muted-foreground text-clock text-md",
    cell: (email) => formatDateTime(email.sent_at ?? email.send_at),
  },
];

function AdminEmailsPage() {
  const { q } = Route.useSearch();
  return (
    <AdminDataPage<AdminEmail>
      {...ADMIN_PAGES.emails}
      description="Confirmations and reminders, sent and waiting. The 200 most recent, unsent first."
      emptyTitle="No emails yet"
      emptyDescription="A confirmation and its reminders are queued when someone books a call."
      rowKey={(email) => email.id}
      columns={COLUMNS}
      initialSearch={q}
      searchText={(email) =>
        `${email.email} ${templateName(email.template)} ${email.state} ${email.last_error ?? ""}`
      }
    />
  );
}
