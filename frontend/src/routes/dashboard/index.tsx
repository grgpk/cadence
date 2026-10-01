import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/dashboard/")({ component: DashboardHome });
function DashboardHome() { const { user } = Route.useRouteContext(); return <section><p style={{ color: "#737373" }}>Dashboard</p><h1>Welcome, {user?.full_name}</h1><div className="card"><h2>Booking widget</h2><p>Configure availability, share public widget, manage calls.</p></div></section>; }
