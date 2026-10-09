import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CalendarDays, Copy, Search } from "lucide-react";
import { useMemo, useState } from "react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { Agenda } from "../../domains/bookings/Agenda";
import { BookingDetailSheet } from "../../domains/bookings/BookingDetailSheet";
import {
  bookingsQuery,
  matchesSearch,
  splitBookings,
} from "../../domains/bookings/bookings";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { hostSettingsQuery } from "../../domains/dashboard/host-api";
import { EmptyState } from "../../domains/shared/EmptyState";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { APP_NAME } from "../../lib/config";

export const Route = createFileRoute("/dashboard/bookings")({
  head: () => ({ meta: [{ title: `Bookings | ${APP_NAME}` }] }),
  component: BookingsPage,
});

const TABS = { UPCOMING: "upcoming", PAST: "past" } as const;

function BookingList({
  loading,
  bookings,
  timeZone,
  search,
  empty,
  onOpen,
}: {
  loading: boolean;
  bookings: Booking[];
  timeZone?: string;
  /** What the host typed, to say so when it leaves nothing. */
  search: string;
  empty: React.ReactNode;
  onOpen: (booking: Booking) => void;
}) {
  return (
    <Card className="gap-0 overflow-hidden py-0">
      {loading ? (
        <TableSkeleton />
      ) : bookings.length > 0 ? (
        <Agenda bookings={bookings} timeZone={timeZone} onOpen={onOpen} />
      ) : search.trim() ? (
        <EmptyState
          icon={Search}
          title={`No one matches "${search.trim()}"`}
          description="Search looks at names and email addresses. Check the spelling, or clear the search."
        />
      ) : (
        empty
      )}
    </Card>
  );
}

function BookingsPage() {
  const query = useQuery(bookingsQuery);
  const settings = useQuery(hostSettingsQuery);
  const [search, setSearch] = useState("");
  const [openUnid, setOpenUnid] = useState<string | null>(null);
  const slug = settings.data?.public_slug;
  const timeZone = settings.data?.timezone;

  const { upcoming, past } = useMemo(
    () =>
      splitBookings(
        (query.data ?? []).filter((booking) => matchesSearch(booking, search)),
        Date.now(),
      ),
    [query.data, search],
  );
  // The agenda needs the host's timezone before it can group calls by day.
  const loading = query.isPending || settings.isPending;
  const open = (booking: Booking) => setOpenUnid(booking.unid);

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
          <div className="flex flex-wrap items-center justify-between gap-3">
            <TabsList>
              <TabsTrigger value={TABS.UPCOMING}>
                Upcoming
                {loading ? null : (
                  <span className="text-muted-foreground tabular-nums">
                    {upcoming.length}
                  </span>
                )}
              </TabsTrigger>
              <TabsTrigger value={TABS.PAST}>
                Past
                {loading ? null : (
                  <span className="text-muted-foreground tabular-nums">
                    {past.length}
                  </span>
                )}
              </TabsTrigger>
            </TabsList>
            <div className="relative w-full sm:w-64">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                type="search"
                aria-label="Search bookings by name or email"
                placeholder="Search by name or email"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                className="bg-card pl-8"
              />
            </div>
          </div>
          <TabsContent value={TABS.UPCOMING}>
            <BookingList
              loading={loading}
              bookings={upcoming}
              timeZone={timeZone}
              search={search}
              onOpen={open}
              empty={
                <EmptyState
                  icon={CalendarDays}
                  title="No upcoming calls"
                  description="A call shows up here as soon as someone picks a time. Send your link to get the first one."
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
              }
            />
          </TabsContent>
          <TabsContent value={TABS.PAST}>
            <BookingList
              loading={loading}
              bookings={past}
              timeZone={timeZone}
              search={search}
              onOpen={open}
              empty={
                <EmptyState
                  icon={CalendarDays}
                  title="No past calls"
                  description="Calls move here once they have started, and cancelled calls are kept here too."
                />
              }
            />
          </TabsContent>
        </Tabs>
      )}
      <BookingDetailSheet
        unid={openUnid}
        timeZone={timeZone}
        onClose={() => setOpenUnid(null)}
      />
    </>
  );
}
