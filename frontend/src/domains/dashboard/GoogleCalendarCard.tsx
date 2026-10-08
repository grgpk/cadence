import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "../../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { QUERY_KEYS } from "../../lib/query-keys";
import {
  connectGoogleCalendar,
  disconnectGoogleCalendar,
  getCalendarEvents,
} from "../widget/cadence-api";

export function GoogleCalendarCard() {
  const queryClient = useQueryClient();
  const calendar = useQuery({
    queryKey: QUERY_KEYS.calendarEvents,
    queryFn: getCalendarEvents,
    retry: false,
  });
  const disconnect = useMutation({
    mutationFn: disconnectGoogleCalendar,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.calendarEvents });
      toast.success("Google Calendar disconnected");
    },
    onError: () => toast.error("Google Calendar was not disconnected. Try again."),
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Google Calendar</CardTitle>
        <CardDescription>
          Optional. Keeps your busy times off your booking page and adds a Google Meet
          link to each call.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-wrap items-center gap-3">
        {calendar.isPending ? (
          <Skeleton className="h-9 w-48" />
        ) : calendar.isError ? (
          <>
            <Button
              onClick={() =>
                void connectGoogleCalendar().catch(() =>
                  toast.error("Google sign-in could not be started. Try again."),
                )
              }
            >
              Connect Google Calendar
            </Button>
            <p className="text-sm text-muted-foreground">Not connected</p>
          </>
        ) : (
          <>
            <span className="inline-flex items-center gap-2 text-sm font-medium text-success">
              <CheckCircle2 className="size-4" />
              Connected
            </span>
            <span className="text-sm text-muted-foreground tabular-nums">
              {calendar.data.length} events in the next 7 days
            </span>
            <Button
              variant="outline"
              size="sm"
              className="ml-auto"
              onClick={() => disconnect.mutate()}
              disabled={disconnect.isPending}
            >
              {disconnect.isPending ? "Disconnecting" : "Disconnect"}
            </Button>
          </>
        )}
      </CardContent>
    </Card>
  );
}
