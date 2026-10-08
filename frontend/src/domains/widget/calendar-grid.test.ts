import { describe, expect, it } from "vitest";

import { calendarWeeks, monthLabel, shiftMonth } from "./calendar-grid";

describe("calendarWeeks", () => {
  it("starts October 2026 on Thursday in a Monday-first grid", () => {
    const weeks = calendarWeeks(2026, 10, true);
    expect(weeks[0]).toEqual([null, null, null, 1, 2, 3, 4]);
    expect(weeks.at(-1)).toEqual([26, 27, 28, 29, 30, 31, null]);
  });

  it("starts the same month on Sunday by default", () => {
    expect(calendarWeeks(2026, 10)[0]).toEqual([null, null, null, null, 1, 2, 3]);
  });

  it("keeps every week seven cells long", () => {
    for (const week of calendarWeeks(2024, 2, true)) expect(week).toHaveLength(7);
    expect(calendarWeeks(2024, 2, true).flat().filter(Boolean)).toHaveLength(29);
  });
});

describe("shiftMonth", () => {
  it("rolls over the year in both directions", () => {
    expect(shiftMonth(2026, 12, 1)).toEqual({ year: 2027, month: 1 });
    expect(shiftMonth(2026, 1, -1)).toEqual({ year: 2025, month: 12 });
  });
});

describe("monthLabel", () => {
  it("names the month and year", () => {
    expect(monthLabel(2026, 10)).toBe("October 2026");
  });
});
