import { cn } from "cn";
import { Video } from "lucide-react";

import type { Booking } from "../../bindings/Booking";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { formatTime } from "../../lib/format";
import { NAME_LINK } from "../shared/link-styles";
import { BOOKING_STATUS, StatusBadge } from "../shared/status-badge";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { dayPhase, zonedTime } from "../../lib/zoned-time";
import { type Attendance, agendaDays, attendanceOf, isCancelled } from "./bookings";

const ATTENDANCE: Record<
  Exclude<Attendance, null>,
  { label: string; className: string }
> = {
  confirmed: { label: "Confirmed", className: "border-success/40 text-success" },
  unconfirmed: { label: "Not confirmed yet", className: "text-muted-foreground" },
  no_show: { label: "No-show", className: "border-destructive/40 text-destructive" },
};

/** Whether the invitee said they are coming, or did not come. */
function AttendanceBadge({ attendance }: { attendance: Attendance }) {
  if (attendance === null) return null;
  const { label, className } = ATTENDANCE[attendance];
  return (
    <Badge variant="outline" className={className}>
      {label}
    </Badge>
  );
}

/** One call, with the color of the part of the day it falls in. */
export function AgendaRow({
  booking,
  timeZone,
  now,
  tracksAttendance = false,
  onOpen,
}: {
  booking: Booking;
  timeZone: string;
  now: number;
  /** Business: show whether the invitee is coming, or did not come. */
  tracksAttendance?: boolean;
  onOpen: () => void;
}) {
  const { hour } = zonedTime(new Date(booking.slot_start), timeZone);
  const cancelled = isCancelled(booking);
  const toCome = new Date(booking.slot_start).getTime() >= now;
  return (
    <li className="flex items-center gap-4 px-5 py-3">
      <span
        aria-hidden="true"
        className={cn(
          "h-9 w-[3px] shrink-0 rounded-full",
          cancelled ? "bg-border" : PHASE_STYLES[dayPhase(hour)].rail,
        )}
      />
      <span
        className={cn(
          "w-20 shrink-0 text-clock text-lg font-medium",
          cancelled && "text-muted-foreground line-through",
        )}
      >
        {formatTime(booking.slot_start, timeZone)}
      </span>
      <div className="min-w-0 flex-1">
        <button
          type="button"
          onClick={onOpen}
          className={cn(
            "block",
            NAME_LINK,
            cancelled && "text-muted-foreground line-through",
          )}
        >
          {booking.invitee_name}
        </button>
        <p className="truncate text-sm text-muted-foreground">{booking.invitee_email}</p>
      </div>
      {tracksAttendance ? (
        <AttendanceBadge attendance={attendanceOf(booking, now)} />
      ) : null}
      {cancelled ? (
        <StatusBadge value={booking.status} statuses={BOOKING_STATUS} />
      ) : !toCome ? null : booking.meeting_url ? (
        <Button variant="outline" size="sm" asChild>
          <a href={booking.meeting_url} target="_blank" rel="noreferrer">
            <Video />
            Join call
          </a>
        </Button>
      ) : (
        <span className="text-sm text-muted-foreground">No call link</span>
      )}
    </li>
  );
}

/**
 * Calls as a list grouped by day, in the order given. A host reads every call
 * on their own clock; without a host timezone each call is on its invitee's.
 */
export function Agenda({
  bookings,
  timeZone,
  onOpen,
  tracksAttendance = false,
  heading: Heading = "h3",
}: {
  bookings: Booking[];
  timeZone?: string;
  onOpen: (booking: Booking) => void;
  tracksAttendance?: boolean;
  /** `h2` where the list sits straight under the page title. */
  heading?: "h2" | "h3";
}) {
  const now = Date.now();
  return agendaDays(bookings, timeZone ?? bookings[0]?.timezone ?? "UTC", now).map(
    (day) => (
      <section key={day.key}>
        <Heading className="border-b bg-muted/60 px-5 py-1.5 text-xs font-medium text-muted-foreground">
          {day.label}
        </Heading>
        <ul className="divide-y border-b last:border-b-0">
          {day.bookings.map((booking) => (
            <AgendaRow
              key={booking.unid}
              booking={booking}
              timeZone={timeZone ?? booking.timezone}
              now={now}
              tracksAttendance={tracksAttendance}
              onOpen={() => onOpen(booking)}
            />
          ))}
        </ul>
      </section>
    ),
  );
}
