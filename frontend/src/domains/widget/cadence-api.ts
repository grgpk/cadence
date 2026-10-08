import { createServerFn } from "@tanstack/react-start";

import type { Booking } from "../../bindings/Booking";
import type { BookingPage } from "../../bindings/BookingPage";
import type { CallingVisit } from "../../bindings/CallingVisit";
import type { CallingVisitRequest } from "../../bindings/CallingVisitRequest";
import type { CalendarEvent } from "../../bindings/CalendarEvent";
import type { CreateBookingRequest } from "../../bindings/CreateBookingRequest";
import type { HostSettings } from "../../bindings/HostSettings";
import type { Lead } from "../../bindings/Lead";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { api } from "../../lib/api";
import { SERVER_API_URL } from "../../lib/api-url";

const NOT_FOUND = 404;
const HOST_SETTINGS_PATH = "/api/host/settings";

function hostPath(slug: string): string {
  return `/api/hosts/${encodeURIComponent(slug)}`;
}

export function saveLead(
  slug: string,
  leadUnid: string,
  update: LeadUpdate,
): Promise<Lead> {
  return api<Lead>(`${hostPath(slug)}/leads/${leadUnid}`, {
    method: "POST",
    body: JSON.stringify(update),
  });
}

export function submitLead(
  slug: string,
  leadUnid: string,
  sourcePage: string,
): Promise<Lead> {
  return api<Lead>(`${hostPath(slug)}/leads/${leadUnid}/submit`, {
    method: "POST",
    body: JSON.stringify({ source_page: sourcePage }),
  });
}

export function createCallingVisit(request: CallingVisitRequest): Promise<CallingVisit> {
  return api<CallingVisit>("/api/calling-visits", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export function updateCallingVisit(
  unid: string,
  update: Omit<
    CallingVisitRequest,
    | "source_page"
    | "referer"
    | "utm_source"
    | "utm_medium"
    | "utm_campaign"
    | "user_agent"
  > & {
    duration_seconds?: number;
    form_started?: boolean;
    form_submitted?: boolean;
    booked_call?: boolean;
  },
): Promise<void> {
  return api<void>("/api/calling-visits/engagement", {
    method: "POST",
    body: JSON.stringify({ unid, ...update }),
  });
}

export function getCalendarEvents(): Promise<CalendarEvent[]> {
  return api<CalendarEvent[]>("/api/google-calendar/events");
}

export async function connectGoogleCalendar(): Promise<void> {
  const config = await api<{ client_id: string; redirect_uri: string }>(
    "/api/google-calendar/oauth-config",
  );
  const scope = "https://www.googleapis.com/auth/calendar.events";
  window.location.href =
    `https://accounts.google.com/o/oauth2/v2/auth?client_id=${encodeURIComponent(config.client_id)}` +
    `&redirect_uri=${encodeURIComponent(config.redirect_uri)}` +
    `&response_type=code&scope=${encodeURIComponent(scope)}&access_type=offline&prompt=consent`;
}

export function submitGoogleCalendarCode(code: string): Promise<void> {
  return api<void>("/api/google-calendar/login", {
    method: "POST",
    body: JSON.stringify({ code }),
  });
}

export function disconnectGoogleCalendar(): Promise<void> {
  return api<void>("/api/google-calendar/disconnect", {
    method: "POST",
    body: JSON.stringify({}),
  });
}

/** A host's booking page, or `null` when the link is unknown or switched off. */
export const getBookingPage = createServerFn({ method: "GET" })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const response = await fetch(
      `${SERVER_API_URL}/api/hosts/${encodeURIComponent(slug)}`,
    );
    if (response.status === NOT_FOUND) return null;
    if (!response.ok) throw new Error("The booking page did not load.");
    return response.json() as Promise<BookingPage>;
  });

/** Every time the host can still be booked in the coming week. */
export function getSlots(slug: string): Promise<TimeSlot[]> {
  return api<TimeSlot[]>(`${hostPath(slug)}/slots`);
}

export function createBooking(
  slug: string,
  request: CreateBookingRequest,
): Promise<Booking> {
  return api<Booking>(`${hostPath(slug)}/bookings`, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export function getHostSettings(): Promise<HostSettings> {
  return api<HostSettings>(HOST_SETTINGS_PATH);
}

export function saveHostSettings(settings: HostSettings): Promise<HostSettings> {
  return api<HostSettings>(HOST_SETTINGS_PATH, {
    method: "PUT",
    body: JSON.stringify(settings),
  });
}
