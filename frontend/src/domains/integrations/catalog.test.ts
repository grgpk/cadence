import { describe, expect, it } from "vitest";

import {
  APP_CATEGORY,
  APP_SLUG,
  APP_TONE,
  APPS,
  appState,
  filterApps,
  isInstalled,
} from "./catalog";
import { CONFERENCING_APP } from "./conferencing";

const MEET = { conferencing_app: CONFERENCING_APP.googleMeet, meeting_link: null };
const LINK = {
  conferencing_app: CONFERENCING_APP.customLink,
  meeting_link: "https://zoom.us/j/1",
};

describe("isInstalled", () => {
  it("counts Google Calendar only when it is connected", () => {
    expect(isInstalled(APP_SLUG.googleCalendar, MEET, true)).toBe(true);
    expect(isInstalled(APP_SLUG.googleCalendar, MEET, false)).toBe(false);
  });

  it("counts the meeting app the host picked", () => {
    expect(isInstalled(APP_SLUG.googleMeet, MEET, false)).toBe(true);
    expect(isInstalled(APP_SLUG.customLink, MEET, false)).toBe(false);
    expect(isInstalled(APP_SLUG.customLink, LINK, false)).toBe(true);
    expect(isInstalled(APP_SLUG.googleMeet, LINK, true)).toBe(false);
  });

  it("does not count a custom link with no link", () => {
    expect(isInstalled(APP_SLUG.customLink, { ...LINK, meeting_link: null }, false)).toBe(
      false,
    );
  });

  it("never counts an app that is not out yet", () => {
    expect(isInstalled(APP_SLUG.zoom, LINK, true)).toBe(false);
  });
});

describe("filterApps", () => {
  it("matches name or summary, ignoring case", () => {
    expect(filterApps(APPS, "ZOOM", undefined).map((app) => app.slug)).toEqual([
      APP_SLUG.customLink,
      APP_SLUG.zoom,
    ]);
  });

  it("keeps one category", () => {
    const calendars = filterApps(APPS, "", APP_CATEGORY.calendar);
    expect(calendars.every((app) => app.category === APP_CATEGORY.calendar)).toBe(true);
    expect(calendars).toHaveLength(3);
  });
});

describe("appState", () => {
  it("says what an app is doing", () => {
    expect(appState(APP_SLUG.googleCalendar, MEET, true)).toEqual({
      tone: APP_TONE.on,
      text: "Busy times blocked",
    });
    expect(appState(APP_SLUG.customLink, LINK, false)).toEqual({
      tone: APP_TONE.on,
      text: "zoom.us/j/1",
    });
    expect(appState(APP_SLUG.customLink, MEET, true).tone).toBe(APP_TONE.off);
  });

  it("warns when Meet is picked without a calendar to make its links", () => {
    expect(appState(APP_SLUG.googleMeet, MEET, false)).toEqual({
      tone: APP_TONE.warn,
      text: "Needs Google Calendar",
    });
  });
});
