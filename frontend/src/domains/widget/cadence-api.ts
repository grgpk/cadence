import { type QueryClient, queryOptions } from "@tanstack/react-query";
import { createServerFn } from "@tanstack/react-start";

import type { BookingPage } from "../../bindings/BookingPage";
import type { CallingVisit } from "../../bindings/CallingVisit";
import type { CallingVisitRequest } from "../../bindings/CallingVisitRequest";
import type { CreateBookingRequest } from "../../bindings/CreateBookingRequest";
import type { CreatedBooking } from "../../bindings/CreatedBooking";
import type { ManagedBooking } from "../../bindings/ManagedBooking";
import type { Lead } from "../../bindings/Lead";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { api } from "../../lib/api";
import { SERVER_API_URL } from "../../lib/api-url";
import { QUERY_KEYS } from "../../lib/query-keys";

const NOT_FOUND = 404;

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

/** A host's booking page, or `null` when the link is unknown or switched off. */
export const getBookingPage = createServerFn({ method: "GET" })
  .validator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const response = await fetch(
      `${SERVER_API_URL}/api/hosts/${encodeURIComponent(slug)}`,
    );
    if (response.status === NOT_FOUND) return null;
    if (!response.ok) throw new Error("The booking page did not load.");
    return response.json() as Promise<BookingPage>;
  });

export function bookingPageQuery(slug: string) {
  return queryOptions({
    queryKey: QUERY_KEYS.bookingPage(slug),
    queryFn: () => getBookingPage({ data: slug }),
  });
}

/** Every time the host can still be booked inside their booking window. */
export function publicSlotsQuery(slug: string) {
  return queryOptions({
    queryKey: QUERY_KEYS.publicSlots(slug),
    queryFn: () => api<TimeSlot[]>(`${hostPath(slug)}/slots`),
  });
}

/**
 * Reads a booking page's open times again: after a booking, a move or a
 * cancel, or when one did not go through and the cached times are stale.
 */
export function refreshOpenTimes(queryClient: QueryClient, slug: string): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: QUERY_KEYS.publicSlots(slug) });
}

export function createBooking(
  slug: string,
  request: CreateBookingRequest,
): Promise<CreatedBooking> {
  return api<CreatedBooking>(`${hostPath(slug)}/bookings`, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

const managePath = (token: string) => `/api/manage/${encodeURIComponent(token)}`;

/** A call as the person who booked it sees it, found by their private link. */
export const managedBookingQuery = (token: string) =>
  queryOptions({
    queryKey: QUERY_KEYS.managedBooking(token),
    queryFn: () => api<ManagedBooking>(managePath(token)),
  });

export function cancelManagedBooking(token: string): Promise<ManagedBooking> {
  return api<ManagedBooking>(`${managePath(token)}/cancel`, { method: "POST" });
}

export function rescheduleManagedBooking(
  token: string,
  slotStart: string,
): Promise<ManagedBooking> {
  return api<ManagedBooking>(`${managePath(token)}/reschedule`, {
    method: "POST",
    body: JSON.stringify({ slot_start: slotStart }),
  });
}
