import { cn } from "cn";

import { prefersHour12 } from "../../lib/format";
import { HourMark } from "../shared/hour-mark";
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
  const hour12 = prefersHour12();
  // "10 AM" is narrower than "10:00", but a long day still needs every other label.
  const step = hours.length > DENSE_HOURS ? 2 : 1;
  return (
    <div
      aria-hidden="true"
      className={cn(row, "@container border-b bg-muted/40 pt-3 pb-0")}
    >
      {/* Lines up with the day names once WeekCanvas's `week` container is wide. */}
      <span className="hidden @3xl/week:block" />
      <div
        className="grid"
        style={{ gridTemplateColumns: `repeat(${hours.length}, minmax(0, 1fr))` }}
      >
        {hours.map((hour, index) => (
          <HourMark
            key={hour}
            hour={hour}
            hour12={hour12}
            label={index % step === 0}
            // A narrow card has room for every other label that is left.
            labelClassName={index % (step * 2) === 0 ? undefined : "hidden @2xl:inline"}
          />
        ))}
      </div>
    </div>
  );
}
