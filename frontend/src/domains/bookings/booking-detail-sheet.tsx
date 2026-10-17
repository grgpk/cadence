import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { cn } from "cn";
import { Globe, Loader2, Mail, Phone, Video } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { Booking } from "../../bindings/Booking";
import type { Lead } from "../../bindings/Lead";
import { Avatar, AvatarFallback } from "../../components/ui/avatar";
import { Button } from "../../components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "../../components/ui/sheet";
import { Skeleton } from "../../components/ui/skeleton";
import { errorMessage } from "../../lib/error-message";
import { formatDay, formatTime, lengthLabel, zoneName } from "../../lib/format";
import { callEnd, dayPhase, zonedTime } from "../../lib/zoned-time";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { ErrorState } from "../shared/error-state";
import { BOOKING_STATUS, StatusBadge } from "../shared/status-badge";
import { QUESTIONS } from "../widget/booking-form";
import { AttendanceBadge } from "./agenda";
import {
  attendanceOf,
  bookingDetailQuery,
  cancelBooking,
  isCancelled,
  isConfirmed,
  setNoShow,
} from "./bookings";

/** Where each form question's answer is kept on the lead. */
const ANSWERS = {
  ask_currently_working_on: "currently_working_on",
  ask_how_heard_about_us: "how_heard_about_us",
  ask_what_stopping_you: "what_stopping_you",
  ask_investment_comfort: "investment_comfort",
  ask_urgency_level: "urgency_level",
} as const satisfies Record<(typeof QUESTIONS)[number]["flag"], keyof Lead>;

/** When the call is, as a block on the day: the same rail its row in the list has. */
function CallBlock({ booking, timeZone }: { booking: Booking; timeZone: string }) {
  const { hour } = zonedTime(new Date(booking.slot_start), timeZone);
  const cancelled = isCancelled(booking);
  return (
    <div className="relative overflow-hidden rounded-lg border bg-muted/60 py-2.5 pr-3 pl-4">
      <span
        aria-hidden="true"
        className={cn(
          "absolute inset-y-0 left-0 w-[3px]",
          cancelled ? "bg-border" : PHASE_STYLES[dayPhase(hour)].rail,
        )}
      />
      <p className="text-sm font-semibold">{formatDay(booking.slot_start, timeZone)}</p>
      <p className={cn("text-clock text-base", cancelled && "line-through")}>
        {formatTime(booking.slot_start, timeZone)} to{" "}
        {formatTime(callEnd(booking.slot_start, booking.minutes).toISOString(), timeZone)}
        <span className="text-muted-foreground"> · {lengthLabel(booking.minutes)}</span>
      </p>
    </div>
  );
}

/**
 * One call in full: who booked it, how to reach them, what they answered, and
 * the way to cancel it. Open while `unid` is set.
 */
