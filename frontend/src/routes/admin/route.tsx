import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";

import { ADMIN_NAV } from "../../domains/admin/routes";
import { AppShell } from "../../domains/shared/AppShell";
import { hasAdminAccess } from "../../lib/auth";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/admin")({
  beforeLoad: ({ context }) => {
    if (!hasAdminAccess(context.user)) {
      throw redirect({ to: ROUTES.dashboard });
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
    <AppShell user={user} home={ROUTES.admin} area="Admin" sections={ADMIN_NAV}>
      <Outlet />
    </AppShell>
  );
}
