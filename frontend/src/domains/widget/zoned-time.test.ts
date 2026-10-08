import { describe, expect, it } from "vitest";

import { callEnd, clockLabel, dayPhase, zonedTime } from "./zoned-time";

describe("zonedTime", () => {
  it("reads an instant on the wall clock of a zone", () => {
    expect(zonedTime(new Date("2026-10-14T07:30:00Z"), "Europe/Paris")).toEqual({
      year: 2026,
      month: 10,
      day: 14,
      hour: 9,
      minute: 30,
    });
  });

  it("crosses the date line of the zone, not of UTC", () => {
    const at = zonedTime(new Date("2026-10-14T23:30:00Z"), "Asia/Tokyo");
    expect(at.day).toBe(15);
    expect(at.hour).toBe(8);
  });

  it("skips the hour lost when clocks go forward", () => {
    // Paris moves from 02:00 to 03:00 on 29 March 2026.
    expect(zonedTime(new Date("2026-03-29T00:30:00Z"), "Europe/Paris").hour).toBe(1);
    expect(zonedTime(new Date("2026-03-29T01:30:00Z"), "Europe/Paris").hour).toBe(3);
  });

  it("reports midnight as hour 0", () => {
    expect(zonedTime(new Date("2026-10-14T00:00:00Z"), "UTC").hour).toBe(0);
  });
});

describe("clockLabel", () => {
  const instant = new Date("2026-10-14T13:05:00Z");

  it("formats in 24h and 12h", () => {
    expect(clockLabel(instant, "UTC", false)).toBe("13:05");
    expect(clockLabel(instant, "UTC", true)).toBe("1:05 PM");
  });
});

describe("callEnd", () => {
  it("adds the call length to the start", () => {
    expect(callEnd("2026-10-14T09:30:00Z", 45).toISOString()).toBe(
      "2026-10-14T10:15:00.000Z",
    );
  });
});

describe("dayPhase", () => {
  it("splits the day at noon and five", () => {
    expect(dayPhase(11)).toBe("Morning");
    expect(dayPhase(12)).toBe("Afternoon");
    expect(dayPhase(16)).toBe("Afternoon");
    expect(dayPhase(17)).toBe("Evening");
  });
});
