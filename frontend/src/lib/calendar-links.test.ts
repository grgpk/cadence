import { describe, expect, it } from "vitest";

import {
  googleCalendarUrl,
  icsFile,
  icsHref,
  type CalendarEvent,
} from "./calendar-links";

const EVENT: CalendarEvent = {
  title: "Call with Ana; intro, part 1",
  start: new Date("2026-10-12T08:00:00Z"),
  end: new Date("2026-10-12T08:30:00Z"),
  location: "https://meet.google.com/abc-defg-hij",
  uid: "2026-10-12T08:00:00Z-ana@cadence",
};

describe("googleCalendarUrl", () => {
  it("fills in the title, the times in UTC and the place", () => {
    const url = new URL(googleCalendarUrl(EVENT));
    expect(url.origin + url.pathname).toBe("https://calendar.google.com/calendar/render");
    expect(url.searchParams.get("text")).toBe(EVENT.title);
    expect(url.searchParams.get("dates")).toBe("20261012T080000Z/20261012T083000Z");
    expect(url.searchParams.get("location")).toBe(EVENT.location);
  });

  it("leaves the place out when there is none", () => {
    const url = new URL(googleCalendarUrl({ ...EVENT, location: undefined }));
    expect(url.searchParams.has("location")).toBe(false);
  });
});

describe("icsFile", () => {
  it("writes one event with escaped text and CRLF line ends", () => {
    const lines = icsFile(EVENT).split("\r\n");
    expect(lines).toContain("DTSTART:20261012T080000Z");
    expect(lines).toContain("DTEND:20261012T083000Z");
    expect(lines).toContain("SUMMARY:Call with Ana\\; intro\\, part 1");
    expect(lines.filter((line) => line === "BEGIN:VEVENT")).toHaveLength(1);
    expect(lines.at(-2)).toBe("END:VCALENDAR");
  });

  it("is the same file every time it is built", () => {
    expect(icsFile(EVENT)).toBe(icsFile(EVENT));
  });
});

describe("icsHref", () => {
  it("is a calendar data link holding the file", () => {
    const href = icsHref(EVENT);
    expect(href.startsWith("data:text/calendar;charset=utf-8,")).toBe(true);
    expect(decodeURIComponent(href.split(",")[1] ?? "")).toBe(icsFile(EVENT));
  });
});
