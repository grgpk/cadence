import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Loader2 } from "lucide-react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { QUERY_KEYS } from "../../lib/query-keys";
import {
  calendarEventsQuery,
  connectGoogleCalendar,
  disconnectGoogleCalendar,
} from "../dashboard/host-api";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { bookingPageQuery } from "../widget/cadence-api";
import { GoogleCalendarIcon } from "./google-calendar-icon";

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
      variant="ghost"
      size="sm"
      onClick={() => setConfirming(true)}
      disabled={disconnect.isPending}
    >
      {disconnect.isPending ? <Loader2 className="animate-spin" /> : null}
      {disconnect.isPending ? "Disconnecting" : "Disconnect"}
    </Button>
  );

  let status: ReactNode;
  let actions: ReactNode;
  if (events.isPending || (events.isError && page.isPending)) {
    status = <Skeleton className="h-4 w-40" />;
    actions = <Skeleton className="h-8 w-24" />;
  } else if (events.isSuccess) {
    status = (
      <span className="inline-flex flex-wrap items-center gap-x-2 gap-y-1">
        <span className="inline-flex items-center gap-1.5 font-medium text-success">
          <CheckCircle2 className="size-4 motion-safe:animate-in motion-safe:zoom-in-50 motion-safe:fade-in motion-safe:duration-300" />
          Connected
        </span>
        <span className="text-muted-foreground tabular-nums">
          {events.data.length} {events.data.length === 1 ? "event" : "events"} in the next
          7 days
        </span>
      </span>
    );
    actions = disconnectButton;
  } else if (page.data?.calendar_connected) {
    status = (
      <span className="inline-flex items-center gap-1.5 font-medium text-warning">
        <AlertTriangle className="size-4" />
        Connected, but Google did not answer
      </span>
    );
    actions = (
      <>
        {disconnectButton}
        <Button
          type="button"
          size="sm"
          onClick={() => connect.mutate()}
          disabled={connect.isPending}
        >
          {connectIcon}
          Reconnect
        </Button>
      </>
    );
  } else {
    status = (
      <span className="text-muted-foreground">
        Not connected. Your busy times are not checked.
      </span>
    );
    actions = (
      <Button type="button" onClick={() => connect.mutate()} disabled={connect.isPending}>
        {connectIcon}
        {connect.isPending ? "Opening Google" : "Connect"}
      </Button>
    );
  }

  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
      <span className="grid size-10 shrink-0 place-items-center rounded-lg border bg-card shadow-xs">
        <GoogleCalendarIcon />
      </span>
      <div className="min-w-0 flex-1 basis-56">
        <p className="font-semibold">Google Calendar</p>
        <div className="mt-0.5 text-sm">{status}</div>
      </div>
      <div className="flex items-center gap-2">{actions}</div>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title="Disconnect Google Calendar?"
        description="New calls stop getting a Google Meet link and use your own link instead, if you set one. Your busy times no longer block your booking page."
        confirmLabel="Disconnect"
        destructive
        onConfirm={() => disconnect.mutate()}
      />
    </div>
  );
}
