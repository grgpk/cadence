import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays, Copy, ExternalLink } from "lucide-react";
import { cn } from "cn";
import { useState } from "react";

import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { Agenda } from "../../domains/bookings/agenda";
import { BookingDetailSheet } from "../../domains/bookings/booking-detail-sheet";
import { bookingsQuery, dayPlan } from "../../domains/bookings/bookings";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { SetupProblems, SetupSummary } from "../../domains/dashboard/booking-page-status";
import {
  availabilityOverridesQuery,
  availabilityQuery,
  hostSettingsQuery,
} from "../../domains/dashboard/host-api";
import { greeting } from "../../domains/dashboard/greeting";
import { InsightsCard } from "../../domains/dashboard/insights-card";
import { SwitchingCard } from "../../domains/dashboard/switching-card";
import { TodayCard, TodayCardSkeleton } from "../../domains/dashboard/today-card";
import { billingQuery } from "../../domains/billing/billing-api";
import { useClockPreference } from "../../hooks/use-clock-preference";
import { useNow } from "../../hooks/use-now";
import { PHASE_STYLES } from "../../domains/shared/day-phase-styles";
import { EmptyState } from "../../domains/shared/empty-state";
import { ErrorState } from "../../domains/shared/error-state";
import { PageHeader } from "../../domains/shared/page-header";
import { PanelHeader } from "../../domains/shared/panel-card";
import { TableSkeleton } from "../../domains/shared/table-skeleton";
import { APP_NAME } from "../../lib/config";
import { bookingPath, ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/")({
  head: () => ({ meta: [{ title: `Overview | ${APP_NAME}` }] }),
  component: DashboardHome,
});

const LATER_LIMIT = 6;

/** The link people book from, with a way to copy it and to see it as they do. */
function BookingLink({ slug }: { slug: string | undefined }) {
  if (!slug) return <Skeleton className="h-8 w-72" />;
  return (
    <>
      <span className="max-w-64 truncate text-sm text-muted-foreground">
        {window.location.host}
        {bookingPath(slug)}
      </span>
      <Button variant="outline" size="sm" onClick={() => copyBookingLink(slug)}>
        <Copy />
        Copy link
      </Button>
      <Button variant="outline" size="sm" asChild>
        <Link to={ROUTES.bookingPage} params={{ slug }} target="_blank">
          <ExternalLink />
          Open
        </Link>
      </Button>
    </>
  );
}

function DashboardHome() {
  const { user } = Route.useRouteContext();
  const bookings = useQuery(bookingsQuery);
  const settings = useQuery(hostSettingsQuery);
  const availability = useQuery(availabilityQuery);
  const overrides = useQuery(availabilityOverridesQuery);
  const billing = useQuery(billingQuery);
  const [hour12] = useClockPreference();
  const [openUnid, setOpenUnid] = useState<string | null>(null);
  const slug = settings.data?.public_slug;
  const timeZone = settings.data?.timezone;
  const openBooking = (booking: { unid: string }) => setOpenUnid(booking.unid);

  const later = timeZone
    ? dayPlan(bookings.data ?? [], timeZone, Date.now()).later.slice(0, LATER_LIMIT)
    : [];
  const now = useNow();
  const firstName = user?.full_name.trim().split(/\s+/)[0];
  const hello = now && timeZone ? greeting(firstName, now, timeZone) : null;
  // Calls are read on the host's clock, so they need the settings too.
  const failed = bookings.isError ? bookings : settings.isError ? settings : null;
  const loading = !failed && (bookings.isPending || settings.isPending);

  return (
    <>
      <PageHeader
        title={hello?.text ?? (firstName ? `Welcome back, ${firstName}` : "Welcome back")}
        accent={
          hello ? (
            <span
              aria-hidden="true"
              className={cn(
                "mt-2 block h-1 w-10 rounded-full",
                PHASE_STYLES[hello.phase].rail,
              )}
            />
          ) : null
        }
        actions={<BookingLink slug={slug} />}
      />

      <SetupProblems />

      {bookings.data?.length === 0 ? <SwitchingCard /> : null}

      {failed ? (
        <Card className="py-5">
          <CardContent>
            <ErrorState
              title="Your calls did not load"
              error={failed.error}
              onRetry={() => void failed.refetch()}
            />
          </CardContent>
        </Card>
      ) : loading || !timeZone || !bookings.data ? (
        <TodayCardSkeleton />
      ) : (
        <TodayCard
          bookings={bookings.data}
          rules={availability.data}
          overrides={overrides.data}
          timeZone={timeZone}
          onOpen={openBooking}
        />
      )}

      {failed ? null : (
        <Card className="gap-0 overflow-hidden py-0">
          <PanelHeader
            title="Later"
            action={
              <Button variant="ghost" size="sm" asChild>
                <Link to={ROUTES.dashboardBookings}>All bookings</Link>
              </Button>
            }
          />
          {loading ? (
            <TableSkeleton rows={2} />
          ) : later.length === 0 ? (
            <EmptyState
              icon={CalendarDays}
              title="Nothing booked after today"
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
              bookings={later}
              timeZone={timeZone}
              onOpen={openBooking}
              tracksAttendance={billing.data?.entitlements.attendance}
            />
          )}
        </Card>
      )}

      {billing.data?.entitlements.insights ? <InsightsCard hour12={hour12} /> : null}

      <SetupSummary />

      <BookingDetailSheet
        unid={openUnid}
        timeZone={timeZone}
        tracksAttendance={billing.data?.entitlements.attendance}
        onClose={() => setOpenUnid(null)}
      />
    </>
  );
}
