import { createFileRoute, Link, Outlet, useNavigate } from "@tanstack/react-router";
import { Button } from "../../components/ui/button";
import { hasAdminAccess, logout } from "../../lib/auth";

export const Route = createFileRoute("/dashboard")({ component: DashboardLayout });

function DashboardLayout() {
  const navigate = useNavigate();
  const { user } = Route.useRouteContext();
  const isAdmin = hasAdminAccess(user);

  return (
    <main className="min-h-svh bg-background">
      <nav className="mx-auto flex max-w-7xl items-center gap-5 border-b px-6 py-4">
        <strong className="mr-auto text-lg">Cadence</strong>
        <Link to="/dashboard" className="text-sm font-medium">
          Overview
        </Link>
        <Link to="/dashboard/bookings" className="text-sm font-medium">
          Bookings
        </Link>
        <Link to="/dashboard/availability" className="text-sm font-medium">
          Availability
        </Link>
        <Link to="/dashboard/bugs" className="text-sm font-medium">
          Bugs
        </Link>
        {isAdmin ? (
          <Link to="/admin" className="text-sm font-medium">
            Admin
          </Link>
        ) : null}
        <Button
          variant="outline"
          size="sm"
          onClick={async () => {
            await logout();
            await navigate({ to: "/login" });
          }}
        >
          Log out
        </Button>
      </nav>
      <div className="mx-auto max-w-7xl px-6 py-8">
        <Outlet />
      </div>
    </main>
  );
}
