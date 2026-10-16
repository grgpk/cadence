import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { CSSProperties } from "react";

import { Button } from "../../components/ui/button";
import { longDayLabel, monthLabel, monthWeeks } from "../../lib/civil-day";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

export function BookingCalendar({
  year,
  month,
  direction,
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
  /** +1 when this month was reached by going forward, -1 by going back. */
  direction: number;
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
  const days = monthWeeks(year, month, { rows: "fit" }).flat();

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
      <div
        // Remounting on a month change replays the slide, from the side of travel.
        key={`${year}-${month}`}
        style={{ "--step-from": `${direction}rem` } as CSSProperties}
        className="grid grid-cols-7 gap-1 motion-safe:animate-step-in"
      >
        {days.map((cell, index) => {
          if (!cell.inMonth) return <span key={`blank-${index}`} />;
          const { day } = cell;
          const available = availableDays.includes(day);
          const selected = selectedDay === day;
          return (
            <button
              key={day}
              type="button"
              disabled={!available}
              aria-pressed={selected}
              aria-current={day === todayDay ? "date" : undefined}
              aria-label={`${longDayLabel(cell)}${available ? "" : ", unavailable"}`}
              onClick={() => onSelect(day)}
              className={cn(
                "relative flex aspect-square w-full items-center justify-center rounded-md text-sm tabular-nums transition-colors outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
                available
                  ? "bg-muted font-semibold hover:bg-accent hover:ring-1 hover:ring-foreground/20"
                  : "text-muted-foreground/60",
                selected && "bg-dusk text-dusk-foreground hover:bg-dusk hover:ring-0",
                day === todayDay && !selected && "ring-1 ring-foreground/25",
              )}
            >
              {day}
              {/* One dot under every day that has open times. */}
              {available ? (
                <span
                  className={cn(
                    "absolute bottom-1.5 size-1 rounded-full",
                    selected ? "bg-dusk-foreground" : "bg-noon",
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
