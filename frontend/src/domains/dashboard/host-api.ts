import { queryOptions } from "@tanstack/react-query";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { AvailabilityRuleInput } from "../../bindings/AvailabilityRuleInput";
import type { CalendarEvent } from "../../bindings/CalendarEvent";
import type { GoogleCalendarLoginRequest } from "../../bindings/GoogleCalendarLoginRequest";
import type { GoogleOAuthConfigResponse } from "../../bindings/GoogleOAuthConfigResponse";
import type { HostSettings } from "../../bindings/HostSettings";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

const HOST_SETTINGS_PATH = "/api/host/settings";
const AVAILABILITY_PATH = "/api/availability";

export const hostSettingsQuery = queryOptions({
  queryKey: QUERY_KEYS.hostSettings,
  queryFn: () => api<HostSettings>(HOST_SETTINGS_PATH),
});

export function saveHostSettings(settings: HostSettings): Promise<HostSettings> {
  return api<HostSettings>(HOST_SETTINGS_PATH, {
    method: "PUT",
    body: JSON.stringify(settings),
  });
}

export const availabilityQuery = queryOptions({
  queryKey: QUERY_KEYS.availability,
  queryFn: () => api<AvailabilityRule[]>(AVAILABILITY_PATH),
});

export function createAvailabilityRule(
  input: AvailabilityRuleInput,
): Promise<AvailabilityRule> {
  return api<AvailabilityRule>(AVAILABILITY_PATH, {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function updateAvailabilityRule(
  id: number,
  input: AvailabilityRuleInput,
): Promise<AvailabilityRule> {
  return api<AvailabilityRule>(`${AVAILABILITY_PATH}/${id}`, {
    method: "PUT",
    body: JSON.stringify(input),
  });
}

export function removeAvailabilityRule(rule: AvailabilityRule): Promise<void> {
  return api<void>(`${AVAILABILITY_PATH}/${rule.id}`, { method: "DELETE" });
}

/** Fails when no calendar is connected, so it is asked once and not again. */
export const calendarEventsQuery = queryOptions({
  queryKey: QUERY_KEYS.calendarEvents,
  queryFn: () => api<CalendarEvent[]>("/api/google-calendar/events"),
  retry: false,
});

export async function connectGoogleCalendar(): Promise<void> {
  const config = await api<GoogleOAuthConfigResponse>(
    "/api/google-calendar/oauth-config",
  );
  const scope = "https://www.googleapis.com/auth/calendar.events";
  window.location.href =
    `https://accounts.google.com/o/oauth2/v2/auth?client_id=${encodeURIComponent(config.client_id)}` +
    `&redirect_uri=${encodeURIComponent(config.redirect_uri)}` +
    `&response_type=code&scope=${encodeURIComponent(scope)}&access_type=offline&prompt=consent` +
    // Google hands this back with the code, proving the sign-in started here.
    `&state=${encodeURIComponent(config.state)}`;
}

export function submitGoogleCalendarCode(
  request: GoogleCalendarLoginRequest,
): Promise<void> {
  return api<void>("/api/google-calendar/login", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export function disconnectGoogleCalendar(): Promise<void> {
  return api<void>("/api/google-calendar/disconnect", {
    method: "POST",
    body: JSON.stringify({}),
  });
}
