import { Link } from "@tanstack/react-router";
import { cn } from "cn";
import { Video } from "lucide-react";
import { useEffect, useRef } from "react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { Booking } from "../../bindings/Booking";
import type { DateOverride } from "../../bindings/DateOverride";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { formatLongDay, formatTime, lengthLabel, prefersHour12 } from "../../lib/format";
import { ROUTES } from "../../lib/routes";
import { useNow } from "../../hooks/use-now";
import { dayKey } from "../../lib/civil-day";
import { hoursOn } from "../availability/hours-math";
import { AgendaRow } from "../bookings/agenda";
import { dayCall, dayPlan, firstName, isCancelled } from "../bookings/bookings";
import { OPEN_STYLES } from "../landing/day-tape";
import { HourMark } from "../shared/hour-mark";
import { NAME_LINK } from "../shared/link-styles";
import { PanelHeader } from "../shared/panel-card";
import { phaseSpans, zonedTime } from "../../lib/zoned-time";

const HOUR = 60;
const MINUTE_MS = 60_000;
/** Wide enough for a first name inside a half-hour call. */
const HOUR_WIDTH_REM = 7;
/** Where "now" sits in the strip when it first scrolls into view. */
const NOW_AT = 0.3;

export function TodayCardSkeleton() {
  return (
    <Card className="gap-0 overflow-hidden py-0" role="status" aria-label="Loading today">
      <div className="flex min-h-13 items-center border-b pl-5">
        <Skeleton className="h-5 w-56" />
      </div>
      <div className="space-y-4 px-5 py-5">
        <Skeleton className="h-9 w-72" />
        <Skeleton className="h-20 w-full" />
      </div>
    </Card>
  );
}

/** "in 25 min", "in 1 h 5 min", or "starting now". */
function untilLabel(start: string, now: Date): string {
  const minutes = Math.ceil((new Date(start).getTime() - now.getTime()) / MINUTE_MS);
  return minutes <= 0 ? "starting now" : `in ${lengthLabel(minutes)}`;
}

function JoinButton({ booking }: { booking: Booking }) {
  return booking.meeting_url ? (
    <Button asChild>
      <a href={booking.meeting_url} target="_blank" rel="noreferrer">
        <Video />
        Join call
      </a>
    </Button>
  ) : (
    <span className="text-sm text-muted-foreground">No call link</span>
  );
}

/** The one call that matters right now: the one under way, else the next one today. */
function Spotlight({
  current,
  next,
  anyToday,
  timeZone,
  now,
  onOpen,
}: {
  current: Booking | null;
  next: Booking | null;
  anyToday: boolean;
  timeZone: string;
  now: Date;
  onOpen: (booking: Booking) => void;
}) {
  const booking = current ?? next;
  if (!booking) {
    return (
      <p className="border-b px-5 py-4 text-sm text-muted-foreground">
        {anyToday ? "Nothing else today." : "No calls today."}
      </p>
    );
  }
  const end = new Date(
    new Date(booking.slot_start).getTime() + booking.minutes * MINUTE_MS,
  ).toISOString();
  return (
    <div className="flex flex-wrap items-center gap-x-5 gap-y-3 border-b px-5 py-4">
      <span
        className={cn(
          "rounded-full px-2 py-0.5 text-xs font-semibold",
          current ? "bg-signal text-signal-foreground" : "bg-muted text-muted-foreground",
        )}
      >
        {current ? "Now" : "Next"}
      </span>
      <span className="text-clock text-3xl font-medium">
        {formatTime(booking.slot_start, timeZone)}
      </span>
      <div className="min-w-0 flex-1">
        <button
          type="button"
          onClick={() => onOpen(booking)}
          className={cn("block text-lg", NAME_LINK)}
        >
          {booking.invitee_name}
        </button>
        <p className="text-sm text-muted-foreground">
          {current
            ? `Until ${formatTime(end, timeZone)}`
            : untilLabel(booking.slot_start, now)}
        </p>
      </div>
      <JoinButton booking={booking} />
    </div>
  );
}

/**
 * Today on the host's own clock: the call that matters now, a strip of open
 * hours and booked calls with a line at "now", and today's calls as a list.
 */
