import { describe, expect, it } from "vitest";

import {
  addDays,
  isSlotLayout,
  relativeDayLabel,
  sharedLength,
  slotsByDay,
  weekDays,
  weekLabel,
} from "./slot-days";

const slot = (start: string) => ({ start, minutes: 30 });

describe("slotsByDay", () => {
  it("groups times by the visitor's day, earliest first", () => {
    const days = slotsByDay(
      [
        slot("2026-10-18T09:00:00Z"),
        slot("2026-10-17T12:30:00Z"),
        slot("2026-10-17T12:00:00Z"),
      ],
      "UTC",
    );
    expect(days.map((day) => day.key)).toEqual(["2026-10-17", "2026-10-18"]);
    expect(days[0]?.slots.map((s) => s.start)).toEqual([
      "2026-10-17T12:00:00Z",
      "2026-10-17T12:30:00Z",
    ]);
  });

  it("puts a time on the day of the zone, not of UTC", () => {
    const [day] = slotsByDay([slot("2026-10-17T23:30:00Z")], "Asia/Tokyo");
    expect(day?.key).toBe("2026-10-18");
  });

  it("keeps a day whole across a clock change", () => {
    // Paris moves its clocks back on 25 October 2026.
    const days = slotsByDay(
      [slot("2026-10-25T00:30:00Z"), slot("2026-10-25T22:30:00Z")],
      "Europe/Paris",
    );
    expect(days.map((day) => day.key)).toEqual(["2026-10-25"]);
  });
});

describe("weeks", () => {
  it("runs seven days, across a month end", () => {
    const days = weekDays({ year: 2026, month: 10, day: 26 });
    expect(days).toHaveLength(7);
    expect(days[6]).toEqual({ year: 2026, month: 11, day: 1 });
  });

  it("labels a week inside one month and across two", () => {
    expect(weekLabel({ year: 2026, month: 10, day: 19 })).toBe("Oct 19 – 25");
    expect(weekLabel({ year: 2026, month: 10, day: 26 })).toBe("Oct 26 – Nov 1");
  });
});

describe("relativeDayLabel", () => {
  const today = { year: 2026, month: 12, day: 31 };

  it("names today and tomorrow, even across a year end", () => {
    expect(relativeDayLabel(today, today)).toBe("Today");
    expect(relativeDayLabel(addDays(today, 1), today)).toBe("Tomorrow");
  });

  it("spells out any later day", () => {
    expect(relativeDayLabel(addDays(today, 2), today)).toBe("Saturday, January 2");
  });
});

it("accepts only known layouts", () => {
  expect(isSlotLayout("week")).toBe(true);
  expect(isSlotLayout("grid")).toBe(false);
});

it("knows the call length only when every time agrees", () => {
  expect(sharedLength([slot("2026-10-17T12:00:00Z"), slot("2026-10-18T12:00:00Z")])).toBe(
    30,
  );
  expect(
    sharedLength([slot("2026-10-17T12:00:00Z"), { start: "x", minutes: 45 }]),
  ).toBeNull();
  expect(sharedLength([])).toBeNull();
});
