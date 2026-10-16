import { addDays, type CivilDay, dayKey } from "../../lib/civil-day";
import type { SlotDay } from "./slot-days";

/** Where the month view is: a month, and the day picked in it if any. */
export type MonthPosition = { year: number; month: number; day: number | null };

const WEEK_DAYS = 7;

function monthIndex({ year, month }: { year: number; month: number }): number {
  return year * 12 + month;
}

/**
 * The month to open on: this one, or the month of the first open time when
 * that is later, so the calendar never opens on a month with nothing to pick.
 */
export function openingMonth(days: SlotDay[], today: CivilDay): MonthPosition {
  const first = days[0];
  const start = first && monthIndex(first) > monthIndex(today) ? first : today;
  return { year: start.year, month: start.month, day: null };
}

/** The days of the shown month that have open times. */
export function openDaysIn(days: SlotDay[], shown: MonthPosition): number[] {
  return days
    .filter((day) => day.year === shown.year && day.month === shown.month)
    .map((day) => day.day);
}

/** The picked day while it still has times, else the month's first that does. */
export function selectedDay(days: SlotDay[], shown: MonthPosition): number | null {
  const open = openDaysIn(days, shown);
  return shown.day !== null && open.includes(shown.day) ? shown.day : (open[0] ?? null);
}

/** Whether the calendar can move back or on: only to months with something to pick. */
export function monthSteps(
  days: SlotDay[],
  shown: MonthPosition,
): { earlier: boolean; later: boolean } {
  const here = monthIndex(shown);
  return {
    earlier: days.some((day) => monthIndex(day) < here),
    later: days.some((day) => monthIndex(day) > here),
  };
}

/**
 * The first open day on or after `day`, or the first one at all. A week starts
 * there rather than on a Monday, so it never opens on days already gone.
 */
export function firstOpenFrom(days: SlotDay[], day: CivilDay | null): SlotDay | null {
  const from = day ? dayKey(day) : "";
  return days.find((open) => open.key >= from) ?? days[0] ?? null;
}

/**
 * The start of the week that holds `day`, stepping a week at a time from the
 * first open day as the arrows do. A day picked on the month calendar then
 * lands inside the week instead of opening one that starts on it.
 */
export function weekAround(days: SlotDay[], day: CivilDay | null): SlotDay | null {
  let start = firstOpenFrom(days, null);
  if (!day) return start;
  const target = dayKey(day);
  while (start) {
    const weekEnd = dayKey(addDays(start, WEEK_DAYS));
    if (target < weekEnd) break;
    const next = days.find((open) => open.key >= weekEnd);
    if (!next) break;
    start = next;
  }
  return start;
}

/**
 * Where the week arrows go from the week starting on `start`: each move lands
 * on an open day, skipping stretches with nothing to pick. `null` when there
 * is nothing that way.
 */
export function weekSteps(
  days: SlotDay[],
  start: CivilDay,
): { earlier: SlotDay | null; later: SlotDay | null } {
  const from = dayKey(start);
  const to = dayKey(addDays(start, WEEK_DAYS));
  return {
    earlier: days.some((day) => day.key < from)
      ? firstOpenFrom(days, addDays(start, -WEEK_DAYS))
      : null,
    later: days.find((day) => day.key >= to) ?? null,
  };
}

/** Days the list shows at first, and adds each time. */
export const DAYS_PER_PAGE = 5;

/** The days the list shows, and whether there are more to show. */
export function listPage(
  days: SlotDay[],
  shown: number,
): { days: SlotDay[]; more: boolean } {
  return { days: days.slice(0, shown), more: days.length > shown };
}

/**
 * The time being picked: one at a time. While one fills, another click does
 * nothing; a pick that did not go through is released.
 */
export type Pick = string | null;

export function pickTime(current: Pick, start: string): Pick {
  return current ?? start;
}

export function releasePick(): Pick {
  return null;
}
