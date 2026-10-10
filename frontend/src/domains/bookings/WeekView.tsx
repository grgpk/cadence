import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useState } from "react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { clockMinutes, formatTime, weekdayName } from "../../lib/format";
import { weekHours } from "../dashboard/week-hours";
import { placeOnWeek, WeekRuler } from "../dashboard/WeekGrid";
import { OPEN_STYLES } from "../landing/DayTape";
import { useNow } from "../../lib/use-now";
import { phaseSpans, zonedTime } from "../widget/zoned-time";
import {
  dayCall,
  firstName,
  isCancelled,
  sameDay,
  weekDays,
  type WeekDay,
} from "./bookings";

const HOUR = 60;
/** Day name and the week's ruler: the same two columns in every row. */
const ROW = "grid gap-x-3 gap-y-1.5 px-5 sm:grid-cols-[7.5rem_minmax(0,1fr)]";

const MONTH = new Intl.DateTimeFormat("en", { month: "long", timeZone: "UTC" });

function monthName(day: WeekDay): string {
  return MONTH.format(new Date(Date.UTC(day.year, day.month - 1, day.day)));
}

/** "12 to 18 October", or "26 October to 1 November" across a month end. */
function weekLabel(days: WeekDay[]): string {
  const first = days[0];
  const last = days.at(-1);
  if (!first || !last) return "";
  const start =
    first.month === last.month ? first.day : `${first.day} ${monthName(first)}`;
  return `${start} to ${last.day} ${monthName(last)}`;
}

/**
 * A week of calls on the host's own clock: each day's open hours with the
 * booked calls on top. The same calls as the list, laid out by time.
 */
export function WeekView({
  bookings,
  rules,
  timeZone,
  onOpen,
}: {
  bookings: Booking[];
  rules: AvailabilityRule[];
  timeZone: string;
  onOpen: (booking: Booking) => void;
}) {
  const [offset, setOffset] = useState(0);
  // Each minute, so the line at "now" and what counts as past keep up.
  const now = useNow()?.getTime() ?? Date.now();
  const today = zonedTime(new Date(now), timeZone);
  const days = weekDays(now, timeZone, offset);

  const calls = bookings
    .filter((booking) => !isCancelled(booking))
    .map((booking) => {
      const call = dayCall(booking, timeZone);
      return { ...call, weekday: days.findIndex((day) => sameDay(day, call.day)) };
    })
    .filter((call) => call.weekday >= 0);

  // The ruler covers the weekly hours and any call booked outside them.
  const open = weekHours(rules);
  const week = {
    first: Math.min(open.first, ...calls.map((call) => Math.floor(call.from / HOUR))),
    last: Math.max(open.last, ...calls.map((call) => Math.ceil(call.to / HOUR))),
  };
  const nowMinutes = today.hour * HOUR + today.minute;

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b py-2 pr-3 pl-5">
        <h2 className="font-semibold">{weekLabel(days)}</h2>
        <div className="flex items-center gap-1">
          {offset === 0 ? null : (
            <Button type="button" variant="ghost" size="sm" onClick={() => setOffset(0)}>
              This week
            </Button>
          )}
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="Previous week"
            onClick={() => setOffset(offset - 1)}
          >
            <ChevronLeft />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="Next week"
            onClick={() => setOffset(offset + 1)}
          >
            <ChevronRight />
          </Button>
        </div>
      </div>
      <WeekRuler week={week} row={ROW} />
      <ul className="divide-y">
        {days.map((day, weekday) => {
          const isToday = sameDay(day, today);
          const dayCalls = calls.filter((call) => call.weekday === weekday);
          return (
            <li key={day.day} className={cn(ROW, "py-2.5 sm:items-center")}>
              <p
                className={cn(
                  "text-sm",
                  isToday ? "font-semibold" : "text-muted-foreground",
                )}
              >
                {weekdayName(weekday).slice(0, 3)}{" "}
                <span className="text-clock text-[0.9375rem]">{day.day}</span>
                {isToday ? (
                  <span className="ml-1.5 rounded-full bg-signal px-1.5 py-px text-xs font-semibold text-signal-foreground">
                    today
                  </span>
                ) : null}
              </p>
              <div className="relative h-10 rounded-md bg-muted/70">
                {rules
                  .filter((hours) => hours.weekday === weekday)
                  .flatMap((hours) =>
                    phaseSpans(
                      clockMinutes(hours.start_time),
                      clockMinutes(hours.end_time),
                    ).map((span) => (
                      <span
                        key={`${hours.id}-${span.phase}`}
                        aria-hidden="true"
                        style={placeOnWeek(week, span.from, span.to)}
                        className={cn(
                          "absolute inset-y-0 border-y",
                          OPEN_STYLES[span.phase],
                        )}
                      />
                    )),
                  )}
                {dayCalls.map(({ booking, from, to }) => (
                  <button
                    key={booking.unid}
                    type="button"
                    style={placeOnWeek(week, from, to)}
                    onClick={() => onOpen(booking)}
                    aria-label={`${booking.invitee_name}, ${formatTime(booking.slot_start, timeZone)}`}
                    title={`${booking.invitee_name}, ${formatTime(booking.slot_start, timeZone)}`}
                    className={cn(
                      "absolute inset-y-0 flex min-w-1.5 items-center overflow-hidden rounded-md px-1.5 text-sm font-medium outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
                      // A call that is over is an outline; one to come is solid ink.
                      new Date(booking.slot_start).getTime() < now
                        ? "border border-foreground/40 bg-card text-muted-foreground hover:bg-muted"
                        : "bg-primary text-primary-foreground hover:bg-primary/85",
                    )}
                  >
                    <span className="truncate">{firstName(booking.invitee_name)}</span>
                  </button>
                ))}
                {isToday &&
                nowMinutes >= week.first * HOUR &&
                nowMinutes <= week.last * HOUR ? (
                  <span
                    aria-hidden="true"
                    style={{ left: placeOnWeek(week, nowMinutes, nowMinutes).left }}
                    className="pointer-events-none absolute -inset-y-1 w-0.5 -translate-x-1/2 bg-signal"
                  />
                ) : null}
              </div>
              {/* On a phone the blocks are too narrow to read, so the calls are also listed. */}
              {dayCalls.length === 0 ? null : (
                <ul className="flex flex-wrap gap-1.5 sm:hidden">
                  {dayCalls.map(({ booking }) => (
                    <li key={booking.unid}>
                      <button
                        type="button"
                        onClick={() => onOpen(booking)}
                        className="rounded-md border bg-card px-2 py-1 text-sm outline-none hover:border-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50"
                      >
                        <span className="text-clock text-[0.9375rem] font-medium">
                          {formatTime(booking.slot_start, timeZone)}
                        </span>{" "}
                        {booking.invitee_name}
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </li>
          );
        })}
      </ul>
      {calls.length === 0 ? (
        <p className="border-t px-5 py-3 text-sm text-muted-foreground">
          No calls this week.
        </p>
      ) : null}
    </Card>
  );
}
