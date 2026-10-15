import { createFileRoute, Outlet } from "@tanstack/react-router";

import { DASHBOARD_NAV } from "../../domains/dashboard/routes";
import { AppShell } from "../../domains/shared/app-shell";
import { NOINDEX_META } from "../../lib/config";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard")({
  head: () => ({ meta: [NOINDEX_META] }),
  component: DashboardLayout,
});

function DashboardLayout() {
  const { user } = Route.useRouteContext();
  if (!user) {
    return null;
  }

  return (
    <AppShell
      user={user}
      home={ROUTES.dashboard}
      area="Dashboard"
      sections={DASHBOARD_NAV}
    >
      <Outlet />
    </AppShell>
  );
}
