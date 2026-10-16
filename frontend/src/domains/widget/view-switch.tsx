import { cn } from "cn";
import { CalendarDays, Columns3, List, type LucideIcon } from "lucide-react";
import type { CSSProperties } from "react";

import { SLOT_LAYOUTS, type SlotLayout } from "./slot-days";

const OPTIONS: Record<SlotLayout, { label: string; icon: LucideIcon }> = {
  month: { label: "Month", icon: CalendarDays },
  week: { label: "Week", icon: Columns3 },
  list: { label: "List", icon: List },
};

/**
 * How the open times are laid out. Three equal parts that fill the column,
 * with one highlight that slides to the chosen one.
 */
export function ViewSwitch({
  value,
  onChange,
}: {
  value: SlotLayout;
  onChange: (layout: SlotLayout) => void;
}) {
  const index = SLOT_LAYOUTS.indexOf(value);
  return (
    // biome-ignore lint/a11y/useSemanticElements: a fieldset would need a visible legend
    <div
      role="group"
      aria-label="Show times as"
      style={{ "--view-index": index } as CSSProperties}
      className="relative grid grid-cols-3 rounded-lg bg-muted p-1"
    >
      <span
        aria-hidden="true"
        className="absolute inset-y-1 left-1 w-[calc((100%-0.5rem)/3)] translate-x-[calc(var(--view-index)*100%)] rounded-md bg-card shadow-xs ring-1 ring-foreground/10 transition-transform duration-200 ease-out motion-reduce:transition-none"
      />
      {SLOT_LAYOUTS.map((layout) => {
        const { label, icon: Icon } = OPTIONS[layout];
        const on = layout === value;
        return (
          <button
            key={layout}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(layout)}
            className={cn(
              "relative flex h-8 min-w-0 items-center justify-center gap-1 rounded-md text-xs font-medium transition-colors outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
              on ? "text-foreground" : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon className="size-3.5 shrink-0" />
            {label}
          </button>
        );
      })}
    </div>
  );
}
