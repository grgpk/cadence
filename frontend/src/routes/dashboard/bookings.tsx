import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays } from "lucide-react";
import { useMemo } from "react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { BOOKING_COLUMNS, isUpcoming } from "../../domains/bookings/booking-columns";
import { DataTable } from "../../domains/shared/DataTable";
import { EmptyState } from "../../domains/shared/EmptyState";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { api } from "../../lib/api";
import { APP_NAME } from "../../lib/config";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/bookings")({
  head: () => ({ meta: [{ title: `Bookings | ${APP_NAME}` }] }),
  component: BookingsPage,
});

const TABS = { UPCOMING: "upcoming", PAST: "past" } as const;

function BookingList({
  bookings,
  empty,
}: {
  bookings: Booking[];
  empty: React.ReactNode;
}) {
  return (
    <Card className="gap-0 overflow-hidden py-0">
      {bookings.length === 0 ? (
        empty
      ) : (
        <DataTable
          columns={BOOKING_COLUMNS}
          rows={bookings}
          rowKey={(booking) => booking.unid}
        />
      )}
    </Card>
  );
}

function BookingsPage() {
  const query = useQuery({
    queryKey: QUERY_KEYS.bookings,
    queryFn: () => api<Booking[]>("/api/bookings"),
  });

  const { upcoming, past } = useMemo(() => {
    const now = Date.now();
    const all = query.data ?? [];
    return {
      // Soonest first for what is coming, most recent first for what has been.
      upcoming: all
        .filter((booking) => isUpcoming(booking, now))
        .sort((a, b) => a.slot_start.localeCompare(b.slot_start)),
      past: all
        .filter((booking) => !isUpcoming(booking, now))
        .sort((a, b) => b.slot_start.localeCompare(a.slot_start)),
    };
  }, [query.data]);

  return (
    <>
      <PageHeader
        title="Bookings"
        description="Calls people booked from your booking page."
      />
      {query.isPending ? (
        <Card className="gap-0 overflow-hidden py-0">
          <TableSkeleton />
        </Card>
      ) : query.isError ? (
        <ErrorState
          title="Bookings did not load"
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <Tabs defaultValue={TABS.UPCOMING}>
          <TabsList>
            <TabsTrigger value={TABS.UPCOMING}>
              Upcoming
              <span className="text-muted-foreground tabular-nums">
                {upcoming.length}
              </span>
            </TabsTrigger>
            <TabsTrigger value={TABS.PAST}>
              Past
              <span className="text-muted-foreground tabular-nums">{past.length}</span>
            </TabsTrigger>
          </TabsList>
          <TabsContent value={TABS.UPCOMING}>
            <BookingList
              bookings={upcoming}
              empty={
                <EmptyState
                  icon={CalendarDays}
                  title="No upcoming calls"
                  description="New bookings appear here as soon as someone picks a time."
                  action={
                    <Button variant="outline" size="sm" asChild>
                      <Link to={ROUTES.dashboardAvailability}>Check your hours</Link>
                    </Button>
                  }
                />
              }
            />
          </TabsContent>
          <TabsContent value={TABS.PAST}>
            <BookingList
              bookings={past}
              empty={
                <EmptyState
                  icon={CalendarDays}
                  title="No past calls"
                  description="Calls move here once their start time has passed."
                />
              }
            />
          </TabsContent>
        </Tabs>
      )}
    </>
  );
}
