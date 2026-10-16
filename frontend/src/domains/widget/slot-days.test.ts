import { describe, expect, it } from "vitest";

import { weekFrom } from "../../lib/civil-day";
import { isSlotLayout, sharedLength, slotsByDay, weekTimetable } from "./slot-days";

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

describe("weekTimetable", () => {
  const week = weekFrom({ year: 2026, month: 10, day: 17 });

  it("lines the same time up across days, earliest row first", () => {
    const days = slotsByDay(
      [
        slot("2026-10-17T14:00:00Z"),
        slot("2026-10-19T09:00:00Z"),
        slot("2026-10-19T14:00:00Z"),
      ],
      "UTC",
    );
    const rows = weekTimetable(days, week, "UTC");
    expect(rows.map((row) => row.minutes)).toEqual([9 * 60, 14 * 60]);
    expect(rows[0]?.cells.map((cell) => cell?.start ?? null)).toEqual([
      null,
      null,
      "2026-10-19T09:00:00Z",
      null,
      null,
      null,
      null,
    ]);
    expect(rows[1]?.cells[0]?.start).toBe("2026-10-17T14:00:00Z");
    expect(rows[1]?.cells[2]?.start).toBe("2026-10-19T14:00:00Z");
  });

  it("leaves out days past the week", () => {
    const days = slotsByDay([slot("2026-10-30T09:00:00Z")], "UTC");
    expect(weekTimetable(days, week, "UTC")).toEqual([]);
  });
});
