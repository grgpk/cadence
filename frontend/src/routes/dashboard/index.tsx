import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays, Copy } from "lucide-react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { BookingPageStatus } from "../../domains/dashboard/BookingPageStatus";
import { BOOKING_COLUMNS, isUpcoming } from "../../domains/bookings/booking-columns";
import { DataTable } from "../../domains/shared/DataTable";
import { EmptyState } from "../../domains/shared/EmptyState";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { getHostSettings } from "../../domains/widget/cadence-api";
import { api } from "../../lib/api";
import { APP_NAME } from "../../lib/config";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/")({
  head: () => ({ meta: [{ title: `Overview | ${APP_NAME}` }] }),
  component: DashboardHome,
});

const NEXT_UP_LIMIT = 5;

function DashboardHome() {
  const { user } = Route.useRouteContext();
  const bookings = useQuery({
    queryKey: QUERY_KEYS.bookings,
    queryFn: () => api<Booking[]>("/api/bookings"),
  });
  const settings = useQuery({
    queryKey: QUERY_KEYS.hostSettings,
    queryFn: getHostSettings,
  });
  // The link exists once the settings have loaded.
  const slug = settings.data?.public_slug;

  const now = Date.now();
  const nextUp = (bookings.data ?? [])
    .filter((booking) => isUpcoming(booking, now))
    .sort((a, b) => a.slot_start.localeCompare(b.slot_start))
    .slice(0, NEXT_UP_LIMIT);
  const firstName = user?.full_name.trim().split(/\s+/)[0];

  return (
    <>
      <PageHeader
        title={firstName ? `Welcome back, ${firstName}` : "Welcome back"}
        description="The page people book you from, and your next calls."
      />

      <BookingPageStatus />

      <Card className="gap-0 overflow-hidden py-0">
        <div className="flex items-center justify-between gap-4 border-b py-3 pr-3 pl-5">
          <h2 className="font-semibold">Next up</h2>
          <Button variant="ghost" size="sm" asChild>
            <Link to={ROUTES.dashboardBookings}>All bookings</Link>
          </Button>
        </div>
        {bookings.isPending ? (
          <TableSkeleton rows={3} />
        ) : bookings.isError ? (
          <CardContent className="py-5">
            <ErrorState
              title="Bookings did not load"
              error={bookings.error}
              onRetry={() => void bookings.refetch()}
            />
          </CardContent>
        ) : nextUp.length === 0 ? (
          <EmptyState
            icon={CalendarDays}
            title="No upcoming calls"
            description="Share your booking page and new calls will show up here."
            action={
              <Button
                variant="outline"
                size="sm"
                disabled={!slug}
                onClick={() => slug && copyBookingLink(slug)}
              >
                <Copy />
                Copy link
              </Button>
            }
          />
        ) : (
          <DataTable
            columns={BOOKING_COLUMNS}
            rows={nextUp}
            rowKey={(booking) => booking.unid}
          />
        )}
      </Card>
    </>
  );
}
