import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useState, type KeyboardEvent } from "react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { DateOverride } from "../../bindings/DateOverride";
import { Button } from "../../components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "../../components/ui/popover";
import {
  dayKey,
  daysInMonth,
  monthLabel,
  monthWeeks,
  shiftMonth,
} from "../../lib/civil-day";
import { clockTime, formatClock, prefersHour12 } from "../../lib/format";
import { placeOnWeek } from "../dashboard/week-grid";
import type { WeekHours } from "../dashboard/week-hours";
import { phaseSpans, zonedTime, type DayPhase } from "../../lib/zoned-time";
import { DateEditor, dateLabel } from "./date-editor";
import { hoursOn, ruleSpan, type Span } from "./hours-math";

const WEEKDAY_HEADS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
/** Dates can be set this many months ahead, as far as the API takes them. */
const MONTHS_AHEAD = 12;
const DAYS_IN_WEEK = 7;
/** Solid fills: a mini tape is too thin for the washes the week uses. */
const PHASE_FILL: Record<DayPhase, string> = {
  Morning: "bg-dawn",
  Afternoon: "bg-noon",
  Evening: "bg-dusk/70",
};

const cellId = (date: string) => `date-cell-${date}`;

/** A date's hours as one thin strip on the week's ruler. */
function MiniTape({ spans, week }: { spans: Span[]; week: WeekHours }) {
  return (
    <span className="relative block h-1.5 overflow-hidden rounded-full bg-foreground/8">
      {spans.flatMap((span) =>
        phaseSpans(span.from, span.to).map((part) => (
          <span
            key={`${span.from}-${part.phase}`}
            style={placeOnWeek(week, part.from, part.to)}
            className={cn("absolute inset-y-0", PHASE_FILL[part.phase])}
          />
        )),
      )}
    </span>
  );
}

/**
 * The coming months, each date drawn with the hours it will really have.
 * A date with its own hours is outlined; picking any date changes it.
 */
