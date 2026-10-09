import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { clockMinutes } from "../../lib/format";

const HOUR = 60;
const DAY_HOURS = 24;
/** Shown when no hours are set yet: an ordinary working day. */
const EMPTY_WEEK = { first: 8, last: 18 };
/** Fewer hours than this and a short range would fill the whole ruler. */
const MIN_HOURS = 6;

export type WeekHours = { first: number; last: number };

/**
 * The hours a week's ruler has to show: from the earliest start to the latest
 * end on any day, in whole hours.
 */
export function weekHours(rules: AvailabilityRule[]): WeekHours {
  if (rules.length === 0) return EMPTY_WEEK;
  let first = Math.floor(
    Math.min(...rules.map((rule) => clockMinutes(rule.start_time))) / HOUR,
  );
  let last = Math.ceil(
    Math.max(...rules.map((rule) => clockMinutes(rule.end_time))) / HOUR,
  );
  if (last - first < MIN_HOURS) {
    last = Math.min(DAY_HOURS, first + MIN_HOURS);
    first = last - MIN_HOURS;
  }
  return { first, last };
}
