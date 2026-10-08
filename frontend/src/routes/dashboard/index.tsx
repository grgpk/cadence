import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays, Clock, Copy, ExternalLink } from "lucide-react";
import { toast } from "sonner";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";
import { BOOKING_COLUMNS, isUpcoming } from "../../domains/bookings/booking-columns";
import { DataTable } from "../../domains/shared/DataTable";
import { EmptyState } from "../../domains/shared/EmptyState";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { getHostSettings } from "../../domains/widget/cadence-api";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";
import { bookingPath, ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/")({ component: DashboardHome });

const NEXT_UP_LIMIT = 5;

function copyBookingLink(slug: string) {
  const link = `${window.location.origin}${bookingPath(slug)}`;
  navigator.clipboard.writeText(link).then(
    () => toast.success("Link copied"),
    () => toast.error("The link could not be copied. Copy it from the address bar."),
  );
}

function DashboardHome() {
  const { user } = Route.useRouteContext();
  const bookings = useQuery({
    queryKey: QUERY_KEYS.bookings,
    queryFn: () => api<Booking[]>("/api/bookings"),
  });
  const availability = useQuery({
    queryKey: QUERY_KEYS.availability,
    queryFn: () => api<AvailabilityRule[]>("/api/availability"),
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
  const hasNoHours = availability.data?.length === 0;
  const firstName = user?.full_name.trim().split(/\s+/)[0];

  return (
    <>
      <PageHeader
        title={firstName ? `Welcome back, ${firstName}` : "Welcome back"}
        description="Your next calls and the page people book them from."
        actions={
          <>
            {slug ? (
              <Button variant="outline" asChild>
                <Link to={ROUTES.bookingPage} params={{ slug }} target="_blank">
                  <ExternalLink />
                  Open booking page
                </Link>
              </Button>
            ) : null}
            <Button disabled={!slug} onClick={() => slug && copyBookingLink(slug)}>
              <Copy />
              Copy link
            </Button>
          </>
        }
      />

      {hasNoHours ? (
        <Card className="border-warning/40">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Clock className="size-4 text-warning" />
              Nobody can book you yet
            </CardTitle>
            <CardDescription>
              You have no hours set, so your link has no times to offer.
            </CardDescription>
            <CardAction>
              <Button size="sm" asChild>
                <Link to={ROUTES.dashboardAvailability}>Set your hours</Link>
              </Button>
            </CardAction>
          </CardHeader>
        </Card>
      ) : null}

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
