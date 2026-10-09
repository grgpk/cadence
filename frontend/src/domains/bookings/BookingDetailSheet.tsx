import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, Video } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { Lead } from "../../bindings/Lead";
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
import { formatDay, formatTime, zoneName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ConfirmDialog } from "../shared/ConfirmDialog";
import { ErrorState } from "../shared/ErrorState";
import { BOOKING_STATUS, StatusBadge } from "../shared/StatusBadge";
import { QUESTIONS } from "../widget/booking-form";
import { bookingDetailQuery, cancelBooking, isCancelled } from "./bookings";

/** Where each form question's answer is kept on the lead. */
const ANSWERS = {
  ask_currently_working_on: "currently_working_on",
  ask_how_heard_about_us: "how_heard_about_us",
  ask_what_stopping_you: "what_stopping_you",
  ask_investment_comfort: "investment_comfort",
  ask_urgency_level: "urgency_level",
} as const satisfies Record<(typeof QUESTIONS)[number]["flag"], keyof Lead>;

/**
 * One call in full: who booked it, how to reach them, what they answered, and
 * the way to cancel it. Open while `unid` is set.
 */
export function BookingDetailSheet({
  unid,
  timeZone,
  onClose,
}: {
  unid: string | null;
  /** The host's timezone; without it the call is shown in the invitee's. */
  timeZone?: string;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const query = useQuery({ ...bookingDetailQuery(unid ?? ""), enabled: unid !== null });
  const cancel = useMutation({
    mutationFn: cancelBooking,
    onSuccess: async (booking) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.bookings }),
        queryClient.invalidateQueries({
          queryKey: QUERY_KEYS.bookingDetail(booking.unid),
        }),
      ]);
      toast.success("Booking cancelled");
    },
    onError: (error) =>
      toast.error(`The booking was not cancelled. ${errorMessage(error)}`),
  });

  const booking = query.data?.booking;
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
        <SheetHeader className="border-b">
          <SheetTitle className="pr-8 break-words">
            {booking ? booking.invitee_name : "Booking"}
          </SheetTitle>
          <SheetDescription>
            {booking && zone ? (
              <span className="text-clock text-[0.9375rem]">
                {formatDay(booking.slot_start, zone)},{" "}
                {formatTime(booking.slot_start, zone)}
              </span>
            ) : (
              "The call and what they told you before it."
            )}
          </SheetDescription>
        </SheetHeader>

        {query.isPending ? (
          <div className="grid gap-4 p-4" role="status" aria-label="Loading booking">
            <Skeleton className="h-5 w-48" />
            <Skeleton className="h-5 w-64" />
            <Skeleton className="h-24 w-full" />
          </div>
        ) : query.isError ? (
          <div className="p-4">
            <ErrorState
              title="This booking did not load"
              error={query.error}
              onRetry={() => void query.refetch()}
            />
          </div>
        ) : booking && zone ? (
          <>
            <dl className="divide-y text-sm">
              <div className="flex items-center justify-between gap-4 px-4 py-3">
                <dt className="text-muted-foreground">Status</dt>
                <dd>
                  <StatusBadge value={booking.status} statuses={BOOKING_STATUS} />
                </dd>
              </div>
              <div className="grid gap-1 px-4 py-3">
                <dt className="text-muted-foreground">Email</dt>
                <dd className="break-words">
                  <a
                    href={`mailto:${booking.invitee_email}`}
                    className="font-medium underline-offset-4 hover:underline"
                  >
                    {booking.invitee_email}
                  </a>
                </dd>
              </div>
              {lead?.phone ? (
                <div className="grid gap-1 px-4 py-3">
                  <dt className="text-muted-foreground">Phone</dt>
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
              {zone === booking.timezone ? null : (
                <div className="grid gap-1 px-4 py-3">
                  <dt className="text-muted-foreground">Their time</dt>
                  <dd>
                    <span className="text-clock text-[0.9375rem]">
                      {formatTime(booking.slot_start, booking.timezone)}
                    </span>
                    , {zoneName(booking.timezone)}
                  </dd>
                </div>
              )}
              {answers.map((answer) => (
                <div key={answer.label} className="grid gap-1 px-4 py-3">
                  <dt className="text-muted-foreground">{answer.label}</dt>
                  <dd className="break-words whitespace-pre-wrap">{answer.value}</dd>
                </div>
              ))}
            </dl>
            {answers.length === 0 ? (
              <p className="border-t px-4 py-3 text-sm text-muted-foreground">
                No answers to show. Turn on questions in Settings to ask before a call.
              </p>
            ) : null}

            <div className="mt-auto flex flex-wrap gap-2 border-t p-4">
              {booking.meeting_url && !isCancelled(booking) ? (
                <Button asChild>
                  <a href={booking.meeting_url} target="_blank" rel="noreferrer">
                    <Video />
                    Join
                  </a>
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
                  {cancel.isPending ? "Cancelling" : "Cancel booking"}
                </Button>
              ) : null}
            </div>

            <ConfirmDialog
              open={confirming}
              onOpenChange={setConfirming}
              title="Cancel this booking?"
              description={`${booking.invitee_name} gets an email saying the call is off, and the time opens up on your booking page again.`}
              confirmLabel="Cancel booking"
              destructive
              onConfirm={() => cancel.mutate(booking.unid)}
            />
          </>
        ) : null}
      </SheetContent>
    </Sheet>
  );
}
