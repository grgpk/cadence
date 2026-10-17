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

/**
 * The visitor's week as a grid, booked as they move the slider. Calls past what
 * Free includes take the colour of the plan that fits. Illustrative only.
 */
export function WeekGrid({
  calls,
  freeCalls,
  freeIncluded,
}: {
  calls: number;
  /** Calls a week Free includes; `null` when it has no limit. */
  freeCalls: number | null;
  /** Bookings a month Free includes, for the key. */
  freeIncluded: number | null;
}) {
  const [hour12] = useClockPreference();
  // Each slot's place in the booking order, or nothing while it is open.
  const position = new Map(ORDER.slice(0, calls).map((slot, index) => [slot, index]));
  const pastFree = freeCalls !== null && calls > freeCalls;
  return (
    <div aria-hidden className="grid gap-3">
      <div className="grid grid-cols-[auto_repeat(5,minmax(0,1fr))] gap-x-1.5 gap-y-1.5 sm:gap-x-2">
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
                <BookedBlock
                  position={position.get(column * HOURS.length + row)}
                  freeCalls={freeCalls}
                />
              </span>
            ))}
          </div>
        ))}
      </div>
      {pastFree ? (
        <p className="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground motion-safe:animate-rise">
          <span className="flex items-center gap-1.5">
            <span className="size-2.5 rounded-[3px] bg-primary" />
            Within Free's {freeIncluded} a month
          </span>
          <span className="flex items-center gap-1.5">
            <span className="size-2.5 rounded-[3px] bg-dusk" />
            Past it, included on Pro and Business
          </span>
        </p>
      ) : null}
    </div>
  );
}

function BookedBlock({
  position,
  freeCalls,
}: {
  position: number | undefined;
  freeCalls: number | null;
}) {
  if (position === undefined) return null;
  const pastFree = freeCalls !== null && position >= freeCalls;
  return (
    <span
      className={cn(
        "absolute -inset-px rounded-sm motion-safe:animate-block-drop motion-safe:[animation-delay:0s]",
        pastFree ? "bg-dusk" : "bg-primary",
      )}
    />
  );
}
