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

/** The part of the visitor's day an hour falls in. */
export function dayPhase(hour: number): DayPhase {
  if (hour < NOON) return "Morning";
  return hour < EVENING ? "Afternoon" : "Evening";
}
