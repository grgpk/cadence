import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CalendarDays, Copy, List, Search } from "lucide-react";
import { useMemo, useState } from "react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { Agenda } from "../../domains/bookings/agenda";
import {
  CALENDAR_MODE,
  type CalendarMode,
  CalendarView,
} from "../../domains/bookings/calendar-view";
import { BookingDetailSheet } from "../../domains/bookings/booking-detail-sheet";
import {
  bookingsQuery,
  matchesSearch,
  splitBookings,
} from "../../domains/bookings/bookings";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { availabilityQuery, hostSettingsQuery } from "../../domains/dashboard/host-api";
import { EmptyState } from "../../domains/shared/empty-state";
import { ErrorState } from "../../domains/shared/error-state";
import { PageHeader } from "../../domains/shared/page-header";
import { SegmentedControl } from "../../domains/shared/segmented-control";
import { TableSkeleton } from "../../domains/shared/table-skeleton";
import { APP_NAME } from "../../lib/config";
import { useNow } from "../../hooks/use-now";

const VIEW = { list: "list", calendar: "calendar" } as const;
type View = (typeof VIEW)[keyof typeof VIEW];

type BookingsSearch = { view?: View; mode?: CalendarMode; offset?: number };

const MODES: readonly unknown[] = Object.values(CALENDAR_MODE);

export const Route = createFileRoute("/dashboard/bookings")({
  // The calendar's place lives in the URL, so a reload stays on the same week.
  validateSearch: (search): BookingsSearch => {
    if (search.view !== VIEW.calendar) return {};
    const offset = Number(search.offset);
    return {
      view: VIEW.calendar,
      mode: MODES.includes(search.mode) ? (search.mode as CalendarMode) : undefined,
      offset: Number.isInteger(offset) && offset !== 0 ? offset : undefined,
    };
  },
  head: () => ({ meta: [{ title: `Bookings | ${APP_NAME}` }] }),
  component: BookingsPage,
});

const TABS = { UPCOMING: "upcoming", PAST: "past", CANCELLED: "cancelled" } as const;
const VIEWS = [
  { value: VIEW.list, label: "List", icon: List },
  { value: VIEW.calendar, label: "Calendar", icon: CalendarDays },
] as const;

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
        <Agenda bookings={bookings} timeZone={timeZone} onOpen={onOpen} heading="h2" />
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
  const availability = useQuery(availabilityQuery);
  const [search, setSearch] = useState("");
  const { view = VIEW.list, mode, offset = 0 } = Route.useSearch();
  const navigate = Route.useNavigate();
  const setView = (next: View) =>
    void navigate({ search: next === VIEW.calendar ? { view: next } : {} });
  const [openUnid, setOpenUnid] = useState<string | null>(null);
  const slug = settings.data?.public_slug;
  const timeZone = settings.data?.timezone;

  // Each minute, so a call that starts moves out of Upcoming on its own.
  const now = useNow()?.getTime();
  const { upcoming, past, cancelled } = useMemo(
    () =>
      splitBookings(
        (query.data ?? []).filter((booking) => matchesSearch(booking, search)),
        now ?? Date.now(),
      ),
    [query.data, search, now],
  );
  // The agenda needs the host's timezone before it can group calls by day.
  const loading = query.isPending || settings.isPending;
  const open = (booking: Booking) => setOpenUnid(booking.unid);

  return (
    <>
      <PageHeader
        title="Bookings"
        description="Calls people booked from your booking page."
        actions={
          <SegmentedControl
            label="Show calls as"
            value={view}
            options={VIEWS}
            onChange={setView}
          />
        }
      />
      {query.isError ? (
        <ErrorState
          title="Bookings did not load"
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : settings.isError ? (
        // Without the host's timezone every time on this page would be a guess.
        <ErrorState
          title="Your timezone did not load"
          error={settings.error}
          onRetry={() => void settings.refetch()}
        />
      ) : view === VIEW.calendar ? (
        loading || availability.isPending ? (
          <Card className="gap-0 overflow-hidden py-0">
            <TableSkeleton rows={7} />
          </Card>
        ) : (
          <CalendarView
            bookings={query.data ?? []}
            rules={availability.data ?? []}
            timeZone={timeZone ?? "UTC"}
            mode={mode}
            offset={offset}
            onNavigate={(next) =>
              void navigate({
                search: {
                  view: VIEW.calendar,
                  mode: next.mode,
                  offset: next.offset || undefined,
                },
                replace: true,
              })
            }
            onOpen={open}
          />
        )
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
              <TabsTrigger value={TABS.CANCELLED}>
                Cancelled
                {loading ? null : (
                  <span className="text-muted-foreground tabular-nums">
                    {cancelled.length}
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
                  description="Calls move here once they have started."
                />
              }
            />
          </TabsContent>
          <TabsContent value={TABS.CANCELLED}>
            <BookingList
              loading={loading}
              bookings={cancelled}
              timeZone={timeZone}
              search={search}
              onOpen={open}
              empty={
                <EmptyState
                  icon={CalendarDays}
                  title="No cancelled calls"
                  description="A call you or the person who booked it cancels is kept here."
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
