import { describe, expect, it } from "vitest";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { lengthLabel } from "../../lib/format";
import {
  callsIn,
  dragSpan,
  foldInto,
  hoursLabel,
  hoursOn,
  minutesAt,
  roomyWeek,
  snap,
  weekdayOf,
  weekTotals,
} from "./hours-math";

function rule(id: number, weekday: number, start: string, end: string): AvailabilityRule {
  return {
    id,
    host_unid: "host-1",
    weekday,
    start_time: `${start}:00`,
    end_time: `${end}:00`,
    slot_minutes: 30,
  };
}

describe("snap and minutesAt", () => {
  it("lands on the quarter hour inside the day", () => {
    expect(snap(547)).toBe(540);
    expect(snap(553)).toBe(555);
    expect(snap(-20)).toBe(0);
    expect(snap(1500)).toBe(1440);
  });

  it("reads the time under the pointer from the week's hours", () => {
    const week = { first: 8, last: 18 };
    expect(minutesAt(100, 100, 1000, week)).toBe(480);
    expect(minutesAt(600, 100, 1000, week)).toBe(780);
    expect(minutesAt(5000, 100, 1000, week)).toBe(1080);
  });
});

describe("totals", () => {
  it("counts whole calls only", () => {
    expect(callsIn({ from: 540, to: 615, slotMinutes: 30 })).toBe(2);
  });

  it("adds up the week", () => {
    const rules = [rule(1, 0, "09:00", "17:00"), rule(2, 1, "09:00", "12:00")];
    expect(weekTotals(rules)).toEqual({ minutes: 660, calls: 22 });
    expect(hoursLabel(660)).toBe("11 hours");
    expect(hoursLabel(450)).toBe("7.5 hours");
    expect(hoursLabel(60)).toBe("1 hour");
  });

  it("gives the ruler an hour of room inside the day", () => {
    expect(roomyWeek({ first: 8, last: 18 })).toEqual({ first: 7, last: 19 });
    expect(roomyWeek({ first: 0, last: 24 })).toEqual({ first: 0, last: 24 });
  });
});

describe("foldInto", () => {
  const morning = rule(1, 0, "09:00", "12:00");
  const afternoon = rule(2, 0, "13:00", "17:00");

  it("adds a range that touches nothing", () => {
    expect(foldInto([morning, afternoon], 1080, 1140)).toEqual({
      keep: undefined,
      from: 1080,
      to: 1140,
      drop: [],
    });
  });

  it("grows the range it overlaps", () => {
    const folded = foldInto([morning, afternoon], 660, 750);
    expect(folded.keep).toBe(morning);
    expect([folded.from, folded.to]).toEqual([540, 750]);
  });

  it("joins two ranges a new one bridges", () => {
    const folded = foldInto([morning, afternoon], 690, 810);
    expect(folded.keep).toBe(morning);
    expect([folded.from, folded.to]).toEqual([540, 1020]);
    expect(folded.drop).toEqual([afternoon]);
  });

  it("keeps the range being moved and drops what it lands on", () => {
    const folded = foldInto([morning, afternoon], 750, 900, morning);
    expect(folded.keep).toBe(morning);
    expect([folded.from, folded.to]).toEqual([750, 1020]);
    expect(folded.drop).toEqual([afternoon]);
  });
});

describe("hoursOn", () => {
  const rules = [rule(1, 0, "09:00", "17:00")];

  it("reads Monday as weekday 0", () => {
    expect(weekdayOf("2026-10-12")).toBe(0);
    expect(weekdayOf("2026-10-18")).toBe(6);
  });

  it("uses the weekday's hours on an ordinary date", () => {
    expect(hoursOn("2026-10-12", rules, [])).toEqual({
      spans: [{ from: 540, to: 1020, slotMinutes: 30 }],
      own: false,
    });
  });

  it("uses a date's own hours, or none on a day off", () => {
    expect(hoursOn("2026-10-12", rules, [{ date: "2026-10-12", ranges: [] }])).toEqual({
      spans: [],
      own: true,
    });
    const short = {
      date: "2026-10-12",
      ranges: [{ start_time: "10:00:00", end_time: "11:00:00", slot_minutes: 60 }],
    };
    expect(hoursOn("2026-10-12", rules, [short]).spans).toEqual([
      { from: 600, to: 660, slotMinutes: 60 },
    ]);
  });
});

describe("dragSpan", () => {
  const base = { from: 540, to: 720, slotMinutes: 30 };
  const bounds = { min: 420, max: 1140 };

  it("moves by the distance from the press, however many moves arrive", () => {
    // Pressed at 10:00, pointer now at 11:00: one hour later, every time.
    expect(dragSpan("move", base, 600, 660, bounds)).toEqual({
      ...base,
      from: 600,
      to: 780,
    });
    expect(dragSpan("move", base, 600, 660, bounds)).toEqual({
      ...base,
      from: 600,
      to: 780,
    });
    expect(dragSpan("move", base, 600, 600, bounds)).toEqual(base);
  });

  it("keeps a moved range inside the track", () => {
    expect(dragSpan("move", base, 600, 1400, bounds)).toEqual({
      ...base,
      from: 960,
      to: 1140,
    });
    expect(dragSpan("move", base, 600, 0, bounds)).toEqual({
      ...base,
      from: 420,
      to: 600,
    });
  });

  it("stretches one end and never past the other", () => {
    expect(dragSpan("end", base, 720, 900, bounds).to).toBe(900);
    expect(dragSpan("end", base, 720, 300, bounds).to).toBe(555);
    expect(dragSpan("start", base, 540, 480, bounds).from).toBe(480);
    expect(dragSpan("start", base, 540, 2000, bounds).from).toBe(705);
  });

  it("draws either way from the press, at least a quarter hour long", () => {
    expect(dragSpan("draw", base, 600, 540, bounds)).toMatchObject({
      from: 540,
      to: 600,
    });
    expect(dragSpan("draw", base, 600, 600, bounds)).toMatchObject({
      from: 600,
      to: 615,
    });
    expect(dragSpan("draw", base, 1140, 1140, bounds)).toMatchObject({
      from: 1125,
      to: 1140,
    });
  });

  it("writes lengths the way the time picker does", () => {
    expect(lengthLabel(45)).toBe("45 min");
    expect(lengthLabel(120)).toBe("2 h");
    expect(lengthLabel(450)).toBe("7 h 30 min");
  });
});
