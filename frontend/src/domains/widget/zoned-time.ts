import { formatClock } from "../../lib/format";

export type ZonedTime = {
  year: number;
  /** 1-indexed, 1 = January. */
  month: number;
  day: number;
  hour: number;
  minute: number;
};

/** Wall-clock reading of an instant in an IANA timezone. */
export function zonedTime(instant: Date, timeZone: string): ZonedTime {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(instant);
  const get = (type: Intl.DateTimeFormatPartTypes) =>
    Number(parts.find((part) => part.type === type)?.value);
  return {
    year: get("year"),
    month: get("month"),
    day: get("day"),
    hour: get("hour"),
    minute: get("minute"),
  };
}

/** "09:30" or "9:30 AM" for an instant, as the visitor's clock shows it. */
export function clockLabel(instant: Date, timeZone: string, hour12: boolean): string {
  const { hour, minute } = zonedTime(instant, timeZone);
  return formatClock(`${hour}:${minute}`, hour12);
}

/** When a call that starts at `start` is over. */
export function callEnd(start: string, minutes: number): Date {
  return new Date(new Date(start).getTime() + minutes * 60_000);
}

export const DAY_PHASES = ["Morning", "Afternoon", "Evening"] as const;
export type DayPhase = (typeof DAY_PHASES)[number];

const NOON = 12;
const EVENING = 17;

const HOUR_MINUTES = 60;
const PHASE_ENDS: { phase: DayPhase; end: number }[] = [
  { phase: "Morning", end: NOON * HOUR_MINUTES },
  { phase: "Afternoon", end: EVENING * HOUR_MINUTES },
  { phase: "Evening", end: 24 * HOUR_MINUTES },
];

/**
 * A stretch of one day, in minutes since midnight, cut where the part of the
 * day changes, so each piece can take its own color.
 */
export function phaseSpans(
  from: number,
  to: number,
): { phase: DayPhase; from: number; to: number }[] {
  const spans: { phase: DayPhase; from: number; to: number }[] = [];
  let start = 0;
  for (const { phase, end } of PHASE_ENDS) {
    if (from < end && to > start) {
      spans.push({ phase, from: Math.max(from, start), to: Math.min(to, end) });
    }
    start = end;
  }
  return spans;
}

/** The part of the visitor's day an hour falls in. */
export function dayPhase(hour: number): DayPhase {
  if (hour < NOON) return "Morning";
  return hour < EVENING ? "Afternoon" : "Evening";
}
