import { describe, expect, it } from "vitest";

import { CONFERENCING_APP } from "../integrations/conferencing";
import { bookingPageProblems } from "./booking-page-status";

const READY = { paused: false, hasHours: true, place: CONFERENCING_APP.googleMeet };

describe("bookingPageProblems", () => {
  it("finds nothing wrong with a page that is set up", () => {
    expect(bookingPageProblems(READY)).toEqual([]);
  });

  it("reports each problem with its own fix", () => {
    const problems = bookingPageProblems({ paused: true, hasHours: false, place: null });
    expect(problems.map((problem) => problem.fix.label)).toEqual([
      "Take bookings again",
      "Set your hours",
      "Add a meeting link",
    ]);
  });

  it("accepts the host's own meeting link", () => {
    expect(bookingPageProblems({ ...READY, place: CONFERENCING_APP.customLink })).toEqual(
      [],
    );
  });
});
