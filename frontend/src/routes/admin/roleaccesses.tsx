import { createFileRoute } from "@tanstack/react-router";

import type { AdminRoleAccess } from "../../bindings/AdminRoleAccess";
import { Badge } from "../../components/ui/badge";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { TwoLine, type Column } from "../../domains/shared/DataTable";
import { APP_NAME } from "../../lib/config";

export const Route = createFileRoute("/admin/roleaccesses")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.roleAccesses.title} | ${APP_NAME}` }] }),
  component: AdminRoleAccessesPage,
});

const COLUMNS: Column<AdminRoleAccess>[] = [
  {
    header: "Person",
    cell: (access) => <TwoLine primary={access.full_name} secondary={access.email} />,
  },
  {
    header: "Role",
    className: "text-right",
    cell: (access) => <Badge variant="secondary">{access.role}</Badge>,
  },
];

function AdminRoleAccessesPage() {
  return (
    <AdminDataPage<AdminRoleAccess>
      {...ADMIN_PAGES.roleAccesses}
      emptyTitle="No roles granted"
      emptyDescription="Roles given to users are listed here."
      rowKey={(access) => access.unid}
      columns={COLUMNS}
      searchText={(access) => `${access.full_name} ${access.email} ${access.role}`}
    />
  );
}
