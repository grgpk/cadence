import { createFileRoute, Link } from "@tanstack/react-router";

import type { AdminHost } from "../../bindings/AdminHost";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { hostGaps } from "../../domains/admin/host-setup";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/DataTable";
import { StatusBadge, USER_STATUS } from "../../domains/shared/StatusBadge";
import { APP_NAME } from "../../lib/config";
import { formatDateTime } from "../../lib/format";
import { bookingPath, ROUTES } from "../../lib/routes";
import { searchFromUrl } from "../../lib/search";

export const Route = createFileRoute("/admin/hosts")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.hosts.title} | ${APP_NAME}` }] }),
  validateSearch: searchFromUrl,
  component: AdminHostsPage,
});

const READY = "Ready";
const SETUP_STATUS = { [READY]: { label: READY, tone: "positive" } } as const;

const COLUMNS: Column<AdminHost>[] = [
  {
    header: "Host",
    cell: (host) => <TwoLine primary={host.full_name} secondary={host.email} />,
  },
  {
    header: "Booking page",
    cell: (host) =>
      host.public_slug ? (
        <Link
          to={ROUTES.bookingPage}
          params={{ slug: host.public_slug }}
          target="_blank"
          className="rounded-sm underline-offset-4 outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50"
        >
          {bookingPath(host.public_slug)}
        </Link>
      ) : (
        <span className="text-muted-foreground">None yet</span>
      ),
  },
  {
    header: "Setup",
    cell: (host) => {
      const gaps = hostGaps(host);
      return (
        <div className="flex flex-wrap gap-1.5">
          {gaps.length === 0 ? (
            <StatusBadge value={READY} statuses={SETUP_STATUS} />
          ) : (
            gaps.map((gap) => (
              <StatusBadge
                key={gap}
                value={gap}
                statuses={{ [gap]: { label: gap, tone: "caution" } }}
              />
            ))
          )}
        </div>
      );
    },
  },
  {
    header: "Account",
    cell: (host) => <StatusBadge value={host.status} statuses={USER_STATUS} />,
  },
  {
    header: "Joined",
    className: "text-muted-foreground tabular-nums",
    cell: (host) => formatDateTime(host.created_at),
  },
];

function AdminHostsPage() {
  const { q } = Route.useSearch();
  return (
    <AdminDataPage<AdminHost>
      {...ADMIN_PAGES.hosts}
      emptyTitle="No hosts yet"
      emptyDescription="A host appears as soon as someone signs up."
      rowKey={(host) => host.unid}
      columns={COLUMNS}
      initialSearch={q}
      searchText={(host) =>
        [
          host.full_name,
          host.email,
          host.public_slug,
          host.status,
          ...hostGaps(host),
          hostGaps(host).length === 0 ? READY : "",
        ].join(" ")
      }
    />
  );
}
