import { describe, expect, it } from "vitest";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { callLengthLabel, daysLabel } from "./hours-labels";

function rule(weekday: number, slotMinutes = 30): AvailabilityRule {
  return {
    id: weekday * 100 + slotMinutes,
    host_unid: "host-1",
    weekday,
    start_time: "09:00:00",
    end_time: "17:00:00",
    slot_minutes: slotMinutes,
  };
}

describe("daysLabel", () => {
  it("joins a run of days and lists the rest", () => {
    expect(daysLabel([0, 1, 2, 3, 4].map((day) => rule(day)))).toBe("Mon to Fri");
    expect(daysLabel([rule(0), rule(1), rule(2), rule(6)])).toBe("Mon to Wed, Sun");
  });

  it("writes two neighbours as a pair, not a range", () => {
    expect(daysLabel([rule(0), rule(1)])).toBe("Mon, Tue");
  });

  it("counts a day once however many ranges it has", () => {
    expect(daysLabel([rule(2), rule(2, 60)])).toBe("Wed");
  });

  it("says every day for a full week", () => {
    expect(daysLabel([0, 1, 2, 3, 4, 5, 6].map((day) => rule(day)))).toBe("Every day");
  });
});

describe("callLengthLabel", () => {
  it("names one length, or each when days differ", () => {
    expect(callLengthLabel([rule(0), rule(1)])).toBe("30 min calls");
    expect(callLengthLabel([rule(0, 60), rule(1, 20)])).toBe("20 and 60 min calls");
  });
});
