import { describe, expect, it } from "vitest";

import { slotsByDay } from "./slot-days";
import {
  DAYS_PER_PAGE,
  firstOpenFrom,
  listPage,
  monthSteps,
  openDaysIn,
  openingMonth,
  pickTime,
  releasePick,
  selectedDay,
  weekAround,
  weekSteps,
} from "./slot-picker-rules";

/** Open at noon UTC on each of these dates. */
const openOn = (...dates: string[]) =>
  slotsByDay(
    dates.map((date) => ({ start: `${date}T12:00:00Z`, minutes: 30 })),
    "UTC",
  );

const OCTOBER = { year: 2026, month: 10, day: null };
const TODAY = { year: 2026, month: 10, day: 16 };

describe("the month view", () => {
  const days = openOn("2026-10-17", "2026-10-20", "2026-11-03");

  it("opens on this month, or on the first month with times when that is later", () => {
    expect(openingMonth(days, TODAY)).toEqual(OCTOBER);
    expect(openingMonth(openOn("2026-12-01"), TODAY)).toEqual({
      year: 2026,
      month: 12,
      day: null,
    });
    expect(openingMonth([], TODAY)).toEqual(OCTOBER);
  });

  it("shows the picked day while it has times, else the month's first that does", () => {
    expect(openDaysIn(days, OCTOBER)).toEqual([17, 20]);
    expect(selectedDay(days, OCTOBER)).toBe(17);
    expect(selectedDay(days, { ...OCTOBER, day: 20 })).toBe(20);
    expect(selectedDay(days, { ...OCTOBER, day: 21 }), "no times on the 21st").toBe(17);
    expect(selectedDay(days, { year: 2026, month: 12, day: null })).toBeNull();
  });

  it("moves only to months that have something to pick", () => {
    expect(monthSteps(days, OCTOBER)).toEqual({ earlier: false, later: true });
    expect(monthSteps(days, { year: 2026, month: 11, day: null })).toEqual({
      earlier: true,
      later: false,
    });
  });
});

describe("the week view", () => {
  const days = openOn(
    "2026-10-17",
    "2026-10-19",
    "2026-10-23",
    "2026-10-24",
    "2026-11-05",
  );

  it("opens on the first open day without a picked day", () => {
    expect(weekAround(days, null)?.key).toBe("2026-10-17");
  });

  it("keeps a picked day inside the week, not at its start", () => {
    expect(weekAround(days, { year: 2026, month: 10, day: 23 })?.key).toBe("2026-10-17");
  });

  it("moves on to the week that holds a later day", () => {
    expect(weekAround(days, { year: 2026, month: 10, day: 24 })?.key).toBe("2026-10-24");
    // Nothing is open in the week after Oct 24, so the next week starts on Nov 5.
    expect(weekAround(days, { year: 2026, month: 11, day: 5 })?.key).toBe("2026-11-05");
  });

  it("finds the first open day on or after a day", () => {
    expect(firstOpenFrom(days, { year: 2026, month: 10, day: 20 })?.key).toBe(
      "2026-10-23",
    );
  });

  it("steps to the next open week and back, never to a week with nothing", () => {
    const first = { year: 2026, month: 10, day: 17 };
    expect(weekSteps(days, first)).toMatchObject({
      earlier: null,
      later: { key: "2026-10-24" },
    });
    const second = { year: 2026, month: 10, day: 24 };
    expect(weekSteps(days, second)).toMatchObject({
      earlier: { key: "2026-10-17" },
      later: { key: "2026-11-05" },
    });
    const last = { year: 2026, month: 11, day: 5 };
    expect(weekSteps(days, last).later).toBeNull();
  });
});

describe("the list view", () => {
  const days = openOn(
    "2026-10-17",
    "2026-10-18",
    "2026-10-19",
    "2026-10-20",
    "2026-10-21",
    "2026-10-22",
  );

  it("shows a page of days and says when there are more", () => {
    const first = listPage(days, DAYS_PER_PAGE);
    expect(first.days).toHaveLength(5);
    expect(first.more).toBe(true);
    const all = listPage(days, DAYS_PER_PAGE * 2);
    expect(all.days).toHaveLength(6);
    expect(all.more).toBe(false);
  });
});

describe("a pick", () => {
  it("holds one time until it is released", () => {
    const first = pickTime(releasePick(), "09:00");
    expect(first).toBe("09:00");
    expect(pickTime(first, "10:00"), "a second click changes nothing").toBe("09:00");
    expect(pickTime(releasePick(), "10:00")).toBe("10:00");
  });
});
