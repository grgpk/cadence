import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CalendarX, Loader2, Video } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import type { ManagedBooking } from "../../bindings/ManagedBooking";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { ApiError } from "../../lib/api";
import { errorMessage } from "../../lib/error-message";
import { formatDay, formatLongDay, prefersHour12, zoneName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { ErrorState } from "../shared/error-state";
import { PublicLayout } from "../shared/public-layout";
import {
  cancelManagedBooking,
  managedBookingQuery,
  rescheduleManagedBooking,
} from "../widget/cadence-api";
import { bookingFailure } from "../widget/booking-session";
import { SlotStep, type CalendarView } from "../widget/slot-step";
import { callEnd, clockLabel, zonedTime } from "../../lib/zoned-time";
import { BookedStrip } from "./booked-strip";
import { isCancelled } from "./bookings";

function headline(booking: ManagedBooking): string {
  if (isCancelled(booking)) return "This call is cancelled";
  return booking.can_change
    ? `Your call with ${booking.host_name}`
    : `Your call with ${booking.host_name} has started`;
}

/** The calendar and times for rescheduling a call, and the question before it moves. */
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
      toast.success("Call rescheduled");
      onDone();
    },
    onError: async (error) => {
      setPickKey((key) => key + 1);
      // The time may have just been taken, so the cached times can no longer be trusted.
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.publicSlots(booking.slug),
      });
      toast.error(
        bookingFailure(error) === "taken"
          ? "That time was just taken. Pick another one."
          : `The call was not rescheduled. ${errorMessage(error)}`,
      );
    },
  });

  // The picker opens below the buttons, often past the bottom of the screen.
  const bringIntoView = useCallback((picker: HTMLDivElement | null) => {
    picker?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, []);

  return (
    <div ref={bringIntoView} className="@container mt-6 scroll-mb-6">
      {/* No second time can be picked while the first is being saved. */}
      <div
        inert={move.isPending}
        aria-busy={move.isPending}
        className="overflow-hidden rounded-2xl border bg-card"
      >
        <SlotStep
          slug={booking.slug}
          timeZone={booking.timezone}
          emptyDescription={`${booking.host_name} has no other open times right now. Your call stays where it is.`}
          pausedDescription={`${booking.host_name} is not taking new times right now. Your call stays where it is.`}
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
        title="Reschedule your call?"
        description={
          picked
            ? `New time: ${formatDay(picked.start, booking.timezone)}, ${clockLabel(new Date(picked.start), booking.timezone, clock12)} (${zoneName(booking.timezone)}). ${booking.host_name} is told about the change.`
            : ""
        }
        confirmLabel="Reschedule"
        dismissLabel="Keep looking"
        onConfirm={() => picked && move.mutate(picked)}
      />
      {move.isPending ? (
        <p
          role="status"
          className="mt-3 flex items-center gap-2 text-sm text-muted-foreground"
        >
          <Loader2 className="size-4 animate-spin" />
          Rescheduling your call
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
  // After a call is rescheduled or cancelled the button that did it is gone,
  // so the heading, which now says what happened, takes the focus.
  const heading = useRef<HTMLHeadingElement>(null);
  const settled = `${query.data?.slot_start}|${query.data?.status}`;
  const lastSettled = useRef<string | null>(null);
  useEffect(() => {
    if (!query.data) return;
    if (lastSettled.current !== null && lastSettled.current !== settled) {
      heading.current?.focus();
    }
    lastSettled.current = settled;
  }, [query.data, settled]);
  // The device's clock style is only known in the browser.
  const [hour12, setHour12] = useState(false);
  useEffect(() => setHour12(prefersHour12()), []);

  const cancel = useMutation({
    mutationFn: () => cancelManagedBooking(token),
    onSuccess: async (cancelled) => {
      queryClient.setQueryData(QUERY_KEYS.managedBooking(token), cancelled);
      setRescheduling(false);
      toast.success("Call cancelled");
      // The time it held is open again.
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.publicSlots(cancelled.slug),
      });
    },
    onError: (error) => toast.error(`The call was not cancelled. ${errorMessage(error)}`),
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
    const unknown = query.error instanceof ApiError && query.error.isNotFound;
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
  const day = formatLongDay(startsAt, booking.timezone);

  return (
    <PublicLayout accountLinks={false}>
      <section
        className={`mx-auto px-4 py-14 sm:py-20 ${rescheduling ? "max-w-4xl" : "max-w-lg"}`}
      >
        <h1
          ref={heading}
          tabIndex={-1}
          className="font-display font-wide text-4xl leading-none font-bold tracking-tight outline-none sm:text-5xl"
        >
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
              hour12={hour12}
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
                Join call
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
                {cancel.isPending ? "Cancelling" : "Cancel call"}
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
          title="Cancel this call?"
          description={`${booking.host_name} is told the call is off and the time opens up for someone else.`}
          confirmLabel="Cancel call"
          dismissLabel="Keep call"
          destructive
          onConfirm={() => cancel.mutate()}
        />
      </section>
    </PublicLayout>
  );
}
