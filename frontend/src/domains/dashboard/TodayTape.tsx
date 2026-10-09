import { Link } from "@tanstack/react-router";
import { cn } from "cn";
import { useEffect, useRef, useState } from "react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { clockMinutes } from "../../lib/format";
import { ROUTES } from "../../lib/routes";
import { isCancelled } from "../bookings/bookings";
import { OPEN_STYLES } from "../landing/DayTape";
import { phaseSpans, zonedTime } from "../widget/zoned-time";

const HOUR = 60;
const MINUTE_MS = 60_000;
/** A booking does not carry its length; used when no rule covers its start. */
const DEFAULT_CALL_MINUTES = 30;
/** Wide enough for a first name inside a half-hour call. */
const HOUR_WIDTH_REM = 7;
/** Where "now" sits in the strip when it first scrolls into view. */
const NOW_AT = 0.3;

/** The current time, refreshed each minute. `null` until the browser has mounted. */
function useNow(): Date | null {
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => {
    setNow(new Date());
    const timer = window.setInterval(() => setNow(new Date()), MINUTE_MS);
    return () => window.clearInterval(timer);
  }, []);
  return now;
}

/**
 * Today on the host's own clock: open hours, booked calls and a line at "now".
 * The same strip as the landing page, holding the host's real day.
 */
export function TodayTape({
  rules,
  bookings,
  timeZone,
}: {
  rules: AvailabilityRule[];
  bookings: Booking[];
  timeZone: string;
}) {
  const now = useNow();
  const scroller = useRef<HTMLDivElement>(null);
  const nowLine = useRef<HTMLDivElement>(null);
  const mounted = now !== null;
  // A long day is wider than the card: start with "now" in view, once.
  useEffect(() => {
    const strip = scroller.current;
    const line = nowLine.current;
    if (mounted && strip && line) {
      strip.scrollLeft = line.offsetLeft - strip.clientWidth * NOW_AT;
    }
  }, [mounted]);
  if (!now) {
    return (
      <Card
        className="gap-0 overflow-hidden py-0"
        role="status"
        aria-label="Loading today"
      >
        <div className="border-b px-5 py-3.5">
          <Skeleton className="h-6 w-56" />
        </div>
        <div className="px-5 py-5">
          <Skeleton className="h-20 w-full" />
        </div>
      </Card>
    );
  }

  const today = zonedTime(now, timeZone);
  // The API counts weekdays from Monday.
  const weekday =
    (new Date(Date.UTC(today.year, today.month - 1, today.day)).getUTCDay() + 6) % 7;
  const open = rules
    .filter((rule) => rule.weekday === weekday)
    .map((rule) => ({
      id: rule.id,
      from: clockMinutes(rule.start_time),
      to: clockMinutes(rule.end_time),
      slotMinutes: rule.slot_minutes,
    }));
  const calls = bookings
    .filter((booking) => !isCancelled(booking))
    .map((booking) => ({
      booking,
      at: zonedTime(new Date(booking.slot_start), timeZone),
    }))
    .filter(
      ({ at }) =>
        at.year === today.year && at.month === today.month && at.day === today.day,
    )
    .map(({ booking, at }) => {
      const from = at.hour * HOUR + at.minute;
      const rule = open.find((hours) => from >= hours.from && from < hours.to);
      return { booking, from, to: from + (rule?.slotMinutes ?? DEFAULT_CALL_MINUTES) };
    });
  const nowMinutes = today.hour * HOUR + today.minute;
  const callsLeft = calls.filter((call) => call.from >= nowMinutes).length;

  const dateLabel = new Intl.DateTimeFormat("en", {
    weekday: "long",
    day: "numeric",
    month: "long",
    timeZone,
  }).format(now);

  const spans = [...open, ...calls];
  const firstHour = Math.floor(Math.min(...spans.map((span) => span.from)) / HOUR);
  const lastHour = Math.ceil(Math.max(...spans.map((span) => span.to)) / HOUR);
  const hours = Array.from(
    { length: lastHour - firstHour },
    (_, index) => firstHour + index,
  );
  const place = (from: number, to: number) => ({
    left: `${((from - firstHour * HOUR) / (hours.length * HOUR)) * 100}%`,
    width: `${((to - from) / (hours.length * HOUR)) * 100}%`,
  });

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-b px-5 py-3.5">
        <h2 className="font-semibold">Today, {dateLabel}</h2>
        {spans.length === 0 ? null : (
          <p className="text-sm text-muted-foreground">
            {callsLeft === 0
              ? "No calls left today"
              : `${callsLeft} ${callsLeft === 1 ? "call" : "calls"} left`}
          </p>
        )}
      </div>
      {spans.length === 0 ? (
        <div className="flex flex-wrap items-center justify-between gap-3 px-5 py-4">
          <p className="text-sm text-muted-foreground">
            You take no calls today, so your link offers no times for it.
          </p>
          <Button variant="outline" size="sm" asChild>
            <Link to={ROUTES.dashboardAvailability}>Edit hours</Link>
          </Button>
        </div>
      ) : (
        <div ref={scroller} className="overflow-x-auto">
          {/* A picture of the day; the same calls are listed under "Next up". */}
          <div
            aria-hidden="true"
            style={{ minWidth: `${hours.length * HOUR_WIDTH_REM}rem` }}
            className="relative px-5 pt-6 pb-5"
          >
            <div className="relative">
              <div
                className="grid"
                style={{ gridTemplateColumns: `repeat(${hours.length}, minmax(0, 1fr))` }}
              >
                {hours.map((hour) => (
                  <div key={hour}>
                    <p className="text-clock text-sm font-medium text-muted-foreground">
                      {String(hour).padStart(2, "0")}:00
                    </p>
                    <div className="relative mt-1 h-2.5 border-t border-foreground/25">
                      <span className="absolute top-0 left-0 h-2.5 w-px bg-foreground/40" />
                      <span className="absolute top-0 left-1/2 h-1.5 w-px bg-foreground/25" />
                    </div>
                  </div>
                ))}
              </div>
              <div className="relative mt-2 h-12">
                {open.flatMap((hours) =>
                  phaseSpans(hours.from, hours.to).map(({ phase, from, to }) => (
                    <span
                      key={`${hours.id}-${phase}`}
                      style={place(from, to)}
                      className={cn("absolute inset-y-0 border-y", OPEN_STYLES[phase])}
                    />
                  )),
                )}
                {calls.map(({ booking, from, to }) => (
                  <span
                    key={booking.unid}
                    style={place(from, to)}
                    className={cn(
                      "absolute inset-y-0 flex min-w-0 items-center rounded-md bg-primary px-1.5 text-sm font-medium text-primary-foreground",
                      to <= nowMinutes && "opacity-45",
                    )}
                  >
                    <span className="truncate">
                      {booking.invitee_name.trim().split(/\s+/)[0]}
                    </span>
                  </span>
                ))}
              </div>
              {nowMinutes >= firstHour * HOUR && nowMinutes <= lastHour * HOUR ? (
                <div
                  ref={nowLine}
                  style={{ left: place(nowMinutes, nowMinutes).left }}
                  className="pointer-events-none absolute -top-5 bottom-0 flex -translate-x-1/2 flex-col items-center"
                >
                  <span className="rounded-full bg-signal px-2 py-0.5 text-xs font-semibold text-signal-foreground">
                    now
                  </span>
                  <span className="w-0.5 flex-1 bg-signal" />
                </div>
              ) : null}
            </div>
          </div>
        </div>
      )}
    </Card>
  );
}
