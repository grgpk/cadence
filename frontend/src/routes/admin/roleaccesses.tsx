import { createFileRoute } from "@tanstack/react-router";

import type { RoleAccess } from "../../bindings/RoleAccess";
import { Badge } from "../../components/ui/badge";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import type { Column } from "../../domains/shared/DataTable";

export const Route = createFileRoute("/admin/roleaccesses")({
  component: AdminRoleAccessesPage,
});

const COLUMNS: Column<RoleAccess>[] = [
  {
    header: "User ID",
    className: "tabular-nums",
    cell: (access) => access.grantedto_unid,
  },
  {
    header: "Role",
    className: "text-right",
    cell: (access) => <Badge variant="secondary">{access.role}</Badge>,
  },
];

function AdminRoleAccessesPage() {
  return (
    <AdminDataPage<RoleAccess>
      {...ADMIN_PAGES.roleAccesses}
      endpoint="/api/admin/roleaccesses"
      emptyTitle="No roles granted"
      emptyDescription="Roles given to users are listed here."
      rowKey={(access) => access.unid}
      columns={COLUMNS}
    />
  );
}
