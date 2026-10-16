import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { DateOverride } from "../../bindings/DateOverride";
import { fromKey, weekdayIndex } from "../../lib/civil-day";
import { clockMinutes } from "../../lib/format";
import type { WeekHours } from "../dashboard/week-hours";

const HOUR = 60;
const DAY_MINUTES = 24 * HOUR;
/** Hours drawn or dragged land on the quarter hour, as the time picker does. */
export const SNAP_MINUTES = 15;
/** A call length for hours drawn on a day that has none to copy. */
export const DEFAULT_SLOT_MINUTES = 30;

/** A stretch of a day in minutes since midnight, and the length of its calls. */
export type Span = { from: number; to: number; slotMinutes: number };

export function ruleSpan(rule: {
  start_time: string;
  end_time: string;
  slot_minutes: number;
}): Span {
  return {
    from: clockMinutes(rule.start_time),
    to: clockMinutes(rule.end_time),
    slotMinutes: rule.slot_minutes,
  };
}

/** The nearest quarter hour, kept inside the day. */
export function snap(minutes: number): number {
  const snapped = Math.round(minutes / SNAP_MINUTES) * SNAP_MINUTES;
  return Math.min(DAY_MINUTES, Math.max(0, snapped));
}

/** The time under a pointer at `x` on a track drawn from `left` and `width` pixels. */
export function minutesAt(
  x: number,
  left: number,
  width: number,
  week: WeekHours,
): number {
  const share = Math.min(1, Math.max(0, (x - left) / width));
  return snap(week.first * HOUR + share * (week.last - week.first) * HOUR);
}

/** How many calls fit in a span. */
export function callsIn({ from, to, slotMinutes }: Span): number {
  return Math.max(0, Math.floor((to - from) / slotMinutes));
}

/** Open minutes and calls across the week. */
export function weekTotals(rules: AvailabilityRule[]): {
  minutes: number;
  calls: number;
} {
  return rules.map(ruleSpan).reduce(
    (total, span) => ({
      minutes: total.minutes + span.to - span.from,
      calls: total.calls + callsIn(span),
    }),
    { minutes: 0, calls: 0 },
  );
}

/** The week ruler with an hour of room on each side, so a range can grow past it. */
export function roomyWeek(week: WeekHours): WeekHours {
  return { first: Math.max(0, week.first - 1), last: Math.min(24, week.last + 1) };
}

/**
 * What saving `from` to `to` on a day does to its other ranges. Ranges it
 * touches or overlaps are folded into one, so a day never holds two ranges
 * over the same time. `self` is the range being moved or stretched.
 */
export function foldInto(
  dayRules: AvailabilityRule[],
  from: number,
  to: number,
  self?: AvailabilityRule,
): {
  keep: AvailabilityRule | undefined;
  from: number;
  to: number;
  drop: AvailabilityRule[];
} {
  const touched = dayRules.filter((rule) => {
    if (rule.id === self?.id) return false;
    const span = ruleSpan(rule);
    return span.from <= to && span.to >= from;
  });
  const keep = self ?? touched[0];
  const drop = touched.filter((rule) => rule.id !== keep?.id);
  const spans = touched.map(ruleSpan);
  return {
    keep,
    from: Math.min(from, ...spans.map((span) => span.from)),
    to: Math.max(to, ...spans.map((span) => span.to)),
    drop,
  };
}

/**
 * The hours a date is open: its own when it has an override, otherwise its
 * weekday's.
 */
export function hoursOn(
  date: string,
  rules: AvailabilityRule[],
  overrides: DateOverride[],
): { spans: Span[]; own: boolean } {
  const override = overrides.find((day) => day.date === date);
  if (override) return { spans: override.ranges.map(ruleSpan), own: true };
  const weekday = weekdayIndex(fromKey(date));
  return {
    spans: rules.filter((rule) => rule.weekday === weekday).map(ruleSpan),
    own: false,
  };
}

/** "40 hours", "7.5 hours", "1 hour". */
export function hoursLabel(minutes: number): string {
  const hours = Math.round((minutes / HOUR) * 10) / 10;
  return `${hours} ${hours === 1 ? "hour" : "hours"}`;
}

export type DragKind = "draw" | "move" | "start" | "end";
/** The earliest and latest minute a drag may reach. */
export type Bounds = { min: number; max: number };

/**
 * Where a span sits while it is dragged. `base` is the span when the press
 * began and `origin` the time under the pointer then; `at` is the time under
 * it now. Working from the press, not the last move, keeps the range under the
 * pointer however many moves arrive.
 */
export function dragSpan(
  kind: DragKind,
  base: Span,
  origin: number,
  at: number,
  bounds: Bounds,
): Span {
  const point = Math.min(bounds.max, Math.max(bounds.min, at));
  switch (kind) {
    case "draw": {
      const from = Math.min(origin, point);
      const to = Math.max(origin, point);
      if (to - from >= SNAP_MINUTES) return { ...base, from, to };
      return from + SNAP_MINUTES <= bounds.max
        ? { ...base, from, to: from + SNAP_MINUTES }
        : { ...base, from: to - SNAP_MINUTES, to };
    }
    case "start":
      return { ...base, from: Math.min(point, base.to - SNAP_MINUTES) };
    case "end":
      return { ...base, to: Math.max(point, base.from + SNAP_MINUTES) };
    case "move": {
      const length = base.to - base.from;
      const from = Math.min(
        bounds.max - length,
        Math.max(bounds.min, base.from + at - origin),
      );
      return { ...base, from, to: from + length };
    }
  }
}