export function BookingDetailSheet({
  unid,
  timeZone,
  tracksAttendance = false,
  onClose,
}: {
  unid: string | null;
  /** The host's timezone; without it the call is shown in the invitee's. */
  timeZone?: string;
  /** Business: whether the invitee confirmed, and marking a no-show. */
  tracksAttendance?: boolean;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  // The sheet slides out after `unid` is cleared; keep showing the call it had.
  const [shown, setShown] = useState(unid);
  if (unid !== null && unid !== shown) setShown(unid);
  const query = useQuery({ ...bookingDetailQuery(shown ?? ""), enabled: shown !== null });
  const cancel = useMutation({
    mutationFn: cancelBooking,
    onSuccess: async (booking) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.bookings }),
        queryClient.invalidateQueries({
          queryKey: QUERY_KEYS.bookingDetail(booking.unid),
        }),
      ]);
      toast.success("Call cancelled");
    },
    onError: (error) => toast.error(`The call was not cancelled. ${errorMessage(error)}`),
  });

  const noShow = useMutation({
    mutationFn: ({ unid, marked }: { unid: string; marked: boolean }) =>
      setNoShow(unid, marked),
    onSuccess: async (booking) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.bookings }),
        queryClient.invalidateQueries({
          queryKey: QUERY_KEYS.bookingDetail(booking.unid),
        }),
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.insights }),
      ]);
      toast.success(booking.no_show_at ? "Marked as a no-show" : "No-show taken back");
    },
    onError: (error) => toast.error(`Nothing changed. ${errorMessage(error)}`),
  });

  const booking = query.data?.booking;
  const over =
    booking !== undefined &&
    !isCancelled(booking) &&
    new Date(booking.slot_start).getTime() + booking.minutes * 60_000 <= Date.now();
  const lead = query.data?.lead;
  const zone = timeZone ?? booking?.timezone;
  const canCancel =
    booking !== undefined &&
    !isCancelled(booking) &&
    new Date(booking.slot_start).getTime() > Date.now();
  const answers = QUESTIONS.map((question) => ({
    label: question.label,
    value: lead?.[ANSWERS[question.flag]],
  })).filter((answer) => answer.value);

  return (
    <Sheet open={unid !== null} onOpenChange={(open) => (open ? undefined : onClose())}>
      <SheetContent className="w-full gap-0 overflow-y-auto sm:max-w-lg">
        <SheetHeader className="gap-4 border-b">
          <div className="flex items-center gap-3 pr-8">
            {booking ? (
              <Avatar aria-hidden="true" className="size-10">
                <AvatarFallback className="bg-muted font-display font-semibold">
                  {booking.invitee_name.trim().charAt(0).toUpperCase()}
                </AvatarFallback>
              </Avatar>
            ) : null}
            <div className="min-w-0 flex-1">
              <SheetTitle className="text-lg break-words">
                {booking ? booking.invitee_name : "Call"}
              </SheetTitle>
              <SheetDescription className={booking ? "sr-only" : undefined}>
                The call and what they told you before it.
              </SheetDescription>
            </div>
            {/* Confirmed is every booking's normal state; only say when it is not. */}
            {booking && !isConfirmed(booking) ? (
              <StatusBadge value={booking.status} statuses={BOOKING_STATUS} />
            ) : booking && tracksAttendance ? (
              <AttendanceBadge attendance={attendanceOf(booking, Date.now())} />
            ) : null}
          </div>
          {booking && zone ? <CallBlock booking={booking} timeZone={zone} /> : null}
        </SheetHeader>

        {query.isPending ? (
          <div className="grid gap-4 p-4" role="status" aria-label="Loading call">
            <Skeleton className="h-5 w-48" />
            <Skeleton className="h-5 w-64" />
            <Skeleton className="h-24 w-full" />
          </div>
        ) : query.isError ? (
          <div className="p-4">
            <ErrorState
              title="This call did not load"
              error={query.error}
              onRetry={() => void query.refetch()}
            />
          </div>
        ) : booking && zone ? (
          <>
            <dl className="divide-y text-sm">
              <div className="flex items-start gap-3 px-4 py-3">
                <dt>
                  <Mail className="mt-0.5 size-4 text-muted-foreground" />
                  <span className="sr-only">Email</span>
                </dt>
                <dd className="min-w-0 break-words">
                  <a
                    href={`mailto:${booking.invitee_email}`}
                    className="font-medium underline-offset-4 hover:underline"
                  >
                    {booking.invitee_email}
                  </a>
                </dd>
              </div>
              {lead?.phone ? (
                <div className="flex items-start gap-3 px-4 py-3">
                  <dt>
                    <Phone className="mt-0.5 size-4 text-muted-foreground" />
                    <span className="sr-only">Phone</span>
                  </dt>
                  <dd>
                    <a
                      href={`tel:${lead.phone}`}
                      className="font-medium underline-offset-4 hover:underline"
                    >
                      {lead.phone}
                    </a>
                  </dd>
                </div>
              ) : null}
              {tracksAttendance && !isCancelled(booking) ? (
                <div className="flex items-center justify-between gap-4 px-4 py-3">
                  <dt className="text-muted-foreground">Coming</dt>
                  <dd>
                    {booking.no_show_at
                      ? "Did not come"
                      : booking.attendance_confirmed_at
                        ? `Said yes on ${formatDay(booking.attendance_confirmed_at, zone)}`
                        : "No reply yet"}
                  </dd>
                </div>
              ) : null}
              {zone === booking.timezone ? null : (
                <div className="flex items-start gap-3 px-4 py-3">
                  <dt>
                    <Globe className="mt-0.5 size-4 text-muted-foreground" />
                    <span className="sr-only">Their time</span>
                  </dt>
                  <dd>
                    <span className="text-clock text-md">
                      {formatTime(booking.slot_start, booking.timezone)}
                    </span>{" "}
                    their time, {zoneName(booking.timezone)}
                  </dd>
                </div>
              )}
            </dl>
            <section className="grid gap-4 border-t px-4 py-4">
              <h3 className="text-sm font-semibold">Their answers</h3>
              {answers.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  Nothing asked for this call. Turn on questions under Event types to
                  learn more before a call.
                </p>
              ) : (
                <dl className="grid gap-4 text-sm">
                  {answers.map((answer) => (
                    <div key={answer.label} className="grid gap-1.5">
                      <dt className="text-muted-foreground">{answer.label}</dt>
                      <dd className="border-l-2 pl-3 break-words whitespace-pre-wrap">
                        {answer.value}
                      </dd>
                    </div>
                  ))}
                </dl>
              )}
            </section>

            <div className="mt-auto flex flex-wrap gap-2 border-t p-4">
              {booking.meeting_url && !isCancelled(booking) ? (
                <Button asChild>
                  <a href={booking.meeting_url} target="_blank" rel="noreferrer">
                    <Video />
                    Join call
                  </a>
                </Button>
              ) : null}
              {tracksAttendance && over ? (
                <Button
                  type="button"
                  variant="outline"
                  disabled={noShow.isPending}
                  onClick={() =>
                    noShow.mutate({
                      unid: booking.unid,
                      marked: booking.no_show_at === null,
                    })
                  }
                >
                  {noShow.isPending ? <Loader2 className="animate-spin" /> : null}
                  {booking.no_show_at ? "They did come" : "Mark no-show"}
                </Button>
              ) : null}
              {canCancel ? (
                <Button
                  type="button"
                  variant="outline"
                  className="text-destructive hover:text-destructive"
                  disabled={cancel.isPending}
                  onClick={() => setConfirming(true)}
                >
                  {cancel.isPending ? <Loader2 className="animate-spin" /> : null}
                  {cancel.isPending ? "Cancelling" : "Cancel call"}
                </Button>
              ) : null}
            </div>

            <ConfirmDialog
              open={confirming}
              onOpenChange={setConfirming}
              title="Cancel this call?"
              description={`${booking.invitee_name} gets an email saying the call is off, and the time opens up on your booking page again.`}
              confirmLabel="Cancel call"
              dismissLabel="Keep call"
              destructive
              onConfirm={() => cancel.mutate(booking.unid)}
            />
          </>
        ) : null}
      </SheetContent>
    </Sheet>
  );
}
