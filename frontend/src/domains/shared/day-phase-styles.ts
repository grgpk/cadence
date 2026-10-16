import type { DayPhase } from "../../lib/zoned-time";

/** Each part of the day has its own color, so a time reads as morning or evening at a glance. */
export const PHASE_STYLES: Record<DayPhase, { rail: string; picked: string }> = {
  Morning: { rail: "bg-dawn", picked: "text-day-foreground" },
  Afternoon: { rail: "bg-noon", picked: "text-day-foreground" },
  Evening: { rail: "bg-dusk", picked: "text-dusk-foreground" },
};
