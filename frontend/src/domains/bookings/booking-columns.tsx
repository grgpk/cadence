import { Video } from "lucide-react";

import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { formatDay, formatTime, zoneName } from "../../lib/format";
import { TwoLine, type Column } from "../shared/DataTable";
import { BOOKING_STATUS, StatusBadge } from "../shared/StatusBadge";

/**
 * Columns for a list of bookings. A host reads every call in their own
 * timezone, with the invitee's time under it when the two differ. Without a
 * host timezone, as in the admin list of every host, each call is in the
 * invitee's.
 */
export function bookingColumns(hostTimezone?: string): Column<Booking>[] {
  return [
    {
      header: "When",
      cell: (booking) => {
        const zone = hostTimezone ?? booking.timezone;
        return (
          <TwoLine
            primary={
              <span className="text-clock text-[0.9375rem]">
                {formatDay(booking.slot_start, zone)},{" "}
                {formatTime(booking.slot_start, zone)}
              </span>
            }
            secondary={
              zone === booking.timezone
                ? zoneName(zone)
                : `${formatTime(booking.slot_start, booking.timezone)} for them, ${zoneName(booking.timezone)}`
            }
          />
        );
      },
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
}