export function DatesCalendar({
  rules,
  overrides,
  timezone,
  week,
}: {
  rules: AvailabilityRule[];
  overrides: DateOverride[];
  timezone: string;
  week: WeekHours;
}) {
  const hour12 = prefersHour12();
  const now = zonedTime(new Date(), timezone);
  const today = dayKey(now);
  const [shown, setShown] = useState({ year: now.year, month: now.month });
  const [open, setOpen] = useState<string | null>(null);
  const [focused, setFocused] = useState<string>(today);
  const monthsIn = (shown.year - now.year) * 12 + shown.month - now.month;
  const days = monthWeeks(shown.year, shown.month, { rows: "fit" }).flat();
  const inMonth = (date: string) =>
    date.startsWith(dayKey({ ...shown, day: 1 }).slice(0, 8));
  // The roving tab stop: the focused date when it is in this month, else its first open date.
  const firstDay = dayKey({ ...shown, day: monthsIn === 0 ? now.day : 1 });
  const tabStop = inMonth(focused) && focused >= today ? focused : firstDay;
  const upcoming = overrides.filter((day) => day.date >= today);

  const goTo = (date: string) => {
    const [year = 0, month = 1] = date.split("-").map(Number);
    setShown({ year, month });
    setFocused(date);
  };

  const onKey = (event: KeyboardEvent, day: number) => {
    const step = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -DAYS_IN_WEEK,
      ArrowDown: DAYS_IN_WEEK,
    }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const last = daysInMonth(shown.year, shown.month);
    const next = dayKey({ ...shown, day: Math.min(last, Math.max(1, day + step)) });
    if (next < today) return;
    setFocused(next);
    document.getElementById(cellId(next))?.focus();
  };

  const summary = (spans: Span[]) =>
    spans
      .map(
        (span) =>
          `${formatClock(clockTime(span.from), hour12)} to ${formatClock(clockTime(span.to), hour12)}`,
      )
      .join(", ");

  return (
    <div className="grid gap-4 p-5">
      <div className="flex items-center justify-between gap-3">
        <p
          className="font-display text-lg font-semibold tracking-tight"
          aria-live="polite"
        >
          {monthLabel(shown.year, shown.month)}
        </p>
        <div className="flex items-center gap-1">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="Previous month"
            disabled={monthsIn <= 0}
            onClick={() => setShown(shiftMonth(shown.year, shown.month, -1))}
          >
            <ChevronLeft />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="Next month"
            disabled={monthsIn >= MONTHS_AHEAD}
            onClick={() => setShown(shiftMonth(shown.year, shown.month, 1))}
          >
            <ChevronRight />
          </Button>
        </div>
      </div>

      <div>
        <div className="grid grid-cols-7 gap-1.5" aria-hidden="true">
          {WEEKDAY_HEADS.map((head) => (
            <span key={head} className="pb-1 text-xs font-medium text-muted-foreground">
              {head}
            </span>
          ))}
        </div>
        <div
          key={`${shown.year}-${shown.month}`}
          className="grid grid-cols-7 gap-1.5 motion-safe:animate-step-in"
        >
          {days.map((cell, index) => {
            if (!cell.inMonth) return <span key={`blank-${index}`} />;
            const { day } = cell;
            const date = dayKey(cell);
            const past = date < today;
            const { spans, own } = hoursOn(date, rules, overrides);
            const off = spans.length === 0;
            const state = off ? "off" : summary(spans);
            return (
              <Popover
                key={date}
                open={open === date}
                onOpenChange={(isOpen) => setOpen(isOpen ? date : null)}
              >
                <PopoverTrigger asChild>
                  <button
                    id={cellId(date)}
                    type="button"
                    disabled={past}
                    tabIndex={date === tabStop ? 0 : -1}
                    aria-current={date === today ? "date" : undefined}
                    aria-label={`${dateLabel(date)}, ${state}${own ? ", set for this date" : ""}`}
                    onFocus={() => setFocused(date)}
                    onKeyDown={(event) => onKey(event, day)}
                    className={cn(
                      "grid min-h-14 content-between gap-2 rounded-md border border-transparent p-1.5 text-left transition-colors outline-none sm:min-h-16 sm:p-2",
                      "hover:border-foreground/20 focus-visible:ring-[3px] focus-visible:ring-ring/50",
                      past ? "opacity-40" : "bg-muted/60",
                      own && !past && "border-dusk bg-dusk/5",
                      open === date && "border-dusk ring-2 ring-dusk/30",
                    )}
                  >
                    <span className="flex items-baseline justify-between gap-1">
                      <span
                        className={cn(
                          "text-clock text-sm tabular-nums",
                          date === today
                            ? "font-bold underline decoration-2 underline-offset-4"
                            : "font-medium",
                          off && !own && "text-muted-foreground",
                        )}
                      >
                        {day}
                      </span>
                      {own && off ? (
                        <span className="hidden text-xs font-medium text-dusk sm:inline">
                          Off
                        </span>
                      ) : null}
                    </span>
                    <MiniTape spans={spans} week={week} />
                  </button>
                </PopoverTrigger>
                <PopoverContent align="start" className="w-[min(26rem,calc(100vw-2rem))]">
                  <DateEditor
                    date={date}
                    rules={rules}
                    overrides={overrides}
                    onDone={() => {
                      setOpen(null);
                      document.getElementById(cellId(date))?.focus();
                    }}
                  />
                </PopoverContent>
              </Popover>
            );
          })}
        </div>
      </div>

      {upcoming.length > 0 ? (
        <div className="grid gap-2 border-t pt-4">
          <h3 className="text-sm font-semibold">Dates set apart</h3>
          <ul className="flex flex-wrap gap-2">
            {upcoming.map((day) => (
              <li key={day.date}>
                <button
                  type="button"
                  onClick={() => {
                    goTo(day.date);
                    setOpen(day.date);
                  }}
                  className="rounded-md border border-dusk/40 bg-card px-2.5 py-1 text-sm transition-colors outline-none hover:border-dusk focus-visible:ring-[3px] focus-visible:ring-ring/50"
                >
                  <span className="font-medium">{dateLabel(day.date)}</span>{" "}
                  <span className="text-clock text-muted-foreground">
                    {day.ranges.length === 0 ? "off" : summary(day.ranges.map(ruleSpan))}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
