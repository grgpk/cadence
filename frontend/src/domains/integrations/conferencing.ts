import type { ConferencingApp } from "../../bindings/ConferencingApp";

/** The values of `ConferencingApp`, so code never repeats the strings. */
export const CONFERENCING_APP = {
  googleMeet: "google_meet",
  customLink: "custom_link",
} as const satisfies Record<string, ConferencingApp>;

/**
 * Where a booked call will actually happen, by the same rule the API uses:
 * Google Meet when the host picked it and a calendar is connected, otherwise
 * the host's own link, otherwise nowhere.
 */
export function callPlace({
  app,
  calendarConnected,
  hasLink,
}: {
  app: ConferencingApp;
  calendarConnected: boolean;
  hasLink: boolean;
}): ConferencingApp | null {
  if (app === CONFERENCING_APP.googleMeet && calendarConnected) {
    return CONFERENCING_APP.googleMeet;
  }
  return hasLink ? CONFERENCING_APP.customLink : null;
}
