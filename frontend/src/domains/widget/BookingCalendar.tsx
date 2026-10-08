import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { Button } from "../../components/ui/button";
import { calendarWeeks, monthLabel } from "./calendar-grid";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

function dayLabel(year: number, month: number, day: number): string {
  return new Intl.DateTimeFormat("en", {
    weekday: "long",
    day: "numeric",
    month: "long",
    timeZone: "UTC",
  }).format(new Date(Date.UTC(year, month - 1, day)));
}

export function BookingCalendar({
  year,
  month,
  availableDays,
  selectedDay,
  todayDay,
  canGoPrev,
  canGoNext,
  onPrev,
  onNext,
  onSelect,
}: {
  year: number;
  /** 1-indexed, 1 = January. */
  month: number;
  availableDays: number[];
  selectedDay: number | null;
  /** Today's day number when the shown month is the current one. */
  todayDay: number | null;
  canGoPrev: boolean;
  canGoNext: boolean;
  onPrev: () => void;
  onNext: () => void;
  onSelect: (day: number) => void;
}) {
  const days = calendarWeeks(year, month, true).flat();

  return (
    <div>
      <div className="flex items-center justify-between">
        <p
          className="font-display text-lg font-semibold tracking-tight"
          aria-live="polite"
        >
          {monthLabel(year, month)}
        </p>
        <div className="flex items-center gap-1">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="Previous month"
            disabled={!canGoPrev}
            onClick={onPrev}
          >
            <ChevronLeft />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="Next month"
            disabled={!canGoNext}
            onClick={onNext}
          >
            <ChevronRight />
          </Button>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-7 gap-1" aria-hidden="true">
        {WEEKDAYS.map((weekday) => (
          <span
            key={weekday}
            className="pb-1 text-center text-xs font-medium text-muted-foreground"
          >
            {weekday}
          </span>
        ))}
      </div>
      <div className="grid grid-cols-7 gap-1">
        {days.map((day, index) => {
          if (day === null) return <span key={`blank-${index}`} />;
          const available = availableDays.includes(day);
          const selected = selectedDay === day;
          return (
            <button
              key={day}
              type="button"
              disabled={!available}
              aria-pressed={selected}
              aria-label={`${dayLabel(year, month, day)}${available ? "" : ", unavailable"}`}
              onClick={() => onSelect(day)}
              className={cn(
                "relative flex aspect-square w-full items-center justify-center rounded-md text-sm tabular-nums outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
                available
                  ? "cursor-pointer bg-muted font-semibold hover:bg-accent hover:ring-1 hover:ring-foreground/20"
                  : "text-muted-foreground/60",
                selected &&
                  "bg-signal text-signal-foreground hover:bg-signal hover:ring-0",
                day === todayDay && !selected && "ring-1 ring-foreground/25",
              )}
            >
              {day}
              {/* The beat: one dot under every day that can be booked. */}
              {available ? (
                <span
                  className={cn(
                    "absolute bottom-1.5 size-1 rounded-full",
                    selected ? "bg-signal-foreground" : "bg-signal",
                  )}
                />
              ) : null}
            </button>
          );
        })}
      </div>
    </div>
  );
}
