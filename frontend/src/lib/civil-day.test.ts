import { describe, expect, it } from "vitest";

import {
  addDays,
  daysBetween,
  daysInMonth,
  dayKey,
  fromKey,
  longDayLabel,
  mondayOf,
  monthLabel,
  monthWeeks,
  relativeDayLabel,
  sameDay,
  shiftMonth,
  shortDayLabel,
  shortWeekday,
  todayIn,
  weekdayIndex,
  weekFrom,
  weekLabel,
} from "./civil-day";

const day = (year: number, month: number, date: number) => ({ year, month, day: date });

describe("keys", () => {
  it("writes the ISO date, padded so keys sort in date order", () => {
    expect(dayKey(day(2026, 10, 5))).toBe("2026-10-05");
    expect(dayKey(day(2026, 9, 30)) < dayKey(day(2026, 10, 1))).toBe(true);
  });

  it("reads a key back into the same day", () => {
    expect(fromKey("2026-10-05")).toEqual(day(2026, 10, 5));
    expect(fromKey(dayKey(day(2027, 1, 31)))).toEqual(day(2027, 1, 31));
  });
});

describe("today", () => {
  it("is the day on the clock of the given timezone", () => {
    // Sunday 23:30 UTC is already Monday in Tokyo.
    const sundayNight = Date.parse("2026-10-18T23:30:00Z");
    expect(todayIn("UTC", sundayNight)).toEqual(day(2026, 10, 18));
    expect(todayIn("Asia/Tokyo", sundayNight)).toEqual(day(2026, 10, 19));
    // And still Sunday afternoon in Los Angeles.
    expect(todayIn("America/Los_Angeles", sundayNight)).toEqual(day(2026, 10, 18));
  });
});

describe("arithmetic", () => {
  it("moves by calendar days across a month and a year end", () => {
    expect(addDays(day(2026, 10, 14), 18)).toEqual(day(2026, 11, 1));
    expect(addDays(day(2026, 12, 31), 1)).toEqual(day(2027, 1, 1));
    expect(addDays(day(2026, 3, 1), -1)).toEqual(day(2026, 2, 28));
    expect(daysBetween(day(2026, 10, 14), day(2026, 11, 1))).toBe(18);
  });

  it("counts whole days across a clock change", () => {
    // Paris moves its clocks back on 25 October 2026.
    expect(daysBetween(day(2026, 10, 24), day(2026, 10, 26))).toBe(2);
    expect(addDays(day(2026, 10, 24), 2)).toEqual(day(2026, 10, 26));
  });

  it("compares days by date alone", () => {
    expect(sameDay(day(2026, 10, 5), { ...day(2026, 10, 5) })).toBe(true);
    expect(sameDay(day(2026, 10, 5), day(2026, 11, 5))).toBe(false);
  });

  it("knows how long a month is", () => {
    expect(daysInMonth(2026, 2)).toBe(28);
    expect(daysInMonth(2028, 2)).toBe(29);
    expect(daysInMonth(2026, 10)).toBe(31);
  });
});

describe("weeks", () => {
  it("numbers weekdays from Monday, as the API does", () => {
    expect(weekdayIndex(day(2026, 10, 12))).toBe(0);
    expect(weekdayIndex(day(2026, 10, 18))).toBe(6);
  });

  it("starts a week on the Monday on or before a day", () => {
    expect(mondayOf(day(2026, 10, 14))).toEqual(day(2026, 10, 12));
    expect(mondayOf(day(2026, 10, 12))).toEqual(day(2026, 10, 12));
    expect(mondayOf(day(2026, 11, 1))).toEqual(day(2026, 10, 26));
  });

  it("runs seven days from its start, across a month end", () => {
    const week = weekFrom(day(2026, 10, 26));
    expect(week).toHaveLength(7);
    expect(week[6]).toEqual(day(2026, 11, 1));
  });
});

describe("monthWeeks", () => {
  it("fits the weeks a month touches, Monday first", () => {
    // October 2026 starts on a Thursday and ends on a Saturday: five weeks.
    const weeks = monthWeeks(2026, 10, { rows: "fit" });
    expect(weeks).toHaveLength(5);
    expect(weeks[0]?.[0]).toEqual({ ...day(2026, 9, 28), inMonth: false });
    expect(weeks[0]?.[3]).toEqual({ ...day(2026, 10, 1), inMonth: true });
    expect(weeks.flat().filter((cell) => cell.inMonth)).toHaveLength(31);
  });

  it("needs six weeks for a month that starts on a Sunday and runs long", () => {
    // 1 November 2026 is a Sunday; the month ends on a Monday.
    const weeks = monthWeeks(2026, 11, { rows: "fit" });
    expect(weeks).toHaveLength(6);
    expect(weeks[0]?.[6]).toMatchObject({ month: 11, day: 1, inMonth: true });
  });

  it("starts on the first when the first is a Monday", () => {
    // 1 June 2026 is a Monday.
    expect(monthWeeks(2026, 6, { rows: "fit" })[0]?.[0]).toEqual({
      ...day(2026, 6, 1),
      inMonth: true,
    });
  });

  it("always gives six weeks when asked, so a grid keeps its height", () => {
    const weeks = monthWeeks(2026, 10, { rows: "six" });
    expect(weeks).toHaveLength(6);
    expect(weeks.at(-1)?.at(-1)).toEqual({ ...day(2026, 11, 8), inMonth: false });
  });

  it("keeps every day across a clock change", () => {
    const october = monthWeeks(2026, 10, { rows: "six" })
      .flat()
      .map((cell) => cell.day);
    expect(october.slice(27, 30)).toEqual([25, 26, 27]);
  });

  it("moves between months across a year end", () => {
    expect(shiftMonth(2026, 10, 3)).toEqual({ year: 2027, month: 1 });
    expect(shiftMonth(2026, 1, -1)).toEqual({ year: 2025, month: 12 });
  });
});

describe("labels", () => {
  it("writes a week month first, joined with to", () => {
    expect(weekLabel(day(2026, 10, 19))).toBe("Oct 19 to 25");
    expect(weekLabel(day(2026, 10, 26))).toBe("Oct 26 to Nov 1");
  });

  it("writes months and days", () => {
    expect(monthLabel(2026, 10)).toBe("October 2026");
    expect(shortDayLabel(day(2026, 10, 14))).toBe("Wed, Oct 14");
    expect(longDayLabel(day(2026, 10, 14))).toBe("Wednesday, October 14");
    expect(shortWeekday(day(2026, 10, 12))).toBe("Mon");
  });

  it("names today and tomorrow, even across a year end", () => {
    const today = day(2026, 12, 31);
    expect(relativeDayLabel(today, today)).toBe("Today");
    expect(relativeDayLabel(addDays(today, 1), today)).toBe("Tomorrow");
    expect(relativeDayLabel(addDays(today, 2), today)).toBe("Saturday, January 2");
  });
});
