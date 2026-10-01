import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CalendarClock, CircleAlert, Inbox } from "lucide-react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

export const Route = createFileRoute("/dashboard/availability")({
  component: AvailabilityPage,
});

function AvailabilityPage() {
  const {
    data: rules = [],
    error,
    isPending,
  } = useQuery({
    queryKey: QUERY_KEYS.availability,
    queryFn: () => api<AvailabilityRule[]>("/api/availability"),
  });

  return (
    <section className="flex flex-1 flex-col gap-6">
      <header>
        <p className="text-sm font-medium text-muted-foreground">Workspace</p>
        <h1 className="text-2xl font-semibold tracking-tight">Availability</h1>
        <p className="text-muted-foreground">
          Set recurring windows for your booking widget.
        </p>
      </header>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CalendarClock className="size-5 text-muted-foreground" />
            Availability rules
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isPending ? (
            <div className="space-y-4">
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <CircleAlert className="size-4" />
              <AlertDescription>{error.message}</AlertDescription>
            </Alert>
          ) : rules.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-12 text-center text-muted-foreground">
              <Inbox className="size-8" />
              <p>No availability rules yet.</p>
            </div>
          ) : (
            <div className="divide-y">
              {rules.map((rule) => (
                <div
                  key={rule.id}
                  className="flex flex-wrap items-center justify-between gap-3 py-3 first:pt-0 last:pb-0"
                >
                  <span className="font-medium">Day {rule.weekday}</span>
                  <span className="text-sm text-muted-foreground">
                    {rule.start_time} to {rule.end_time} · {rule.slot_minutes} min
                  </span>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
