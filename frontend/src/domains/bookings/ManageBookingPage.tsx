import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CalendarX, Loader2, Video } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import type { ManagedBooking } from "../../bindings/ManagedBooking";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { ApiError } from "../../lib/api";
import { errorMessage } from "../../lib/error-message";
import { formatDay, prefersHour12, zoneName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";
import { ConfirmDialog } from "../shared/ConfirmDialog";
import { ErrorState } from "../shared/ErrorState";
import { PublicLayout } from "../shared/PublicLayout";
import {
  cancelManagedBooking,
  managedBookingQuery,
  rescheduleManagedBooking,
} from "../widget/cadence-api";
import { SlotStep, type CalendarView } from "../widget/SlotStep";
import { callEnd, clockLabel, zonedTime } from "../widget/zoned-time";
import { BookedStrip } from "./BookedStrip";
import { isCancelled } from "./bookings";

const NOT_FOUND = 404;

function headline(booking: ManagedBooking): string {
  if (isCancelled(booking)) return "This call is cancelled";
  return booking.can_change
    ? `Your call with ${booking.host_name}`
    : `Your call with ${booking.host_name} has started`;
}

/** The calendar and times for moving a call, and the question before it moves. */
function Reschedule({
  token,
  booking,
  hour12,
  onDone,
}: {
  token: string;
  booking: ManagedBooking;
  hour12: boolean;
  onDone: () => void;
}) {
  const queryClient = useQueryClient();
  const [clock12, setClock12] = useState(hour12);
  const [view, setView] = useState<CalendarView>(() => {
    const today = zonedTime(new Date(), booking.timezone);
    return { year: today.year, month: today.month, day: null };
  });
  const [picked, setPicked] = useState<TimeSlot | null>(null);
  // Bumped to let go of a time that was picked and then not confirmed.
  const [pickKey, setPickKey] = useState(0);

  const move = useMutation({
    mutationFn: (slot: TimeSlot) => rescheduleManagedBooking(token, slot.start),
    onSuccess: async (moved) => {
      queryClient.setQueryData(QUERY_KEYS.managedBooking(token), moved);
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.publicSlots(booking.slug),
      });
      toast.success("Call moved");
      onDone();
    },
    onError: async (error) => {
      setPickKey((key) => key + 1);
      // The time may have just been taken, so the cached times can no longer be trusted.
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.publicSlots(booking.slug),
      });
      toast.error(
        error instanceof ApiError && error.isConflict
          ? "That time was just taken. Pick another one."
          : `The call was not moved. ${errorMessage(error)}`,
      );
    },
  });

  // The picker opens below the buttons, often past the bottom of the screen.
  const bringIntoView = useCallback((picker: HTMLDivElement | null) => {
    picker?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, []);

  return (
    <div ref={bringIntoView} className="@container mt-6 scroll-mb-6">
      <div className="overflow-hidden rounded-2xl border bg-card">
        <SlotStep
          slug={booking.slug}
          timeZone={booking.timezone}
          emptyDescription={`${booking.host_name} has no other open times right now. Your call stays where it is.`}
          view={view}
          onViewChange={setView}
          hour12={clock12}
          onHour12Change={setClock12}
          pickKey={pickKey}
          onPick={setPicked}
        />
      </div>
      <ConfirmDialog
        open={picked !== null}
        onOpenChange={(open) => {
          if (open) return;
          setPicked(null);
          setPickKey((key) => key + 1);
        }}
        title="Move your call?"
        description={
          picked
            ? `New time: ${formatDay(picked.start, booking.timezone)}, ${clockLabel(new Date(picked.start), booking.timezone, clock12)} (${zoneName(booking.timezone)}). ${booking.host_name} is told about the change.`
            : ""
        }
        confirmLabel="Move call"
        onConfirm={() => picked && move.mutate(picked)}
      />
      {move.isPending ? (
        <p
          role="status"
          className="mt-3 flex items-center gap-2 text-sm text-muted-foreground"
        >
          <Loader2 className="size-4 animate-spin" />
          Moving your call
        </p>
      ) : null}
    </div>
  );
}

