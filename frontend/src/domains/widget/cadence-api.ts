import type { CallingVisit } from "../../bindings/CallingVisit";
import type { CallingVisitRequest } from "../../bindings/CallingVisitRequest";
import type { CalendarEvent } from "../../bindings/CalendarEvent";
import type { GoogleTimeSlot } from "../../bindings/GoogleTimeSlot";
import type { Lead } from "../../bindings/Lead";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import { api } from "../../lib/api";

export function saveLead(leadUnid: string, update: LeadUpdate): Promise<Lead> {
  return api<Lead>(`/api/leads/${leadUnid}`, {
    method: "POST",
    body: JSON.stringify(update),
  });
}

export function getLead(leadUnid: string): Promise<Lead | null> {
  return api<Lead | null>(`/api/leads/${leadUnid}`);
}

export function submitLead(leadUnid: string, sourcePage: string): Promise<Lead> {
  return api<Lead>(`/api/leads/${leadUnid}/submit`, {
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

export function getAvailableSlots(
  date: string,
  timezone: string,
): Promise<GoogleTimeSlot[]> {
  return api<GoogleTimeSlot[]>("/api/google-calendar/available-slots", {
    method: "POST",
    body: JSON.stringify({ date, timezone }),
  });
}

type BookSlotRequest = {
  date: string;
  time: string;
  timezone: string;
  attendee_email: string;
  attendee_name: string;
  lead_id: string;
};

export function createBooking(request: BookSlotRequest): Promise<unknown> {
  return api("/api/google-calendar/book", {
    method: "POST",
    body: JSON.stringify(request),
  });
}
