import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Loader2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { bookingPageQuery } from "../widget/cadence-api";
import {
  calendarEventsQuery,
  connectGoogleCalendar,
  disconnectGoogleCalendar,
} from "./host-api";

/** The host's Google Calendar link: its state and the one action that changes it. */
export function GoogleCalendarConnection({ slug }: { slug: string }) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const events = useQuery(calendarEventsQuery);
  // The events call fails both when no calendar is connected and when Google
  // is unreachable. The booking page knows which, straight from our database.
  const page = useQuery(bookingPageQuery(slug));
  // Pending until the browser leaves for Google's sign-in page.
  const connect = useMutation({
    mutationFn: connectGoogleCalendar,
    onError: () => toast.error("Google sign-in could not be started. Try again."),
  });
  const connectIcon = connect.isPending ? <Loader2 className="animate-spin" /> : null;
  const disconnect = useMutation({
    mutationFn: disconnectGoogleCalendar,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.calendarEvents }),
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.bookingPage(slug) }),
      ]);
      toast.success("Google Calendar disconnected");
    },
    onError: () => toast.error("Google Calendar was not disconnected. Try again."),
  });

  const disconnectButton = (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className="ml-auto"
      onClick={() => setConfirming(true)}
      disabled={disconnect.isPending}
    >
      {disconnect.isPending ? <Loader2 className="animate-spin" /> : null}
      {disconnect.isPending ? "Disconnecting" : "Disconnect"}
    </Button>
  );

  return (
    <div className="grid gap-1.5">
      <p className="text-sm leading-none font-medium">Google Calendar</p>
      <div className="flex min-h-9 flex-wrap items-center gap-x-3 gap-y-2">
        {events.isPending || (events.isError && page.isPending) ? (
          <Skeleton className="h-9 w-48" />
        ) : events.isSuccess ? (
          <>
            <span className="inline-flex items-center gap-2 text-sm font-medium text-success">
              <CheckCircle2 className="size-4" />
              Connected
            </span>
            <span className="text-sm text-muted-foreground tabular-nums">
              {events.data.length} {events.data.length === 1 ? "event" : "events"} in the
              next 7 days
            </span>
            {disconnectButton}
          </>
        ) : page.data?.calendar_connected ? (
          <>
            <span className="inline-flex items-center gap-2 text-sm font-medium text-warning">
              <AlertTriangle className="size-4" />
              Connected, but Google did not answer
            </span>
            <Button
              type="button"
              size="sm"
              onClick={() => connect.mutate()}
              disabled={connect.isPending}
            >
              {connectIcon}
              Reconnect
            </Button>
            {disconnectButton}
          </>
        ) : (
          <>
            <Button
              type="button"
              onClick={() => connect.mutate()}
              disabled={connect.isPending}
            >
              {connectIcon}
              Connect Google Calendar
            </Button>
            <p className="text-sm text-muted-foreground">Not connected</p>
          </>
        )}
      </div>
      <p className="text-sm text-muted-foreground">
        Keeps your busy times off your booking page and adds a Google Meet link to each
        call.
      </p>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title="Disconnect Google Calendar?"
        description="New calls stop getting a Google Meet link, and your busy times no longer block your booking page."
        confirmLabel="Disconnect"
        destructive
        onConfirm={() => disconnect.mutate()}
      />
    </div>
  );
}
