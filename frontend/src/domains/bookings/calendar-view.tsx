import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { Booking } from "../../bindings/Booking";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { useIsMobile } from "../../hooks/use-mobile";
import { useNow } from "../../hooks/use-now";
import {
  addDays,
  type CivilDay,
  dayKey,
  daysBetween,
  longDayLabel,
  mondayOf,
  type MonthCell,
  monthLabel,
  monthWeeks,
  sameDay,
  shiftMonth,
  todayIn,
  weekdayIndex,
  weekFrom,
  weekLabel,
} from "../../lib/civil-day";
import {
  clockMinutes,
  formatTime,
  hourLabel,
  prefersHour12,
  weekdayName,
} from "../../lib/format";
import { placeOnWeek } from "../dashboard/week-grid";
import { weekHours, type WeekHours } from "../dashboard/week-hours";
import { OPEN_STYLES } from "../landing/day-tape";
import { SegmentedControl } from "../shared/segmented-control";
import { callEnd, dayPhase, phaseSpans, zonedTime } from "../../lib/zoned-time";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { dayCall, isCancelled, lanes } from "./bookings";

export const CALENDAR_MODE = { day: "day", week: "week", month: "month" } as const;
export type CalendarMode = (typeof CALENDAR_MODE)[keyof typeof CALENDAR_MODE];

const MODES = [
  { value: CALENDAR_MODE.day, label: "Day" },
  { value: CALENDAR_MODE.week, label: "Week" },
  { value: CALENDAR_MODE.month, label: "Month" },
] as const;

const HOUR = 60;
/** Height of one hour on the day and week grids. */
const HOUR_PX = 56;
/** Calls a month cell lists before it says how many more. */
const MONTH_CELL_CALLS = 3;

type Call = ReturnType<typeof dayCall>;

/**
 * The host's calls on a calendar: one day or a week as columns of hours, or a
 * whole month. Open hours are shaded under the calls, as on the availability page.
 */
export function CalendarView({
  bookings,
  rules,
  timeZone,
  mode: chosenMode,
  offset,
  onNavigate,
  onOpen,
}: {
  bookings: Booking[];
  rules: AvailabilityRule[];
  timeZone: string;
  /** Unset until the host picks one: a week, or a day on a phone. */
  mode?: CalendarMode;
  /** Days, weeks or months away from today's, by `mode`. */
  offset: number;
  onNavigate: (next: { mode: CalendarMode; offset: number }) => void;
  onOpen: (booking: Booking) => void;
}) {
  const isMobile = useIsMobile();
  const mode = chosenMode ?? (isMobile ? CALENDAR_MODE.day : CALENDAR_MODE.week);
  // Each minute, so the line at "now" and what counts as past keep up.
  const now = useNow()?.getTime() ?? Date.now();
  const today = todayIn(timeZone, now);

  const callsByDay = new Map<string, Call[]>();
  for (const booking of bookings) {
    if (isCancelled(booking)) continue;
    const call = dayCall(booking, timeZone);
    const key = dayKey(call.day);
    callsByDay.set(key, [...(callsByDay.get(key) ?? []), call]);
  }
  const callsOn = (day: CivilDay) => callsByDay.get(dayKey(day)) ?? [];

  const month =
    mode === CALENDAR_MODE.month ? shiftMonth(today.year, today.month, offset) : null;
  const days =
    mode === CALENDAR_MODE.day
      ? [addDays(today, offset)]
      : mode === CALENDAR_MODE.week
        ? weekFrom(addDays(mondayOf(today), offset * 7))
        : [];
  const label = month
    ? monthLabel(month.year, month.month)
    : mode === CALENDAR_MODE.day && days[0]
      ? longDayLabel(days[0])
      : days[0]
        ? weekLabel(days[0])
        : "";
  const unit = mode === CALENDAR_MODE.day ? "day" : mode;

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b py-2 pr-3 pl-5">
        <div className="flex min-w-0 items-center gap-1">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={offset === 0}
            onClick={() => onNavigate({ mode, offset: 0 })}
          >
            Today
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Previous ${unit}`}
            onClick={() => onNavigate({ mode, offset: offset - 1 })}
          >
            <ChevronLeft />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Next ${unit}`}
            onClick={() => onNavigate({ mode, offset: offset + 1 })}
          >
            <ChevronRight />
          </Button>
          <h2 className="ml-2 truncate font-semibold tabular-nums" aria-live="polite">
            {label}
          </h2>
        </div>
        <SegmentedControl
          label="Calendar range"
          size="sm"
          value={mode}
          options={MODES}
          onChange={(next) => onNavigate({ mode: next, offset: 0 })}
        />
      </div>
      {month ? (
        <MonthGrid
          days={monthWeeks(month.year, month.month, { rows: "six" }).flat()}
          today={today}
          timeZone={timeZone}
          callsOn={callsOn}
          onOpen={onOpen}
          onPickDay={(day) =>
            onNavigate({ mode: CALENDAR_MODE.day, offset: daysBetween(today, day) })
          }
        />
      ) : (
        <TimeGrid
          days={days}
          today={today}
          now={now}
          timeZone={timeZone}
          rules={rules}
          callsOn={callsOn}
          onOpen={onOpen}
        />
      )}
    </Card>
  );
}

