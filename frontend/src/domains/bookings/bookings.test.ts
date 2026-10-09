import { describe, expect, it } from "vitest";

import type { Booking } from "../../bindings/Booking";
import { agendaDays, matchesSearch, splitBookings } from "./bookings";

const NOW = Date.parse("2026-10-14T12:00:00Z");

function booking(unid: string, slotStart: string, status: Booking["status"]): Booking {
  return {
    unid,
    host_unid: "host-1",
    lead_unid: null,
    slot_start: slotStart,
    timezone: "Europe/Paris",
    invitee_name: "Ana Kovac",
    invitee_email: "ana@example.com",
    status,
    meeting_url: null,
    created_at: "2026-10-01T00:00:00Z",
  };
}

function ids(bookings: Booking[]): string[] {
  return bookings.map((item) => item.unid);
}

describe("splitBookings", () => {
  const all = [
    booking("later", "2026-10-20T09:00:00Z", "confirmed"),
    booking("old", "2026-10-01T09:00:00Z", "confirmed"),
    booking("soon", "2026-10-15T09:00:00Z", "pending"),
    booking("yesterday", "2026-10-13T09:00:00Z", "confirmed"),
    booking("called-off", "2026-10-16T09:00:00Z", "cancelled"),
  ];

  it("lists what is coming soonest first", () => {
    expect(ids(splitBookings(all, NOW).upcoming)).toEqual(["soon", "later"]);
  });

  it("keeps a cancelled call out of what is coming", () => {
    const { upcoming, past } = splitBookings(all, NOW);
    expect(ids(upcoming)).not.toContain("called-off");
    expect(ids(past)).toEqual(["called-off", "yesterday", "old"]);
  });

  it("counts a call that starts now as coming", () => {
    const now = booking("now", "2026-10-14T12:00:00Z", "confirmed");
    expect(ids(splitBookings([now], NOW).upcoming)).toEqual(["now"]);
  });
});

describe("agendaDays", () => {
  const upcoming = [
    booking("today-1", "2026-10-14T13:00:00Z", "confirmed"),
    booking("today-2", "2026-10-14T15:00:00Z", "confirmed"),
    booking("tomorrow", "2026-10-15T09:00:00Z", "confirmed"),
    booking("later", "2026-10-20T09:00:00Z", "confirmed"),
  ];

  it("groups calls under today, tomorrow and then the date", () => {
    const days = agendaDays(upcoming, "UTC", NOW);
    expect(days.map((day) => day.label)).toEqual(["Today", "Tomorrow", "Tue, Oct 20"]);
    expect(ids(days[0]?.bookings ?? [])).toEqual(["today-1", "today-2"]);
  });

  it("decides the day in the given timezone", () => {
    // 23:30 UTC on the 14th is already the 15th in Tokyo.
    const late = [booking("late", "2026-10-14T23:30:00Z", "confirmed")];
    expect(agendaDays(late, "UTC", NOW)[0]?.label).toBe("Today");
    expect(agendaDays(late, "Asia/Tokyo", NOW)[0]?.label).toBe("Tomorrow");
  });
});

describe("matchesSearch", () => {
  const ana = booking("ana", "2026-10-15T09:00:00Z", "confirmed");

  it("finds a call by part of the name or the email, in any case", () => {
    expect(matchesSearch(ana, "kov")).toBe(true);
    expect(matchesSearch(ana, "ANA@EXAMPLE")).toBe(true);
    expect(matchesSearch(ana, "  ana ")).toBe(true);
  });

  it("keeps everything when nothing is typed and drops what does not match", () => {
    expect(matchesSearch(ana, "  ")).toBe(true);
    expect(matchesSearch(ana, "leo")).toBe(false);
  });
});
