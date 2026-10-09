import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays } from "lucide-react";
import { useMemo } from "react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { bookingColumns } from "../../domains/bookings/booking-columns";
import { hostSettingsQuery } from "../../domains/dashboard/host-api";
import { bookingsQuery, splitBookings } from "../../domains/bookings/bookings";
import { DataTable } from "../../domains/shared/DataTable";
import { EmptyState } from "../../domains/shared/EmptyState";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/bookings")({
  head: () => ({ meta: [{ title: `Bookings | ${APP_NAME}` }] }),
  component: BookingsPage,
});

const TABS = { UPCOMING: "upcoming", PAST: "past" } as const;

function BookingList({
  loading,
  bookings,
  empty,
}: {
  loading: boolean;
  bookings: Booking[];
  empty: React.ReactNode;
}) {
  const settings = useQuery(hostSettingsQuery);
  return (
    <Card className="gap-0 overflow-hidden py-0">
      {loading ? (
        <TableSkeleton />
      ) : bookings.length === 0 ? (
        empty
      ) : (
        <DataTable
          columns={bookingColumns(settings.data?.timezone)}
          rows={bookings}
          rowKey={(booking) => booking.unid}
        />
      )}
    </Card>
  );
}

function BookingsPage() {
  const query = useQuery(bookingsQuery);

  const { upcoming, past } = useMemo(
    () => splitBookings(query.data ?? [], Date.now()),
    [query.data],
  );

  return (
    <>
      <PageHeader
        title="Bookings"
        description="Calls people booked from your booking page."
      />
      {query.isError ? (
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
              {query.isPending ? null : (
                <span className="text-muted-foreground tabular-nums">
                  {upcoming.length}
                </span>
              )}
            </TabsTrigger>
            <TabsTrigger value={TABS.PAST}>
              Past
              {query.isPending ? null : (
                <span className="text-muted-foreground tabular-nums">{past.length}</span>
              )}
            </TabsTrigger>
          </TabsList>
          <TabsContent value={TABS.UPCOMING}>
            <BookingList
              loading={query.isPending}
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
              loading={query.isPending}
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
