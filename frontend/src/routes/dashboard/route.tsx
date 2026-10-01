import { createFileRoute, Outlet } from "@tanstack/react-router";

import { DashboardSidenavLayout } from "../../domains/dashboard/components/dashboard-sidenav";

export const Route = createFileRoute("/dashboard")({ component: DashboardLayout });

function DashboardLayout() {
  const { user } = Route.useRouteContext();
  if (!user) {
    return null;
  }

  return (
    <DashboardSidenavLayout user={user}>
      <Outlet />
    </DashboardSidenavLayout>
  );
}
