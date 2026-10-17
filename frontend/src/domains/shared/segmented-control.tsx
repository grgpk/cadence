import { cn } from "cn";
import type { LucideIcon } from "lucide-react";

/** The track every segmented control sits in. */
export const SEGMENT_TRACK = "flex shrink-0 rounded-md bg-muted p-0.5";
/** One choice. Add `SEGMENT_ON` to the chosen one. */
export const SEGMENT =
  "flex items-center gap-1.5 rounded-sm font-medium text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50";
export const SEGMENT_ON = "bg-card text-foreground shadow-xs";

const SIZES = { sm: "px-2 py-0.5 text-xs", md: "px-2.5 py-1 text-sm" };

/** A few choices side by side, one of them on. Changes a view, never a page. */
export function SegmentedControl<T extends string | boolean>({
  label,
  value,
  options,
  onChange,
  size = "md",
  className,
}: {
  /** What is being chosen, for someone who cannot see the group. */
  label: string;
  value: T;
  options: readonly { value: T; label: string; icon?: LucideIcon }[];
  onChange: (value: T) => void;
  size?: keyof typeof SIZES;
  className?: string;
}) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a fieldset would need a visible legend
    <div role="group" aria-label={label} className={cn(SEGMENT_TRACK, className)}>
      {options.map(({ value: option, label: text, icon: Icon }) => (
        <button
          key={String(option)}
          type="button"
          aria-pressed={value === option}
          onClick={() => onChange(option)}
          className={cn(SEGMENT, SIZES[size], value === option && SEGMENT_ON)}
        >
          {/* Shown on the chosen option only; the others keep its space so nothing shifts. */}
          {Icon ? (
            <Icon className={cn("size-4", value !== option && "invisible")} />
          ) : null}
          {text}
        </button>
      ))}
    </div>
  );
}
