import { describe, expect, it } from "vitest";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { weekHours } from "./week-hours";

function rule(weekday: number, start: string, end: string): AvailabilityRule {
  return {
    id: weekday,
    host_unid: "host-1",
    weekday,
    start_time: start,
    end_time: end,
    slot_minutes: 30,
  };
}

describe("weekHours", () => {
  it("shows a working day when no hours are set", () => {
    expect(weekHours([])).toEqual({ first: 8, last: 18 });
  });

  it("runs from the earliest start to the latest end, in whole hours", () => {
    const rules = [rule(0, "09:30:00", "12:00:00"), rule(3, "07:00:00", "17:15:00")];
    expect(weekHours(rules)).toEqual({ first: 7, last: 18 });
  });

  it("never shows fewer than six hours", () => {
    expect(weekHours([rule(0, "10:00:00", "11:00:00")])).toEqual({ first: 10, last: 16 });
  });

  it("keeps a short late range inside the day", () => {
    expect(weekHours([rule(0, "22:00:00", "23:30:00")])).toEqual({ first: 18, last: 24 });
  });
});
