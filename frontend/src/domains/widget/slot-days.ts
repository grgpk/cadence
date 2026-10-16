import type { TimeSlot } from "../../bindings/TimeSlot";
import { addDays, type CivilDay, dayKey, dayOfInstant } from "../../lib/civil-day";
import { zonedTime } from "../../lib/zoned-time";

/** The ways a visitor can look through open times. Month is the original card. */
export const SLOT_LAYOUTS = ["month", "week", "list"] as const;
export type SlotLayout = (typeof SLOT_LAYOUTS)[number];
export const DEFAULT_SLOT_LAYOUT: SlotLayout = "month";

export function isSlotLayout(value: unknown): value is SlotLayout {
  return SLOT_LAYOUTS.some((layout) => layout === value);
}

/** The open times of one day, in the order they happen. */
export type SlotDay = CivilDay & { key: string; slots: TimeSlot[] };

const WEEK_DAYS = 7;

/** Open times grouped by the day they fall on in `timeZone`, earliest first. */
export function slotsByDay(slots: TimeSlot[], timeZone: string): SlotDay[] {
  const days = new Map<string, SlotDay>();
  const earliestFirst = [...slots].sort(
    (a, b) => Date.parse(a.start) - Date.parse(b.start),
  );
  // In time order, each day is first seen before any later one.
  for (const slot of earliestFirst) {
    const { year, month, day } = dayOfInstant(slot.start, timeZone);
    const key = dayKey({ year, month, day });
    const existing = days.get(key);
    if (existing) existing.slots.push(slot);
    else days.set(key, { key, year, month, day, slots: [slot] });
  }
  return [...days.values()];
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
