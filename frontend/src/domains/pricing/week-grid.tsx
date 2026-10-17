import { cn } from "cn";

import { useClockPreference } from "../../hooks/use-clock-preference";
import { hourLabel } from "../../lib/format";
import { dayPhase } from "../../lib/zoned-time";
import { OPEN_STYLES } from "../landing/day-tape";

const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri"] as const;
/** A working day with lunch kept free: six slots, thirty a week. */
const HOURS = [9, 10, 11, 14, 15, 16] as const;
/**
 * The order slots get booked in, as `day * HOURS.length + hour`. Fixed, so
 * one more call adds one block and the others stay put; spread so the week
 * fills evenly rather than day by day.
 */
const ORDER = [
  0, 13, 26, 9, 22, 4, 17, 6, 25, 14, 2, 21, 10, 29, 18, 7, 24, 1, 15, 28, 11, 20, 3, 16,
  27, 8, 23, 12, 5, 19,
];

/** The visitor's week as a grid, booked as they move the slider. Illustrative only. */
export function WeekGrid({ calls }: { calls: number }) {
  const [hour12] = useClockPreference();
  const booked = new Set(ORDER.slice(0, calls));
  return (
    <div
      aria-hidden
      className="grid grid-cols-[auto_repeat(5,minmax(0,1fr))] gap-x-1.5 gap-y-1.5 sm:gap-x-2"
    >
      <span />
      {DAYS.map((day) => (
        <span key={day} className="pb-0.5 text-xs font-medium text-muted-foreground">
          {day}
        </span>
      ))}
      {HOURS.map((hour, row) => (
        <div key={hour} className="contents">
          <span className="text-clock self-center pr-2 text-xs text-muted-foreground">
            {hourLabel(hour, hour12)}
          </span>
          {DAYS.map((day, column) => (
            <span
              key={day}
              className={cn(
                "relative h-6 rounded-sm border sm:h-7",
                OPEN_STYLES[dayPhase(hour)],
              )}
            >
              {booked.has(column * HOURS.length + row) ? (
                <span className="absolute -inset-px rounded-sm bg-primary motion-safe:animate-block-drop motion-safe:[animation-delay:0s]" />
              ) : null}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
}
