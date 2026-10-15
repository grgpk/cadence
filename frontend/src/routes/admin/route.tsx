import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";

import { ADMIN_NAV } from "../../domains/admin/routes";
import { AppShell } from "../../domains/shared/app-shell";
import { hasAdminAccess } from "../../lib/auth";
import { NOINDEX_META } from "../../lib/config";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/admin")({
  beforeLoad: ({ context }) => {
    if (!hasAdminAccess(context.user)) {
      throw redirect({ to: ROUTES.dashboard });
    }
  },
  head: () => ({ meta: [NOINDEX_META] }),
  component: AdminLayout,
});

function AdminLayout() {
  const { user } = Route.useRouteContext();
  if (!user) {
    return null;
  }

  return (
    <AppShell user={user} home={ROUTES.admin} area="Admin" sections={ADMIN_NAV} wide>
      <Outlet />
    </AppShell>
  );
}
