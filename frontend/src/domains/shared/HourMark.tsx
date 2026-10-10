import { cn } from "cn";

import { hourLabel } from "../../lib/format";

const SIZES = {
  sm: { label: "h-5 text-sm", ticks: "h-2.5", half: "h-1.5" },
  lg: { label: "text-lg", ticks: "h-3", half: "h-1.5" },
};

/**
 * One hour on a ruler: its label, a tall tick on the hour and a short one on
 * the half. Every strip of hours in the app is a row of these.
 */
export function HourMark({
  hour,
  hour12,
  size = "sm",
  label = true,
  labelClassName,
}: {
  hour: number;
  /** Whether the viewer reads "2 PM" rather than "14:00". */
  hour12: boolean;
  size?: keyof typeof SIZES;
  /** Off where there is no room for every label; the ticks stay. */
  label?: boolean;
  labelClassName?: string;
}) {
  const styles = SIZES[size];
  return (
    <div>
      <p
        className={cn(
          "text-clock font-medium whitespace-nowrap text-muted-foreground",
          styles.label,
        )}
      >
        {label ? <span className={labelClassName}>{hourLabel(hour, hour12)}</span> : null}
      </p>
      <div className={cn("relative mt-1 border-t border-foreground/25", styles.ticks)}>
        <span
          className={cn("absolute top-0 left-0 w-px bg-foreground/40", styles.ticks)}
        />
        <span
          className={cn("absolute top-0 left-1/2 w-px bg-foreground/25", styles.half)}
        />
      </div>
    </div>
  );
}
