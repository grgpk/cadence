import { cn } from "cn";
import { useEffect, useRef, useState } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { PHASE_STYLES } from "./slot-step";
import { clockLabel, dayPhase, zonedTime } from "../../lib/zoned-time";

/** Long enough to see the time fill before the card moves on, as in the month view. */
const PICK_MS = 170;

/** The one time being picked, reported once its fill has played. */
export function usePick(onPick: (slot: TimeSlot) => void) {
  const [picked, setPicked] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  // A view that is left before the fill finishes must not report its pick.
  useEffect(() => () => window.clearTimeout(timer.current), []);

  function pick(slot: TimeSlot) {
    if (picked !== null) return;
    setPicked(slot.start);
    timer.current = window.setTimeout(() => onPick(slot), PICK_MS);
  }

  return { picked, pick };
}

/** One open time, with the rail of its part of the day. */
export function TimeChip({
  slot,
  timeZone,
  hour12,
  picked,
  onPick,
  dayLabel,
  className,
}: {
  slot: TimeSlot;
  timeZone: string;
  hour12: boolean;
  picked: boolean;
  onPick: (slot: TimeSlot) => void;
  /** Read out with the time where the day is not in a heading nearby. */
  dayLabel?: string;
  className?: string;
}) {
  const start = new Date(slot.start);
  const styles = PHASE_STYLES[dayPhase(zonedTime(start, timeZone).hour)];
  const time = clockLabel(start, timeZone, hour12);
  return (
    <Button
      type="button"
      variant="outline"
      className={cn(
        "group relative h-10 w-full overflow-hidden px-2 text-clock text-[0.9375rem] hover:border-foreground hover:bg-card",
        picked && styles.picked,
        className,
      )}
      aria-label={dayLabel ? `${dayLabel}, ${time}` : undefined}
      onClick={() => onPick(slot)}
    >
      <span
        aria-hidden="true"
        className={cn(
          "absolute inset-y-0 left-0 transition-[width] duration-150 ease-out motion-reduce:transition-none",
          styles.rail,
          picked ? "w-full" : "w-[3px] group-hover:w-1.5",
        )}
      />
      <span className="relative">{time}</span>
    </Button>
  );
}