/** Days side by side as columns of hours, Google Calendar style. */
function TimeGrid({
  days,
  today,
  now,
  timeZone,
  rules,
  callsOn,
  onOpen,
}: {
  days: CivilDay[];
  today: CivilDay;
  now: number;
  timeZone: string;
  rules: AvailabilityRule[];
  callsOn: (day: CivilDay) => Call[];
  onOpen: (booking: Booking) => void;
}) {
  const calls = days.flatMap(callsOn);
  // The open hours, stretched to any call booked outside them.
  const open = weekHours(rules);
  const hours: WeekHours = {
    first: Math.min(open.first, ...calls.map((call) => Math.floor(call.from / HOUR))),
    last: Math.max(open.last, ...calls.map((call) => Math.ceil(call.to / HOUR))),
  };
  const hourList = Array.from(
    { length: hours.last - hours.first },
    (_, index) => hours.first + index,
  );
  const hour12 = prefersHour12();
  const clock = zonedTime(new Date(now), timeZone);
  const nowMinutes = clock.hour * HOUR + clock.minute;
  const columns = {
    gridTemplateColumns: `3.5rem repeat(${days.length}, minmax(0, 1fr))`,
  };
  // A week's seven columns scroll sideways on a phone rather than squeeze.
  const wide = days.length > 1 ? "min-w-[44rem]" : undefined;

  return (
    <div className="overflow-x-auto">
      <div className={wide}>
        <div style={columns} className="grid border-b">
          <span />
          {days.map((day) => {
            const isToday = sameDay(day, today);
            return (
              <div
                key={dayKey(day)}
                className={cn(
                  "grid justify-items-center gap-0.5 py-2",
                  daysBetween(today, day) < 0 && "opacity-50",
                )}
              >
                <span
                  className={cn(
                    "text-xs font-medium tracking-wide uppercase",
                    isToday ? "text-signal" : "text-muted-foreground",
                  )}
                >
                  {weekdayName(weekdayIndex(day)).slice(0, 3)}
                </span>
                <span
                  className={cn(
                    "text-clock grid size-9 place-items-center rounded-full text-lg",
                    isToday && "bg-signal font-semibold text-signal-foreground",
                  )}
                >
                  {day.day}
                </span>
              </div>
            );
          })}
        </div>
        <div style={columns} className="grid">
          <div aria-hidden="true" className="relative">
            {hourList.map((hour, index) => (
              <span
                key={hour}
                style={{ top: index * HOUR_PX }}
                className="text-clock absolute right-2 -translate-y-1/2 text-xs whitespace-nowrap text-muted-foreground first:translate-y-0.5"
              >
                {hourLabel(hour, hour12)}
              </span>
            ))}
          </div>
          {days.map((day) => {
            const weekday = weekdayIndex(day);
            const isToday = sameDay(day, today);
            const showNow =
              isToday &&
              nowMinutes >= hours.first * HOUR &&
              nowMinutes <= hours.last * HOUR;
            // How far down the column time has already gone: all of a past day, today up to now.
            const ahead = daysBetween(today, day);
            const pastUntil =
              ahead < 0
                ? hours.last * HOUR
                : ahead === 0
                  ? Math.min(Math.max(nowMinutes, hours.first * HOUR), hours.last * HOUR)
                  : null;
            return (
              <div
                key={dayKey(day)}
                style={{ height: hourList.length * HOUR_PX }}
                className="relative border-l"
              >
                {rules
                  .filter((rule) => rule.weekday === weekday)
                  .flatMap((rule) =>
                    phaseSpans(
                      clockMinutes(rule.start_time),
                      clockMinutes(rule.end_time),
                    ).map((span) => {
                      const place = placeOnWeek(hours, span.from, span.to);
                      return (
                        <span
                          key={`${rule.id}-${span.phase}`}
                          aria-hidden="true"
                          style={{ top: place.left, height: place.width }}
                          // Open hours sit behind the calls: a hint, not a fill.
                          className={cn(
                            "absolute inset-x-0 border-0 opacity-45",
                            OPEN_STYLES[span.phase],
                          )}
                        />
                      );
                    }),
                  )}
                {pastUntil !== null && pastUntil > hours.first * HOUR ? (
                  <span
                    aria-hidden="true"
                    style={{
                      top: 0,
                      height: placeOnWeek(hours, hours.first * HOUR, pastUntil).width,
                    }}
                    className="absolute inset-x-0 bg-background/45"
                  />
                ) : null}
                {hourList.map((hour, index) => (
                  <span
                    key={hour}
                    aria-hidden="true"
                    style={{ top: index * HOUR_PX }}
                    className="absolute inset-x-0 border-t border-border/70"
                  />
                ))}
                {lanes(callsOn(day)).map(({ booking, from, to, lane, lanes: width }) => {
                  const place = placeOnWeek(hours, from, to);
                  const time = formatTime(booking.slot_start, timeZone);
                  const end = formatTime(
                    callEnd(booking.slot_start, booking.minutes).toISOString(),
                    timeZone,
                  );
                  const past = new Date(booking.slot_start).getTime() < now;
                  // A call of 45 minutes or more has room to say when it ends.
                  const roomy = to - from >= 45;
                  return (
                    <button
                      key={booking.unid}
                      type="button"
                      onClick={() => onOpen(booking)}
                      aria-label={`${booking.invitee_name}, ${time} to ${end}`}
                      title={`${booking.invitee_name}, ${time} to ${end}`}
                      style={{
                        top: place.left,
                        height: place.width,
                        left: `calc(${(lane / width) * 100}% + 2px)`,
                        width: `calc(${100 / width}% - 4px)`,
                      }}
                      className={cn(
                        "absolute flex min-h-5 flex-col overflow-hidden rounded-md border-l-4 px-1.5 py-0.5 text-left text-xs leading-tight shadow-xs transition-colors outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
                        // A call that is over fades; one to come is solid.
                        past
                          ? "border-muted-foreground/50 bg-muted text-muted-foreground hover:bg-muted/80"
                          : "border-noon bg-primary text-primary-foreground hover:bg-primary/85",
                      )}
                    >
                      <span className="truncate font-semibold">
                        {booking.invitee_name}
                      </span>
                      <span className="text-clock truncate opacity-80">
                        {roomy ? `${time} to ${end}` : time}
                      </span>
                    </button>
                  );
                })}
                {showNow ? (
                  <span
                    aria-hidden="true"
                    style={{ top: placeOnWeek(hours, nowMinutes, nowMinutes).left }}
                    className="pointer-events-none absolute inset-x-0 z-10 h-0.5 -translate-y-1/2 bg-signal"
                  >
                    <span className="absolute top-1/2 -left-1.5 size-3 -translate-y-1/2 rounded-full bg-signal" />
                  </span>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
      {calls.length === 0 ? (
        <p className="border-t px-5 py-3 text-sm text-muted-foreground">
          {days.length === 1 ? "No calls this day." : "No calls this week."}
        </p>
      ) : null}
    </div>
  );
}

/** A call's dot, in the color of the part of the day it starts in. */
function phaseDot(call: Call): string {
  return PHASE_STYLES[dayPhase(Math.floor(call.from / HOUR))].rail;
}

/** A month as six weeks of days, each listing its first few calls. */
function MonthGrid({
  days,
  today,
  timeZone,
  callsOn,
  onOpen,
  onPickDay,
}: {
  days: MonthCell[];
  today: CivilDay;
  timeZone: string;
  callsOn: (day: CivilDay) => Call[];
  onOpen: (booking: Booking) => void;
  onPickDay: (day: CivilDay) => void;
}) {
  return (
    <div>
      <div className="grid grid-cols-7 border-b">
        {Array.from({ length: 7 }, (_, index) => (
          <span
            key={weekdayName(index)}
            className="py-2 text-center text-xs font-medium tracking-wide text-muted-foreground uppercase"
          >
            {weekdayName(index).slice(0, 3)}
          </span>
        ))}
      </div>
      <div className="grid grid-cols-7">
        {days.map((day, index) => {
          const calls = [...callsOn(day)].sort((a, b) => a.from - b.from);
          const isToday = sameDay(day, today);
          const past = daysBetween(today, day) < 0;
          const more = calls.length - MONTH_CELL_CALLS;
          return (
            <div
              key={dayKey(day)}
              className={cn(
                "flex min-h-16 flex-col gap-0.5 border-b p-1 sm:min-h-28",
                index % 7 !== 0 && "border-l",
                !day.inMonth && "bg-muted/40",
              )}
            >
              <button
                type="button"
                onClick={() => onPickDay(day)}
                aria-label={`${longDayLabel(day)}${calls.length ? `, ${calls.length} ${calls.length === 1 ? "call" : "calls"}` : ""}`}
                className={cn(
                  "text-clock grid size-7 shrink-0 place-items-center self-center rounded-full text-sm transition-colors outline-none hover:bg-muted focus-visible:ring-[3px] focus-visible:ring-ring/50",
                  (!day.inMonth || past) && "text-muted-foreground",
                  isToday &&
                    "bg-signal font-semibold text-signal-foreground hover:bg-signal/85",
                )}
              >
                {day.day}
              </button>
              {/* A phone has no room for names: one dot per call, and the day opens them. */}
              {calls.length > 0 ? (
                <button
                  type="button"
                  tabIndex={-1}
                  aria-hidden="true"
                  onClick={() => onPickDay(day)}
                  className={cn(
                    "flex flex-wrap justify-center gap-1 pt-0.5 sm:hidden",
                    past && "opacity-50",
                  )}
                >
                  {calls.slice(0, MONTH_CELL_CALLS).map((call) => (
                    <span
                      key={call.booking.unid}
                      className={cn("size-1.5 rounded-full", phaseDot(call))}
                    />
                  ))}
                </button>
              ) : null}
              {calls.slice(0, MONTH_CELL_CALLS).map((call) => {
                const { booking } = call;
                const time = formatTime(booking.slot_start, timeZone);
                return (
                  <button
                    key={booking.unid}
                    type="button"
                    onClick={() => onOpen(booking)}
                    title={`${booking.invitee_name}, ${time}`}
                    className={cn(
                      "flex min-w-0 items-center gap-1 rounded-sm px-1 text-left text-xs transition-colors outline-none hover:bg-muted focus-visible:ring-[3px] focus-visible:ring-ring/50 max-sm:hidden",
                      past && "text-muted-foreground",
                    )}
                  >
                    <span
                      aria-hidden="true"
                      className={cn("size-1.5 shrink-0 rounded-full", phaseDot(call))}
                    />
                    <span className="text-clock shrink-0 text-muted-foreground">
                      {time}
                    </span>
                    <span className="truncate font-medium">{booking.invitee_name}</span>
                  </button>
                );
              })}
              {more > 0 ? (
                <button
                  type="button"
                  onClick={() => onPickDay(day)}
                  className="rounded-sm px-1 text-left text-xs font-medium text-muted-foreground transition-colors outline-none hover:bg-muted hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50 max-sm:text-center"
                >
                  +{more}
                  <span className="max-sm:hidden"> more</span>
                </button>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
