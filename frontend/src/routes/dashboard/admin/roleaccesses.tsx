import { createFileRoute } from "@tanstack/react-router";

import type { RoleAccess } from "../../../bindings/RoleAccess";
import { AdminDataPage } from "../../../domains/admin/components/admin-data-page";

export const Route = createFileRoute("/dashboard/admin/roleaccesses")({
  component: AdminRoleAccessesPage,
});

function AdminRoleAccessesPage() {
  return (
    <AdminDataPage<RoleAccess>
      title="Role Accesses"
      description="Users and roles granted in Cadence."
      endpoint="/api/admin/roleaccesses"
      emptyMessage="No role accesses found."
      itemKey={(access) => access.unid}
      renderItem={(access) => (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-4">
          <p className="font-mono text-sm">{access.grantedto_unid}</p>
          <p className="text-sm font-medium">{access.role}</p>
        </div>
      )}
    />
  );
}
