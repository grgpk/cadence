import type { TimeSlot } from "../../bindings/TimeSlot";
import { formatLongDay } from "../../lib/format";
import { zonedTime } from "./zoned-time";

/** The ways a visitor can look through open times. Month is the original card. */
export const SLOT_LAYOUTS = ["month", "week", "list"] as const;
export type SlotLayout = (typeof SLOT_LAYOUTS)[number];
export const DEFAULT_SLOT_LAYOUT: SlotLayout = "month";

export function isSlotLayout(value: unknown): value is SlotLayout {
  return SLOT_LAYOUTS.some((layout) => layout === value);
}

/** A date on the visitor's calendar, with no time or zone attached. */
export type CivilDay = { year: number; month: number; day: number };

/** The open times of one day, in the order they happen. */
export type SlotDay = CivilDay & { key: string; slots: TimeSlot[] };

const DAY_MS = 86_400_000;
const WEEK_DAYS = 7;

/** "2026-10-17", which also sorts in date order. */
export function dayKey({ year, month, day }: CivilDay): string {
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function utcDate({ year, month, day }: CivilDay): Date {
  return new Date(Date.UTC(year, month - 1, day));
}

function fromUtc(date: Date): CivilDay {
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    day: date.getUTCDate(),
  };
}

export function addDays(day: CivilDay, days: number): CivilDay {
  return fromUtc(new Date(utcDate(day).getTime() + days * DAY_MS));
}

/** Open times grouped by the day they fall on in `timeZone`, earliest first. */
export function slotsByDay(slots: TimeSlot[], timeZone: string): SlotDay[] {
  const days = new Map<string, SlotDay>();
  const earliestFirst = [...slots].sort(
    (a, b) => Date.parse(a.start) - Date.parse(b.start),
  );
  // In time order, each day is first seen before any later one.
  for (const slot of earliestFirst) {
    const { year, month, day } = zonedTime(new Date(slot.start), timeZone);
    const key = dayKey({ year, month, day });
    const existing = days.get(key);
    if (existing) existing.slots.push(slot);
    else days.set(key, { key, year, month, day, slots: [slot] });
  }
  return [...days.values()];
}

/** The seven days in a row from `start`. */
export function weekDays(start: CivilDay): CivilDay[] {
  return Array.from({ length: WEEK_DAYS }, (_, index) => addDays(start, index));
}

/** "Oct 19 – 25", or "Oct 26 – Nov 1" when the week crosses a month. */
export function weekLabel(start: CivilDay): string {
  const end = addDays(start, WEEK_DAYS - 1);
  const month = (day: CivilDay) =>
    utcDate(day).toLocaleString("en", { month: "short", timeZone: "UTC" });
  return start.month === end.month
    ? `${month(start)} ${start.day} – ${end.day}`
    : `${month(start)} ${start.day} – ${month(end)} ${end.day}`;
}

/** "Today", "Tomorrow", or "Monday, October 19". */
export function relativeDayLabel(day: CivilDay, today: CivilDay): string {
  const key = dayKey(day);
  if (key === dayKey(today)) return "Today";
  if (key === dayKey(addDays(today, 1))) return "Tomorrow";
  return formatLongDay(utcDate(day), "UTC", false);
}

/** "Mon" for a day, read on the calendar rather than in any zone. */
export function shortWeekday(day: CivilDay): string {
  return utcDate(day).toLocaleString("en", { weekday: "short", timeZone: "UTC" });
}

/** "Monday, October 19", for a screen reader naming a column of times. */
export function longDayLabel(day: CivilDay): string {
  return formatLongDay(utcDate(day), "UTC", false);
}

/** The call length when every open time has the same one; hosts can vary it by day. */
export function sharedLength(slots: TimeSlot[]): number | null {
  const lengths = new Set(slots.map((slot) => slot.minutes));
  return lengths.size === 1 ? (slots[0]?.minutes ?? null) : null;
}

/** One time of day across a week: the open slot on each day, or `null`. */
type TimetableRow = {
  /** Minutes since the visitor's midnight. */
  minutes: number;
  cells: (TimeSlot | null)[];
};

/**
 * A week's open times as rows, one per time of day, so the same time lines up
 * across days. A wall time that happens twice on one day, when clocks go
 * back, keeps the first slot.
 */
export function weekTimetable(
  days: SlotDay[],
  week: CivilDay[],
  timeZone: string,
): TimetableRow[] {
  const byKey = new Map(days.map((day) => [day.key, day]));
  const rows = new Map<number, TimetableRow>();
  week.forEach((day, column) => {
    for (const slot of byKey.get(dayKey(day))?.slots ?? []) {
      const { hour, minute } = zonedTime(new Date(slot.start), timeZone);
      const minutes = hour * 60 + minute;
      let row = rows.get(minutes);
      if (!row) {
        row = { minutes, cells: week.map(() => null) };
        rows.set(minutes, row);
      }
      row.cells[column] ??= slot;
    }
  });
  return [...rows.values()].sort((a, b) => a.minutes - b.minutes);
}
