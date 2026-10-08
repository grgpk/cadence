import { Video } from "lucide-react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { formatDay, formatTime } from "../../lib/format";
import { TwoLine, type Column } from "../shared/DataTable";
import { BOOKING_STATUS, StatusBadge } from "../shared/StatusBadge";

/** Columns for a list of bookings, shared by the host dashboard and the admin list. */
export const BOOKING_COLUMNS: Column<Booking>[] = [
  {
    header: "When",
    cell: (booking) => (
      <TwoLine
        primary={
          <span className="tabular-nums">
            {formatDay(booking.slot_start, booking.timezone)},{" "}
            {formatTime(booking.slot_start, booking.timezone)}
          </span>
        }
        secondary={booking.timezone.replaceAll("_", " ")}
      />
    ),
  },
  {
    header: "Invitee",
    cell: (booking) => (
      <TwoLine primary={booking.invitee_name} secondary={booking.invitee_email} />
    ),
  },
  {
    header: "Status",
    cell: (booking) => <StatusBadge value={booking.status} statuses={BOOKING_STATUS} />,
  },
  {
    header: "Call",
    className: "text-right",
    cell: (booking) =>
      booking.meeting_url ? (
        <Button variant="outline" size="sm" asChild>
          <a href={booking.meeting_url} target="_blank" rel="noreferrer">
            <Video />
            Join
          </a>
        </Button>
      ) : (
        <span className="text-muted-foreground">No link</span>
      ),
  },
];

export function isUpcoming(booking: Booking, now: number): boolean {
  return new Date(booking.slot_start).getTime() >= now;
}
