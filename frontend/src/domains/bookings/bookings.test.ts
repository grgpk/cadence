import { describe, expect, it } from "vitest";

import type { Booking } from "../../bindings/Booking";
import { splitBookings } from "./bookings";

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
