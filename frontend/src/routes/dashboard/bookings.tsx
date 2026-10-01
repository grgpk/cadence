import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CalendarDays, CircleAlert, Inbox } from "lucide-react";

import type { Booking } from "../../bindings/Booking";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

export const Route = createFileRoute("/dashboard/bookings")({ component: BookingsPage });

function BookingsPage() {
  const {
    data: bookings = [],
    error,
    isPending,
  } = useQuery({
    queryKey: QUERY_KEYS.bookings,
    queryFn: () => api<Booking[]>("/api/bookings"),
  });

  return (
    <section className="flex flex-1 flex-col gap-6">
      <header>
        <p className="text-sm font-medium text-muted-foreground">Workspace</p>
        <h1 className="text-2xl font-semibold tracking-tight">Bookings</h1>
        <p className="text-muted-foreground">Calls booked through your Cadence widget.</p>
      </header>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CalendarDays className="size-5 text-muted-foreground" />
            Upcoming and past calls
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isPending ? (
            <div className="space-y-4">
              <Skeleton className="h-14 w-full" />
              <Skeleton className="h-14 w-full" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <CircleAlert className="size-4" />
              <AlertDescription>{error.message}</AlertDescription>
            </Alert>
          ) : bookings.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-12 text-center text-muted-foreground">
              <Inbox className="size-8" />
              <p>No bookings yet.</p>
            </div>
          ) : (
            <div className="divide-y">
              {bookings.map((booking) => (
                <article
                  key={booking.unid}
                  className="flex flex-wrap items-center justify-between gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div>
                    <p className="font-medium">{booking.invitee_name}</p>
                    <p className="text-sm text-muted-foreground">
                      {booking.invitee_email}
                    </p>
                  </div>
                  <div className="text-right text-sm text-muted-foreground">
                    <p>{new Date(booking.slot_start).toLocaleString()}</p>
                    <p>{booking.status}</p>
                  </div>
                </article>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
