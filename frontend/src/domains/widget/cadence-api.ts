import { type QueryClient, queryOptions } from "@tanstack/react-query";
import { createServerFn } from "@tanstack/react-start";

import type { BookingPage } from "../../bindings/BookingPage";
import type { HostPage } from "../../bindings/HostPage";
import type { CallingVisit } from "../../bindings/CallingVisit";
import type { CallingVisitRequest } from "../../bindings/CallingVisitRequest";
import type { CreateBookingRequest } from "../../bindings/CreateBookingRequest";
import type { CreatedBooking } from "../../bindings/CreatedBooking";
import type { ManagedBooking } from "../../bindings/ManagedBooking";
import type { Lead } from "../../bindings/Lead";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { api } from "../../lib/api";
import { CLIENT_API_URL, SERVER_API_URL } from "../../lib/api-url";
import { QUERY_KEYS } from "../../lib/query-keys";

const NOT_FOUND = 404;

/** One kind of call on a host's link: `/book/<host>/<type>`. */
export type PageRef = { host: string; type: string };

function hostPath(host: string): string {
  return `/api/hosts/${encodeURIComponent(host)}`;
}

/**
 * The meta tags that give a shared link the host's own picture: name,
 * colour and kinds of call. Chat apps fetch it from the public API.
 */
export function previewMeta(host: string) {
  const image = `${CLIENT_API_URL}${hostPath(host)}/preview.png`;
  return [
    { property: "og:image", content: image },
    { property: "og:image:width", content: "1200" },
    { property: "og:image:height", content: "630" },
    { name: "twitter:image", content: image },
  ];
}

function typePath({ host, type }: PageRef): string {
  return `${hostPath(host)}/types/${encodeURIComponent(type)}`;
}

export function saveLead(
  pageRef: PageRef,
  leadUnid: string,
  update: LeadUpdate,
): Promise<Lead> {
  return api<Lead>(`${typePath(pageRef)}/leads/${leadUnid}`, {
    method: "POST",
    body: JSON.stringify(update),
  });
}

export function submitLead(
  pageRef: PageRef,
  leadUnid: string,
  sourcePage: string,
): Promise<Lead> {
  return api<Lead>(`${typePath(pageRef)}/leads/${leadUnid}/submit`, {
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

async function readOrNull<T>(path: string, failure: string): Promise<T | null> {
  const response = await fetch(`${SERVER_API_URL}${path}`);
  if (response.status === NOT_FOUND) return null;
  if (!response.ok) throw new Error(failure);
  return response.json() as Promise<T>;
}

/** A host's link and the kinds of call it offers, or `null` when unknown. */
export const getHostPage = createServerFn({ method: "GET" })
  .validator((host: string) => host)
  .handler(({ data: host }) =>
    readOrNull<HostPage>(hostPath(host), "The booking page did not load."),
  );

export function hostPageQuery(host: string) {
  return queryOptions({
    queryKey: QUERY_KEYS.hostPage(host),
    queryFn: () => getHostPage({ data: host }),
  });
}

/** One kind of call on a host's link, or `null` when either is unknown. */
export const getBookingPage = createServerFn({ method: "GET" })
  .validator((pageRef: PageRef) => pageRef)
  .handler(({ data: pageRef }) =>
    readOrNull<BookingPage>(typePath(pageRef), "The booking page did not load."),
  );

/** Every time the host can still be booked for this kind of call. */
export function publicSlotsQuery(pageRef: PageRef) {
  return queryOptions({
    queryKey: QUERY_KEYS.publicSlots(pageRef.host, pageRef.type),
    queryFn: () => api<TimeSlot[]>(`${typePath(pageRef)}/slots`),
  });
}

/**
 * Reads a booking page's open times again: after a booking, a move or a
 * cancel, or when one did not go through and the cached times are stale.
 */
export function refreshOpenTimes(
  queryClient: QueryClient,
  pageRef: PageRef,
): Promise<void> {
  return queryClient.invalidateQueries({
    queryKey: QUERY_KEYS.publicSlots(pageRef.host, pageRef.type),
  });
}

export function createBooking(
  pageRef: PageRef,
  request: CreateBookingRequest,
): Promise<CreatedBooking> {
  return api<CreatedBooking>(`${typePath(pageRef)}/bookings`, {
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

/** The invitee says they are coming, from their reminder. */
export function attendManagedBooking(token: string): Promise<ManagedBooking> {
  return api<ManagedBooking>(`${managePath(token)}/attend`, { method: "POST" });
}

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
