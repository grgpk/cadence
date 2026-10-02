import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import {
  ArrowRight,
  Bug,
  CalendarCheck,
  CalendarDays,
  CheckCircle2,
  Users,
  Video,
} from "lucide-react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";
import {
  connectGoogleCalendar,
  disconnectGoogleCalendar,
  getCalendarEvents,
} from "../../domains/widget/cadence-api";

export const Route = createFileRoute("/admin/")({
  component: AdminOverviewPage,
});

const CARDS = [
  {
    title: "Leads",
    description: "Review captured coaching leads.",
    to: "/admin/leads",
    icon: Users,
  },
  {
    title: "Bookings",
    description: "Review booked calls and statuses.",
    to: "/admin/bookings",
    icon: CalendarCheck,
  },
  {
    title: "Calling Visits",
    description: "Inspect call funnel visits.",
    to: "/admin/calling-visits",
    icon: Video,
  },
  {
    title: "Bug Reports",
    description: "Monitor application errors.",
    to: "/admin/bug-reports",
    icon: Bug,
  },
] as const;

function AdminOverviewPage() {
  const queryClient = useQueryClient();
  const calendarQuery = useQuery({
    queryKey: ["google-calendar-events"],
    queryFn: getCalendarEvents,
    retry: false,
  });
  const disconnect = useMutation({
    mutationFn: disconnectGoogleCalendar,
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["google-calendar-events"] }),
  });

  return (
    <section className="flex flex-1 flex-col gap-6 p-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Admin</h1>
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
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CalendarDays className="size-5 text-muted-foreground" />
            Google Calendar
          </CardTitle>
          <CardDescription>
            Required for live availability and Google Meet booking.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center gap-3">
          {calendarQuery.isPending ? (
            <span className="text-sm text-muted-foreground">Checking connection...</span>
          ) : calendarQuery.isError ? (
            <button
              type="button"
              className="inline-flex h-9 items-center justify-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground shadow-xs transition-colors hover:bg-primary/90"
              onClick={() => void connectGoogleCalendar()}
            >
              Connect Google Calendar
            </button>
          ) : (
            <>
              <span className="inline-flex items-center gap-2 rounded-md bg-green-100 px-3 py-2 text-sm font-medium text-green-700">
                <CheckCircle2 className="size-4" /> Connected
              </span>
              <span className="text-sm text-muted-foreground">
                {calendarQuery.data.length} events next 7 days
              </span>
              <button
                type="button"
                className="inline-flex h-9 items-center justify-center rounded-md border border-input bg-background px-4 text-sm font-medium shadow-xs transition-colors hover:bg-accent hover:text-accent-foreground"
                onClick={() => disconnect.mutate()}
                disabled={disconnect.isPending}
              >
                {disconnect.isPending ? "Disconnecting..." : "Disconnect"}
              </button>
            </>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
