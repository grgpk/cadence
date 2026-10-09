import { cn } from "cn";

import type { WeekHours } from "./week-hours";

const HOUR = 60;
/** Past this many hours the ruler labels every other one. */
const DENSE_HOURS = 12;

/** Where a stretch of a day, in minutes since midnight, sits on the week's ruler. */
export function placeOnWeek(week: WeekHours, from: number, to: number) {
  const span = (week.last - week.first) * HOUR;
  return {
    left: `${((from - week.first * HOUR) / span) * 100}%`,
    width: `${((to - from) / span) * 100}%`,
  };
}

/**
 * The hours every day of a week is drawn against. `row` is the grid the day
 * rows under it use, so the ruler lines up with their tracks.
 */
export function WeekRuler({ week, row }: { week: WeekHours; row: string }) {
  const hours = Array.from(
    { length: week.last - week.first },
    (_, index) => week.first + index,
  );
  const step = hours.length > DENSE_HOURS ? 2 : 1;
  return (
    <div aria-hidden="true" className={cn(row, "border-b bg-muted/40 pt-3 pb-0")}>
      <span className="hidden sm:block" />
      <div
        className="grid"
        style={{ gridTemplateColumns: `repeat(${hours.length}, minmax(0, 1fr))` }}
      >
        {hours.map((hour, index) => (
          <div key={hour}>
            <p className="h-5 text-clock text-sm font-medium text-muted-foreground">
              {index % step === 0 ? (
                // A phone has room for every other label.
                <span
                  className={index % (step * 2) === 0 ? undefined : "hidden sm:inline"}
                >
                  {String(hour).padStart(2, "0")}:00
                </span>
              ) : null}
            </p>
            <div className="relative mt-1 h-2.5 border-t border-foreground/25">
              <span className="absolute top-0 left-0 h-2.5 w-px bg-foreground/40" />
              <span className="absolute top-0 left-1/2 h-1.5 w-px bg-foreground/25" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
