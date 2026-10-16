import { describe, expect, it } from "vitest";

import type { Booking } from "../../bindings/Booking";
import {
  agendaDays,
  dayCall,
  dayPlan,
  firstName,
  lanes,
  matchesSearch,
  splitBookings,
} from "./bookings";

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
    minutes: 30,
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

  it("keeps a cancelled call in its own list, whatever its time", () => {
    const { upcoming, past, cancelled } = splitBookings(all, NOW);
    expect(ids(upcoming)).not.toContain("called-off");
    expect(ids(past)).toEqual(["yesterday", "old"]);
    expect(ids(cancelled)).toEqual(["called-off"]);
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

describe("agendaDays across a clock change", () => {
  // Clocks go back in Paris on 25 October 2026, so that day lasts 25 hours.
  const lateOnTheLongDay = Date.parse("2026-10-24T22:30:00Z"); // 00:30 on the 25th
  const stillTheLongDay = [booking("same", "2026-10-25T22:30:00Z", "confirmed")]; // 23:30 on the 25th
  const nextDay = [booking("next", "2026-10-26T08:00:00Z", "confirmed")];

  it("keeps a call 24 hours later on the same day as today", () => {
    expect(agendaDays(stillTheLongDay, "Europe/Paris", lateOnTheLongDay)[0]?.label).toBe(
      "Today",
    );
  });

  it("calls the next calendar day tomorrow", () => {
    expect(agendaDays(nextDay, "Europe/Paris", lateOnTheLongDay)[0]?.label).toBe(
      "Tomorrow",
    );
  });
});

describe("dayCall", () => {
  it("places a call on its day and clock in the given timezone", () => {
    const call = { ...booking("late", "2026-10-14T22:30:00Z", "confirmed"), minutes: 45 };
    expect(dayCall(call, "UTC")).toMatchObject({
      day: { year: 2026, month: 10, day: 14 },
      from: 22 * 60 + 30,
      to: 22 * 60 + 30 + 45,
    });
    // 07:30 the next morning in Tokyo.
    expect(dayCall(call, "Asia/Tokyo")).toMatchObject({
      day: { year: 2026, month: 10, day: 15 },
      from: 7 * 60 + 30,
    });
  });

  it("uses the call's own length, not the hours it was booked from", () => {
    const hour = { ...booking("long", "2026-10-14T09:00:00Z", "confirmed"), minutes: 60 };
    expect(dayCall(hour, "UTC").to - dayCall(hour, "UTC").from).toBe(60);
  });
});

describe("firstName", () => {
  it("takes the first word of a name", () => {
    expect(firstName("  Mia  Chen ")).toBe("Mia");
    expect(firstName("Noor")).toBe("Noor");
  });
});

describe("lanes", () => {
  it("keeps calls that do not overlap in one full-width lane", () => {
    const placed = lanes([
      { from: 540, to: 570 },
      { from: 570, to: 600 },
    ]);
    expect(placed.map(({ lane, lanes: width }) => [lane, width])).toEqual([
      [0, 1],
      [0, 1],
    ]);
  });

  it("puts overlapping calls side by side and reuses a free lane", () => {
    const placed = lanes([
      { from: 540, to: 600 },
      { from: 550, to: 580 },
      { from: 585, to: 620 },
      { from: 700, to: 730 },
    ]);
    expect(placed.map(({ lane, lanes: width }) => [lane, width])).toEqual([
      [0, 2],
      [1, 2],
      [1, 2],
      [0, 1],
    ]);
  });
});

describe("dayPlan", () => {
  const all = [
    booking("tomorrow", "2026-10-15T09:00:00Z", "confirmed"),
    booking("afternoon", "2026-10-14T15:00:00Z", "confirmed"),
    booking("under-way", "2026-10-14T11:45:00Z", "confirmed"),
    booking("morning", "2026-10-14T08:00:00Z", "confirmed"),
    booking("called-off", "2026-10-14T13:00:00Z", "cancelled"),
    booking("called-off-later", "2026-10-16T09:00:00Z", "cancelled"),
  ];

  it("lists today's calls in time order, cancelled ones included", () => {
    expect(ids(dayPlan(all, "UTC", NOW).today)).toEqual([
      "morning",
      "under-way",
      "called-off",
      "afternoon",
    ]);
  });

  it("picks the call under way and the next live call today", () => {
    const plan = dayPlan(all, "UTC", NOW);
    expect(plan.current?.unid).toBe("under-way");
    expect(plan.next?.unid).toBe("afternoon");
  });

  it("has no next call once today's calls are over", () => {
    const plan = dayPlan(all, "UTC", Date.parse("2026-10-14T18:00:00Z"));
    expect(plan.current).toBeNull();
    expect(plan.next).toBeNull();
  });

  it("keeps later days apart, without cancelled calls", () => {
    expect(ids(dayPlan(all, "UTC", NOW).later)).toEqual(["tomorrow"]);
  });

  it("reads today on the host's clock", () => {
    // 23:30 in UTC on the 14th is already the 15th in Tbilisi.
    const late = [booking("late", "2026-10-14T23:30:00Z", "confirmed")];
    expect(ids(dayPlan(late, "Asia/Tbilisi", NOW).later)).toEqual(["late"]);
    expect(ids(dayPlan(late, "UTC", NOW).today)).toEqual(["late"]);
  });
});
