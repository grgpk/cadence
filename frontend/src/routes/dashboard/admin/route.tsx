import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";

import { AdminSidenavLayout } from "../../../domains/admin/components/admin-sidenav";

export const Route = createFileRoute("/dashboard/admin")({
  beforeLoad: ({ context }) => {
    const isAdmin = context.user?.roles.some(
      (role) => role === "Admin" || role === "Root",
    );
    if (!isAdmin) {
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
