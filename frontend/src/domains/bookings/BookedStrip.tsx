import { cn } from "cn";

import { PHASE_STYLES } from "../widget/SlotStep";
import { dayPhase, zonedTime } from "../widget/zoned-time";

/** The strip shows the hour before the call and the two after it starts. */
const STRIP_HOURS = 4;
const STRIP_MINUTES = STRIP_HOURS * 60;

/** The booked call as a block on its stretch of the day. */
export function BookedStrip({
  start,
  minutes,
  timeZone,
  label,
}: {
  start: Date;
  minutes: number;
  timeZone: string;
  /** The call's time range, printed under its block. */
  label: string;
}) {
  const at = zonedTime(start, timeZone);
  const firstHour = at.hour - 1;
  const styles = PHASE_STYLES[dayPhase(at.hour)];
  const left = ((60 + at.minute) / STRIP_MINUTES) * 100;
  const width = Math.min((minutes / STRIP_MINUTES) * 100, 100 - left);

  return (
    <div aria-hidden="true" className="rounded-xl border bg-card px-4 pt-3 pb-4">
      <div className="grid grid-cols-4">
        {Array.from({ length: STRIP_HOURS }, (_, index) => (
          <div key={index}>
            <p className="text-clock text-sm font-medium text-muted-foreground">
              {String((firstHour + index + 24) % 24).padStart(2, "0")}:00
            </p>
            <div className="relative mt-1 h-2.5 border-t border-foreground/25">
              <span className="absolute top-0 left-0 h-2.5 w-px bg-foreground/40" />
              <span className="absolute top-0 left-1/2 h-1.5 w-px bg-foreground/25" />
            </div>
          </div>
        ))}
      </div>
      <div className="relative mt-2 h-9">
        <span
          style={{ left: `${left}%`, width: `${width}%` }}
          className={cn(
            "vt-booked-block absolute inset-y-0 min-w-6 rounded-md motion-safe:animate-block-drop",
            styles.rail,
          )}
        />
      </div>
      {/* Follows the block, but never runs off the right edge. */}
      <p
        style={{ marginLeft: `min(${left}%, calc(100% - 9rem))` }}
        className="mt-1.5 text-clock text-sm font-medium"
      >
        {label}
      </p>
    </div>
  );
}