/** A call as the person who booked it sees it: when it is, and how to move or cancel it. */
export function ManageBookingPage({ token }: { token: string }) {
  const queryClient = useQueryClient();
  const query = useQuery({
    ...managedBookingQuery(token),
    // An unknown link stays unknown; asking again only delays saying so.
    retry: false,
  });
  const [rescheduling, setRescheduling] = useState(false);
  const [confirmingCancel, setConfirmingCancel] = useState(false);
  // The device's clock style is only known in the browser.
  const [hour12, setHour12] = useState(false);
  useEffect(() => setHour12(prefersHour12()), []);

  const cancel = useMutation({
    mutationFn: () => cancelManagedBooking(token),
    onSuccess: (cancelled) => {
      queryClient.setQueryData(QUERY_KEYS.managedBooking(token), cancelled);
      setRescheduling(false);
      toast.success("Booking cancelled");
    },
    onError: (error) =>
      toast.error(`The booking was not cancelled. ${errorMessage(error)}`),
  });

  if (query.isPending) {
    return (
      <PublicLayout accountLinks={false}>
        <section
          className="mx-auto grid max-w-lg gap-4 px-4 py-14 sm:py-20"
          role="status"
          aria-label="Loading your call"
        >
          <Skeleton className="h-10 w-3/4" />
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-36 w-full" />
        </section>
      </PublicLayout>
    );
  }

  if (query.isError) {
    const unknown = query.error instanceof ApiError && query.error.status === NOT_FOUND;
    return (
      <PublicLayout accountLinks={false}>
        <section className="mx-auto flex max-w-lg flex-col items-center gap-4 px-4 py-20 text-center">
          {unknown ? (
            <>
              <CalendarX className="size-8 text-muted-foreground" />
              <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
                This link does not open a call
              </h1>
              <p className="text-muted-foreground">
                Check that the whole link was copied from your confirmation email.
              </p>
              <Button asChild>
                <Link to={ROUTES.home}>Go to the home page</Link>
              </Button>
            </>
          ) : (
            <ErrorState
              title="Your call did not load"
              error={query.error}
              onRetry={() => void query.refetch()}
            />
          )}
        </section>
      </PublicLayout>
    );
  }

  const booking = query.data;
  const startsAt = new Date(booking.slot_start);
  const cancelled = isCancelled(booking);
  const timeRange = `${clockLabel(startsAt, booking.timezone, hour12)} to ${clockLabel(callEnd(booking.slot_start, booking.minutes), booking.timezone, hour12)}`;
  const day = new Intl.DateTimeFormat("en", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: booking.timezone,
  }).format(startsAt);

  return (
    <PublicLayout accountLinks={false}>
      <section
        className={`mx-auto px-4 py-14 sm:py-20 ${rescheduling ? "max-w-3xl" : "max-w-lg"}`}
      >
        <h1 className="font-display font-wide text-4xl leading-none font-bold tracking-tight sm:text-5xl">
          {headline(booking)}
        </h1>
        <p className="mt-4 text-muted-foreground">
          {cancelled
            ? `${booking.title}. Nothing more to do; the time is free again.`
            : booking.title}
        </p>

        {cancelled ? null : (
          <div className="mt-8">
            <BookedStrip
              // A moved call is a new block: it drops in again.
              key={booking.slot_start}
              start={startsAt}
              minutes={booking.minutes}
              timeZone={booking.timezone}
              label={timeRange}
            />
          </div>
        )}

        <dl className="mt-4 divide-y rounded-xl border bg-card text-sm">
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Date</dt>
            <dd className={`text-right font-medium ${cancelled ? "line-through" : ""}`}>
              {day}
            </dd>
          </div>
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Time</dt>
            <dd
              className={`text-right text-clock text-[0.9375rem] font-medium ${cancelled ? "line-through" : ""}`}
            >
              {timeRange}
            </dd>
          </div>
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Timezone</dt>
            <dd className="text-right font-medium">{zoneName(booking.timezone)}</dd>
          </div>
        </dl>

        <div className="mt-8 flex flex-wrap gap-3">
          {booking.meeting_url && !cancelled ? (
            <Button asChild>
              <a href={booking.meeting_url} target="_blank" rel="noreferrer">
                <Video />
                Join the call
              </a>
            </Button>
          ) : null}
          {booking.can_change ? (
            <>
              <Button
                type="button"
                variant={booking.meeting_url ? "outline" : "default"}
                aria-expanded={rescheduling}
                onClick={() => setRescheduling(!rescheduling)}
              >
                {rescheduling ? "Keep this time" : "Reschedule"}
              </Button>
              <Button
                type="button"
                variant="outline"
                className="text-destructive hover:text-destructive"
                disabled={cancel.isPending}
                onClick={() => setConfirmingCancel(true)}
              >
                {cancel.isPending ? <Loader2 className="animate-spin" /> : null}
                {cancel.isPending ? "Cancelling" : "Cancel booking"}
              </Button>
            </>
          ) : (
            <Button variant={cancelled ? "default" : "outline"} asChild>
              <Link to={ROUTES.bookingPage} params={{ slug: booking.slug }}>
                Book another time
              </Link>
            </Button>
          )}
        </div>

        {rescheduling && booking.can_change ? (
          <Reschedule
            token={token}
            booking={booking}
            hour12={hour12}
            onDone={() => setRescheduling(false)}
          />
        ) : null}

        <ConfirmDialog
          open={confirmingCancel}
          onOpenChange={setConfirmingCancel}
          title="Cancel this booking?"
          description={`${booking.host_name} is told the call is off and the time opens up for someone else.`}
          confirmLabel="Cancel booking"
          destructive
          onConfirm={() => cancel.mutate()}
        />
      </section>
    </PublicLayout>
  );
}
