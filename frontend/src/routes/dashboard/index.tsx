import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowRight, CalendarDays, Settings2 } from "lucide-react";

import { Button } from "../../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";

export const Route = createFileRoute("/dashboard/")({ component: DashboardHome });

function DashboardHome() {
  const { user } = Route.useRouteContext();

  return (
    <section className="flex flex-1 flex-col gap-6">
      <header>
        <p className="text-sm font-medium text-muted-foreground">Dashboard</p>
        <h1 className="text-2xl font-semibold tracking-tight">
          Welcome, {user?.full_name}
        </h1>
        <p className="text-muted-foreground">
          Manage your booking cadence from one place.
        </p>
      </header>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CalendarDays className="size-5 text-muted-foreground" />
            Booking widget
          </CardTitle>
          <CardDescription>
            Configure availability, share public widget, manage calls.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-3">
          <Button asChild>
            <Link to="/dashboard/bookings">
              View bookings <ArrowRight className="size-4" />
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link to="/dashboard/availability">
              <Settings2 className="size-4" />
              Configure availability
            </Link>
          </Button>
        </CardContent>
      </Card>
    </section>
  );
}
