import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";

import { AdminSidenavLayout } from "../../domains/admin/components/admin-sidenav";
import { hasAdminAccess } from "../../lib/auth";

export const Route = createFileRoute("/admin")({
  beforeLoad: ({ context }) => {
    if (!hasAdminAccess(context.user)) {
      throw redirect({ to: "/dashboard" });
    }
  },
  component: AdminLayout,
});

function AdminLayout() {
  const { user } = Route.useRouteContext();
  if (!user) {
    return null;
  }

  return (
    <AdminSidenavLayout user={user}>
      <Outlet />
    </AdminSidenavLayout>
  );
}