export function TodayCard({
  bookings,
  rules,
  overrides,
  timeZone,
  onOpen,
}: {
  bookings: Booking[];
  /** Unknown until loaded: then the strip shows calls only and never says "off". */
  rules: AvailabilityRule[] | undefined;
  overrides: DateOverride[] | undefined;
  timeZone: string;
  onOpen: (booking: Booking) => void;
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
  if (!now) return <TodayCardSkeleton />;

  const hour12 = prefersHour12();
  const today = zonedTime(now, timeZone);
  const plan = dayPlan(bookings, timeZone, now.getTime());
  const hoursKnown = rules !== undefined && overrides !== undefined;
  const open = hoursKnown ? hoursOn(dayKey(today), rules, overrides).spans : [];
  const calls = plan.today
    .filter((booking) => !isCancelled(booking))
    .map((booking) => dayCall(booking, timeZone));
  const nowMinutes = today.hour * HOUR + today.minute;
  const callsLeft = calls.filter((call) => call.from >= nowMinutes).length;

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
      <PanelHeader
        title={`Today, ${formatLongDay(now, timeZone, false)}`}
        action={
          calls.length === 0 ? null : (
            <p className="text-right text-sm text-muted-foreground">
              {callsLeft === 0
                ? "No calls left today"
                : `${callsLeft} ${callsLeft === 1 ? "call" : "calls"} left`}
            </p>
          )
        }
      />

      <Spotlight
        current={plan.current}
        next={plan.next}
        anyToday={calls.length > 0}
        timeZone={timeZone}
        now={now}
        onOpen={onOpen}
      />

      {spans.length === 0 ? (
        hoursKnown ? (
          <div className="flex flex-wrap items-center justify-between gap-3 border-b px-5 py-4 last:border-b-0">
            <p className="text-sm text-muted-foreground">
              You're off today, so your link offers no times today.
            </p>
            <Button variant="outline" size="sm" asChild>
              <Link to={ROUTES.dashboardAvailability}>Edit hours</Link>
            </Button>
          </div>
        ) : null
      ) : (
        // A picture of the day; the same calls are listed below it.
        <div aria-hidden="true" className="border-b last:border-b-0">
          <div ref={scroller} className="overflow-x-auto">
            <div
              style={{ minWidth: `${hours.length * HOUR_WIDTH_REM}rem` }}
              className="relative px-5 pt-6 pb-3"
            >
              <div className="relative">
                <div
                  className="grid"
                  style={{
                    gridTemplateColumns: `repeat(${hours.length}, minmax(0, 1fr))`,
                  }}
                >
                  {hours.map((hour) => (
                    <HourMark key={hour} hour={hour} hour12={hour12} />
                  ))}
                </div>
                <div className="relative mt-2 h-12">
                  {open.flatMap((span, index) =>
                    phaseSpans(span.from, span.to).map(({ phase, from, to }) => (
                      <span
                        key={`${index}-${phase}`}
                        style={place(from, to)}
                        className={cn("absolute inset-y-0 border-y", OPEN_STYLES[phase])}
                      />
                    )),
                  )}
                  {calls.map(({ booking, from, to }) => (
                    <button
                      key={booking.unid}
                      type="button"
                      tabIndex={-1}
                      onClick={() => onOpen(booking)}
                      style={place(from, to)}
                      className={cn(
                        "absolute inset-y-0 flex min-w-0 cursor-pointer items-center rounded-md px-1.5 text-sm font-medium transition-colors",
                        // A call that is over is an outline; one to come is solid ink.
                        to <= nowMinutes
                          ? "border border-foreground/40 bg-card text-muted-foreground hover:bg-muted"
                          : "bg-primary text-primary-foreground hover:bg-primary/90",
                      )}
                    >
                      <span className="truncate">{firstName(booking.invitee_name)}</span>
                    </button>
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
          <div className="flex flex-wrap gap-x-4 gap-y-1 px-5 pb-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <span className={cn("h-3 w-5 border-y", OPEN_STYLES.Afternoon)} />
              Open for booking
            </span>
            <span className="flex items-center gap-1.5">
              <span className="h-3 w-5 rounded-sm bg-primary" />
              Booked
            </span>
            <span className="flex items-center gap-1.5">
              <span className="h-3 w-5 rounded-sm border border-foreground/40" />
              Done
            </span>
          </div>
        </div>
      )}

      {plan.today.length === 0 ? null : (
        <ul className="divide-y">
          {plan.today.map((booking) => (
            <AgendaRow
              key={booking.unid}
              booking={booking}
              timeZone={timeZone}
              now={now.getTime()}
              onOpen={() => onOpen(booking)}
            />
          ))}
        </ul>
      )}
    </Card>
  );
}
