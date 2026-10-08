import { describe, expect, it } from "vitest";

import {
  clockMinutes,
  clockTime,
  formatClock,
  formatDateTime,
  formatDay,
  formatTime,
  weekdayName,
  zoneName,
} from "./format";

const INSTANT = "2026-10-14T13:05:00Z";

/** Some runtimes put a narrow no-break space before AM and PM. */
function plain(text: string): string {
  return text.replaceAll("\u202f", " ");
}

describe("formatTime", () => {
  it("writes the time in the asked style and zone", () => {
    expect(plain(formatTime(INSTANT, "UTC", true))).toBe("1:05 PM");
    expect(plain(formatTime(INSTANT, "UTC", false))).toBe("13:05");
    expect(plain(formatTime(INSTANT, "America/New_York", true))).toBe("9:05 AM");
  });

  it("writes midnight as 00 on a 24 hour clock", () => {
    expect(plain(formatTime("2026-10-14T00:10:00Z", "UTC", false))).toBe("00:10");
    expect(plain(formatTime("2026-10-14T00:10:00Z", "UTC", true))).toBe("12:10 AM");
  });
});

describe("formatDateTime", () => {
  it("keeps the date and follows the clock style", () => {
    expect(plain(formatDateTime(INSTANT, "UTC", true))).toBe("Oct 14, 2026, 1:05 PM");
    expect(plain(formatDateTime(INSTANT, "UTC", false))).toBe("Oct 14, 2026, 13:05");
  });
});

describe("formatDay", () => {
  it("uses the day of the zone", () => {
    expect(formatDay(INSTANT, "UTC")).toBe("Wed, Oct 14");
    expect(formatDay("2026-10-14T23:30:00Z", "Asia/Tokyo")).toBe("Thu, Oct 15");
  });
});

describe("clock helpers", () => {
  it("converts between a wall-clock time and minutes", () => {
    expect(clockMinutes("09:30")).toBe(570);
    expect(clockMinutes("09:30:00")).toBe(570);
    expect(clockTime(570)).toBe("09:30");
    expect(clockTime(0)).toBe("00:00");
  });

  it("formats a wall-clock time in both styles", () => {
    expect(formatClock("17:00:00", true)).toBe("5:00 PM");
    expect(formatClock("17:00:00", false)).toBe("17:00");
    expect(formatClock("00:15", true)).toBe("12:15 AM");
    expect(formatClock("9:5", false)).toBe("09:05");
  });
});

describe("zoneName", () => {
  it("reads an IANA name without underscores", () => {
    expect(zoneName("America/New_York")).toBe("America/New York");
  });
});

describe("weekdayName", () => {
  it("counts from Monday", () => {
    expect(weekdayName(0)).toBe("Monday");
    expect(weekdayName(6)).toBe("Sunday");
    expect(weekdayName(9)).toBe("Day 9");
  });
});
