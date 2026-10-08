import { describe, expect, it } from "vitest";

import type { AdminHost } from "../../bindings/AdminHost";
import { HOST_GAPS, hostGaps } from "./host-setup";

const READY: AdminHost = {
  unid: "host-1",
  full_name: "Ana Kovac",
  email: "ana@example.com",
  status: "Active",
  created_at: "2026-10-01T00:00:00Z",
  public_slug: "ana",
  page_active: true,
  has_hours: true,
  calendar_connected: true,
  has_meeting_link: false,
};

describe("hostGaps", () => {
  it("finds nothing missing for a host who is set up", () => {
    expect(hostGaps(READY)).toEqual([]);
  });

  it("reports only 'not started' before a booking page exists", () => {
    expect(hostGaps({ ...READY, public_slug: null, has_hours: false })).toEqual([
      HOST_GAPS.notStarted,
    ]);
  });

  it("accepts a meeting link in place of Google Calendar", () => {
    expect(
      hostGaps({ ...READY, calendar_connected: false, has_meeting_link: true }),
    ).toEqual([]);
  });

  it("lists every gap of a started page", () => {
    expect(hostGaps({ ...READY, has_hours: false, calendar_connected: false })).toEqual([
      HOST_GAPS.noHours,
      HOST_GAPS.noCallLink,
    ]);
  });
});
