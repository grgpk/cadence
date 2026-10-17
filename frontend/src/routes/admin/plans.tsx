import { createFileRoute } from "@tanstack/react-router";

import type { AdminPlan } from "../../bindings/AdminPlan";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { planCounts, STATUS } from "../../domains/billing/billing";
import { CancellationReasons } from "../../domains/billing/cancellation-reasons";
import { CompMenu } from "../../domains/billing/comp-menu";
import { PLAN_NAMES } from "../../domains/pricing/pricing";
import { TwoLine, type Column } from "../../domains/shared/data-table";
import { StatusBadge, SUBSCRIPTION_STATUS } from "../../domains/shared/status-badge";
import { APP_NAME } from "../../lib/config";
import { formatDate } from "../../lib/format";
import { searchFromUrl } from "../../lib/search";

export const Route = createFileRoute("/admin/plans")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.plans.title} | ${APP_NAME}` }] }),
  validateSearch: searchFromUrl,
  component: AdminPlansPage,
});

/** The date that matters next: a trial's end, a renewal, or a cancellation. */
function nextDate(row: AdminPlan): string {
  if (row.status === STATUS.TRIALING && row.trial_ends_at) {
    return `Trial ends ${formatDate(row.trial_ends_at)}`;
  }
  if (
    row.current_period_end &&
    (row.status === STATUS.ACTIVE || row.status === STATUS.PAST_DUE)
  ) {
    const verb = row.cancel_at_period_end ? "Ends" : "Renews";
    return `${verb} ${formatDate(row.current_period_end)}`;
  }
  return "";
}

const COLUMNS: Column<AdminPlan>[] = [
  {
    header: "Host",
    cell: (row) => <TwoLine primary={row.full_name} secondary={row.email} />,
  },
  {
    header: "Plan",
    cell: (row) => (
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{PLAN_NAMES[row.plan]}</span>
        {row.status ? (
          <StatusBadge value={row.status} statuses={SUBSCRIPTION_STATUS} />
        ) : null}
      </div>
    ),
  },
  {
    header: "Next",
    className: "text-muted-foreground text-sm",
    cell: nextDate,
  },
  {
    header: "Actions",
    className: "text-right",
    cell: (row) => <CompMenu row={row} endpoint={ADMIN_PAGES.plans.endpoint} />,
  },
];

function PlanSummary({ rows }: { rows: AdminPlan[] }) {
  const counts = planCounts(rows);
  return (
    <dl className="grid grid-cols-3 gap-3">
      {(Object.keys(counts) as (keyof typeof counts)[]).map((plan) => (
        <div key={plan} className="rounded-xl border bg-card px-4 py-3">
          <dt className="text-sm text-muted-foreground">{PLAN_NAMES[plan]}</dt>
          <dd className="text-clock text-2xl font-semibold">{counts[plan]}</dd>
        </div>
      ))}
    </dl>
  );
}

function AdminPlansPage() {
  const { q } = Route.useSearch();
  return (
    <>
      <AdminDataPage<AdminPlan>
        {...ADMIN_PAGES.plans}
        emptyTitle="No hosts yet"
        emptyDescription="A host appears as soon as someone signs up."
        rowKey={(row) => row.user_unid}
        columns={COLUMNS}
        initialSearch={q}
        summary={(rows) => <PlanSummary rows={rows} />}
        searchText={(row) =>
          [row.full_name, row.email, PLAN_NAMES[row.plan], row.status ?? ""].join(" ")
        }
      />
      <CancellationReasons />
    </>
  );
}
