import { Link, createFileRoute } from "@tanstack/react-router";
import { ArrowRight, Bug, CalendarCheck, Users, Video } from "lucide-react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../../components/ui/card";

export const Route = createFileRoute("/dashboard/admin/")({
  component: AdminOverviewPage,
});

const CARDS = [
  {
    title: "Leads",
    description: "Review captured coaching leads.",
    to: "/dashboard/admin/leads",
    icon: Users,
  },
  {
    title: "Bookings",
    description: "Review booked calls and statuses.",
    to: "/dashboard/admin/bookings",
    icon: CalendarCheck,
  },
  {
    title: "Calling Visits",
    description: "Inspect call funnel visits.",
    to: "/dashboard/admin/calling-visits",
    icon: Video,
  },
  {
    title: "Bug Reports",
    description: "Monitor application errors.",
    to: "/dashboard/admin/bug-reports",
    icon: Bug,
  },
] as const;

function AdminOverviewPage() {
  return (
    <section className="flex flex-1 flex-col gap-6 p-6">
      <header>
        <p className="text-sm font-medium text-muted-foreground">Cadence</p>
        <h1 className="text-2xl font-semibold tracking-tight">Admin Overview</h1>
        <p className="text-muted-foreground">
          Manage leads, calls, access, and application health.
        </p>
      </header>
      <div className="grid gap-4 md:grid-cols-2">
        {CARDS.map(({ title, description, to, icon: Icon }) => (
          <Link key={to} to={to} className="group">
            <Card className="h-full transition-colors group-hover:border-primary">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle>{title}</CardTitle>
                  <Icon className="size-5 text-muted-foreground" />
                </div>
                <CardDescription>{description}</CardDescription>
              </CardHeader>
              <CardContent>
                <span className="inline-flex items-center gap-2 text-sm font-medium">
                  Open section <ArrowRight className="size-4" />
                </span>
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>
    </section>
  );
}
