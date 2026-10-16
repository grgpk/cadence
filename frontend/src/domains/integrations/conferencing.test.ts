import { describe, expect, it } from "vitest";

import { callPlace, CONFERENCING_APP } from "./conferencing";

describe("callPlace", () => {
  it("uses Google Meet only with a connected calendar", () => {
    expect(
      callPlace({
        app: CONFERENCING_APP.googleMeet,
        calendarConnected: true,
        hasLink: true,
      }),
    ).toBe(CONFERENCING_APP.googleMeet);
    expect(
      callPlace({
        app: CONFERENCING_APP.googleMeet,
        calendarConnected: false,
        hasLink: true,
      }),
    ).toBe(CONFERENCING_APP.customLink);
  });

  it("uses the host's link when they picked it, calendar or not", () => {
    expect(
      callPlace({
        app: CONFERENCING_APP.customLink,
        calendarConnected: true,
        hasLink: true,
      }),
    ).toBe(CONFERENCING_APP.customLink);
  });

  it("has no place without a calendar or a link", () => {
    expect(
      callPlace({
        app: CONFERENCING_APP.googleMeet,
        calendarConnected: false,
        hasLink: false,
      }),
    ).toBeNull();
  });
});
