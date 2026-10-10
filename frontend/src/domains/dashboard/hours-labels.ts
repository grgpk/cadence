import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { weekdayName } from "../../lib/format";

const WEEK_DAYS = 7;

const shortDay = (weekday: number) => weekdayName(weekday).slice(0, 3);

/** "Mon to Fri, Sun" for the weekdays that have hours. */
export function daysLabel(rules: AvailabilityRule[]): string {
  const days = [...new Set(rules.map((rule) => rule.weekday))].sort((a, b) => a - b);
  if (days.length === WEEK_DAYS) return "Every day";
  const runs: number[][] = [];
  for (const day of days) {
    const run = runs.at(-1);
    if (run && run.at(-1) === day - 1) run.push(day);
    else runs.push([day]);
  }
  return runs
    .map((run) => {
      const [first = 0] = run;
      const last = run.at(-1) ?? first;
      if (first === last) return shortDay(first);
      return `${shortDay(first)}${run.length === 2 ? ", " : " to "}${shortDay(last)}`;
    })
    .join(", ");
}

/** "30 min calls", or "20 and 30 min calls" when the days differ. */
export function callLengthLabel(rules: AvailabilityRule[]): string {
  const lengths = [...new Set(rules.map((rule) => rule.slot_minutes))].sort(
    (a, b) => a - b,
  );
  return `${lengths.join(" and ")} min calls`;
}
