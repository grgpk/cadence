import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarDays, Copy, Video } from "lucide-react";
import { useState } from "react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { BookingDetailSheet } from "../../domains/bookings/BookingDetailSheet";
import {
  agendaDays,
  bookingsQuery,
  splitBookings,
} from "../../domains/bookings/bookings";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { BookingPageStatus } from "../../domains/dashboard/BookingPageStatus";
import { availabilityQuery, hostSettingsQuery } from "../../domains/dashboard/host-api";
import { TodayTape } from "../../domains/dashboard/TodayTape";
import { EmptyState } from "../../domains/shared/EmptyState";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { PHASE_STYLES } from "../../domains/widget/SlotStep";
import { dayPhase, zonedTime } from "../../domains/widget/zoned-time";
import { APP_NAME } from "../../lib/config";
import { formatTime } from "../../lib/format";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/")({
  head: () => ({ meta: [{ title: `Overview | ${APP_NAME}` }] }),
  component: DashboardHome,
});

const NEXT_UP_LIMIT = 6;

/** One call in the agenda, with the color of the part of the day it falls in. */
function AgendaRow({
  booking,
  timeZone,
  onOpen,
}: {
  booking: Booking;
  timeZone: string;
  onOpen: () => void;
}) {
  const { hour } = zonedTime(new Date(booking.slot_start), timeZone);
  return (
    <li className="flex items-center gap-4 px-5 py-3">
      <span
        aria-hidden="true"
        className={`h-9 w-[3px] shrink-0 rounded-full ${PHASE_STYLES[dayPhase(hour)].rail}`}
      />
      <span className="w-20 shrink-0 text-clock text-lg font-medium">
        {formatTime(booking.slot_start, timeZone)}
      </span>
      <div className="min-w-0 flex-1">
        <button
          type="button"
          onClick={onOpen}
          className="block max-w-full truncate rounded-sm text-left font-medium underline-offset-4 outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50"
        >
          {booking.invitee_name}
        </button>
        <p className="truncate text-sm text-muted-foreground">{booking.invitee_email}</p>
      </div>
      {booking.meeting_url ? (
        <Button variant="outline" size="sm" asChild>
          <a href={booking.meeting_url} target="_blank" rel="noreferrer">
            <Video />
            Join
          </a>
        </Button>
      ) : (
        <span className="text-sm text-muted-foreground">No link</span>
      )}
    </li>
  );
}

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
          // Settings that failed to load fall back to each invitee's own timezone.
          agendaDays(nextUp, timeZone ?? nextUp[0]?.timezone ?? "UTC", Date.now()).map(
            (day) => (
              <section key={day.key}>
                <h3 className="border-b bg-muted/60 px-5 py-1.5 text-xs font-medium text-muted-foreground">
                  {day.label}
                </h3>
                <ul className="divide-y border-b last:border-b-0">
                  {day.bookings.map((booking) => (
                    <AgendaRow
                      key={booking.unid}
                      booking={booking}
                      timeZone={timeZone ?? booking.timezone}
                      onOpen={() => setOpenUnid(booking.unid)}
                    />
                  ))}
                </ul>
              </section>
            ),
          )
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
