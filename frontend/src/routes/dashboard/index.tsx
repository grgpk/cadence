import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays, Copy } from "lucide-react";
import { useState } from "react";

import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { Agenda } from "../../domains/bookings/agenda";
import { BookingDetailSheet } from "../../domains/bookings/booking-detail-sheet";
import { bookingsQuery, splitBookings } from "../../domains/bookings/bookings";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { BookingPageStatus } from "../../domains/dashboard/booking-page-status";
import { availabilityQuery, hostSettingsQuery } from "../../domains/dashboard/host-api";
import { TodayTape } from "../../domains/dashboard/today-tape";
import { EmptyState } from "../../domains/shared/empty-state";
import { ErrorState } from "../../domains/shared/error-state";
import { PageHeader } from "../../domains/shared/page-header";
import { TableSkeleton } from "../../domains/shared/table-skeleton";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/")({
  head: () => ({ meta: [{ title: `Overview | ${APP_NAME}` }] }),
  component: DashboardHome,
});

const NEXT_UP_LIMIT = 6;

function DashboardHome() {
  const { user } = Route.useRouteContext();
  const bookings = useQuery(bookingsQuery);
  const settings = useQuery(hostSettingsQuery);
  const availability = useQuery(availabilityQuery);
  const [openUnid, setOpenUnid] = useState<string | null>(null);
  // The link exists once the settings have loaded.
  const slug = settings.data?.public_slug;
  const timeZone = settings.data?.timezone;

  const nextUp = splitBookings(bookings.data ?? [], Date.now()).upcoming.slice(
    0,
    NEXT_UP_LIMIT,
  );
  const firstName = user?.full_name.trim().split(/\s+/)[0];

  return (
    <>
      <PageHeader
        title={firstName ? `Welcome back, ${firstName}` : "Welcome back"}
        description="Your day, your next calls and the page people book you from."
      />

      {/* Without its data the day strip has nothing true to show; the cards below report failures. */}
      {timeZone && availability.data && bookings.data ? (
        <TodayTape
          rules={availability.data}
          bookings={bookings.data}
          timeZone={timeZone}
        />
      ) : null}

      <Card className="gap-0 overflow-hidden py-0">
        <div className="flex items-center justify-between gap-4 border-b py-2.5 pr-3 pl-5">
          <h2 className="font-semibold">Next up</h2>
          <Button variant="ghost" size="sm" asChild>
            <Link to={ROUTES.dashboardBookings}>All bookings</Link>
          </Button>
        </div>
        {bookings.isPending || (settings.isPending && !bookings.isError) ? (
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
          <Agenda
            bookings={nextUp}
            timeZone={timeZone}
            onOpen={(booking) => setOpenUnid(booking.unid)}
          />
        )}
      </Card>

      <BookingPageStatus />

      <BookingDetailSheet
        unid={openUnid}
        timeZone={timeZone}
        onClose={() => setOpenUnid(null)}
      />
    </>
  );
}
