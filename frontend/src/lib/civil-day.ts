import { dateFormatter, formatDay, formatLongDay } from "./format";
import { zonedTime } from "./zoned-time";

/**
 * A date on someone's calendar, with no time or zone attached. Weeks start on
 * Monday. Arithmetic runs on UTC midnights, so a clock change never moves a day.
 */
export type CivilDay = { year: number; month: number; day: number };

/** A day in a month's grid; days from the months either side are not `inMonth`. */
export type MonthCell = CivilDay & { inMonth: boolean };

const DAY_MS = 86_400_000;
const WEEK_DAYS = 7;
/** Six full weeks: every month fits, whatever day it starts on. */
const SIX_WEEKS = 6;

function midnight({ year, month, day }: CivilDay): Date {
  return new Date(Date.UTC(year, month - 1, day));
}

function fromMidnight(date: Date): CivilDay {
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    day: date.getUTCDate(),
  };
}

/** "2026-10-05": sorts in date order, and is the API's form for a date. */
export function dayKey({ year, month, day }: CivilDay): string {
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

/** The day an ISO date names, such as a date override's `date`. */
export function fromKey(key: string): CivilDay {
  const [year = 0, month = 1, day = 1] = key.split("-").map(Number);
  return { year, month, day };
}

/** The day an instant falls on, on the clock of `timeZone`. */
export function dayOfInstant(
  instant: Date | number | string,
  timeZone: string,
): CivilDay {
  const { year, month, day } = zonedTime(new Date(instant), timeZone);
  return { year, month, day };
}

/** Today on the clock of `timeZone`. */
export function todayIn(timeZone: string, now: Date | number): CivilDay {
  return dayOfInstant(now, timeZone);
}

export function addDays(day: CivilDay, days: number): CivilDay {
  return fromMidnight(new Date(midnight(day).getTime() + days * DAY_MS));
}

/** Whole calendar days from `from` to `to`. */
export function daysBetween(from: CivilDay, to: CivilDay): number {
  return Math.round((midnight(to).getTime() - midnight(from).getTime()) / DAY_MS);
}

export function sameDay(a: CivilDay, b: CivilDay): boolean {
  return a.year === b.year && a.month === b.month && a.day === b.day;
}

/** Monday is 0, as in the API's weekly hours. */
export function weekdayIndex(day: CivilDay): number {
  return (midnight(day).getUTCDay() + 6) % WEEK_DAYS;
}

/** The Monday on or before `day`. */
export function mondayOf(day: CivilDay): CivilDay {
  return addDays(day, -weekdayIndex(day));
}

/** Seven days in a row, from `start`. */
export function weekFrom(start: CivilDay): CivilDay[] {
  return Array.from({ length: WEEK_DAYS }, (_, index) => addDays(start, index));
}

/** The month `offset` months from the given one. */
export function shiftMonth(
  year: number,
  month: number,
  offset: number,
): { year: number; month: number } {
  const { year: shiftedYear, month: shiftedMonth } = fromMidnight(
    new Date(Date.UTC(year, month - 1 + offset, 1)),
  );
  return { year: shiftedYear, month: shiftedMonth };
}

/** 28 to 31. */
export function daysInMonth(year: number, month: number): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

/**
 * A month as weeks, Monday first. `"fit"` gives only the weeks the month
 * touches (five or six); `"six"` always gives six, so a grid keeps its height.
 */
export function monthWeeks(
  year: number,
  month: number,
  { rows }: { rows: "fit" | "six" },
): MonthCell[][] {
  const first: CivilDay = { year, month, day: 1 };
  const start = mondayOf(first);
  const last: CivilDay = { year, month, day: daysInMonth(year, month) };
  const weeks =
    rows === "six" ? SIX_WEEKS : Math.ceil((daysBetween(start, last) + 1) / WEEK_DAYS);
  return Array.from({ length: weeks }, (_, week) =>
    weekFrom(addDays(start, week * WEEK_DAYS)).map((day) => ({
      ...day,
      inMonth: day.month === month,
    })),
  );
}

const MONTH_SHORT = { month: "short", timeZone: "UTC" } as const;

/** "October 2026" */
export function monthLabel(year: number, month: number): string {
  return dateFormatter("en", { month: "long", year: "numeric", timeZone: "UTC" }).format(
    midnight({ year, month, day: 1 }),
  );
}

/** "Oct 19 to 25", or "Oct 26 to Nov 1" when the week crosses a month. */
export function weekLabel(start: CivilDay): string {
  const end = addDays(start, WEEK_DAYS - 1);
  const month = (day: CivilDay) => dateFormatter("en", MONTH_SHORT).format(midnight(day));
  return start.month === end.month
    ? `${month(start)} ${start.day} to ${end.day}`
    : `${month(start)} ${start.day} to ${month(end)} ${end.day}`;
}

/** "Wed, Oct 14" */
export function shortDayLabel(day: CivilDay): string {
  return formatDay(midnight(day).toISOString(), "UTC");
}

/** "Wednesday, October 14" */
export function longDayLabel(day: CivilDay): string {
  return formatLongDay(midnight(day), "UTC", false);
}

/** "Mon" */
export function shortWeekday(day: CivilDay): string {
  return dateFormatter("en", { weekday: "short", timeZone: "UTC" }).format(midnight(day));
}

/** "Today", "Tomorrow", or "Wednesday, October 14". */
export function relativeDayLabel(day: CivilDay, today: CivilDay): string {
  if (sameDay(day, today)) return "Today";
  if (sameDay(day, addDays(today, 1))) return "Tomorrow";
  return longDayLabel(day);
}
