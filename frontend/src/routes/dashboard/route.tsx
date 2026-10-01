import { createFileRoute, Link, Outlet, useNavigate } from "@tanstack/react-router";
import { logout } from "../../lib/auth";

export const Route = createFileRoute("/dashboard")({ component: DashboardLayout });

function DashboardLayout() {
  const navigate = useNavigate();
  const { user } = Route.useRouteContext();
  const isAdmin = user?.roles.some((role) => role === "Admin" || role === "Root");

  return (
    <main style={{ maxWidth: 1180, margin: "0 auto", padding: 24 }}>
      <nav style={{ display: "flex", gap: 18, alignItems: "center", marginBottom: 32 }}>
        <strong style={{ marginRight: "auto" }}>Cadence</strong>
        <Link to="/dashboard">Overview</Link>
        <Link to="/dashboard/bookings">Bookings</Link>
        <Link to="/dashboard/availability">Availability</Link>
        <Link to="/dashboard/bugs">Bugs</Link>
        {isAdmin ? <Link to="/dashboard/admin">Admin</Link> : null}
        <button
          className="button"
          onClick={async () => {
            await logout();
            await navigate({ to: "/login" });
          }}
        >
          Log out
        </button>
      </nav>
      <Outlet />
    </main>
  );
}
