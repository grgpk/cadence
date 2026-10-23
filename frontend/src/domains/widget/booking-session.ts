import type { CallingVisit } from "../../bindings/CallingVisit";
import type { CallingVisitRequest } from "../../bindings/CallingVisitRequest";
import type { CreateBookingRequest } from "../../bindings/CreateBookingRequest";
import type { CreatedBooking } from "../../bindings/CreatedBooking";
import type { Lead } from "../../bindings/Lead";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { ApiError } from "../../lib/api";
import { type Arrival, type FormValues, leadUpdate, queryValue } from "./booking-form";
import {
  createBooking,
  createCallingVisit,
  saveLead,
  submitLead,
  updateCallingVisit,
  type PageRef,
} from "./cadence-api";

/** The API calls a booking session makes; tests pass a recorder instead. */
export type BookingApi = {
  createCallingVisit: (request: CallingVisitRequest) => Promise<CallingVisit>;
  updateCallingVisit: typeof updateCallingVisit;
  saveLead: (pageRef: PageRef, leadUnid: string, update: LeadUpdate) => Promise<Lead>;
  submitLead: (pageRef: PageRef, leadUnid: string) => Promise<Lead>;
  createBooking: (
    pageRef: PageRef,
    request: CreateBookingRequest,
  ) => Promise<CreatedBooking>;
};

export const CADENCE_API: BookingApi = {
  createCallingVisit,
  updateCallingVisit,
  saveLead,
  submitLead,
  createBooking,
};

/**
 * What the confirmation page reads from its address. The booking page writes
 * it and the confirmation route checks it, so both use this one type.
 */
export type ConfirmationSearch = {
  start?: string;
  minutes?: number;
  tz?: string;
  /** The clock the visitor chose: the server cannot know the device's own. */
  h12: boolean;
  /** A Google Meet link, the only kind the page shows as a button. */
  meet?: string;
  /** Whether the call has a place to meet at all. */
  link: boolean;
  /** The host's link and the kind of call, for booking another time. */
  slug?: string;
  type?: string;
  host?: string;
  /** The kind of call, as the host names it. */
  title?: string;
  /** The private link to change or cancel the call. */
  manage?: string;
};

/** What the visitor chose to book, and how they read times. */
export type BookingChoice = {
  slot: TimeSlot;
  values: FormValues;
  /** The visitor's IANA timezone. */
  timezone: string;
  hour12: boolean;
  hostName: string;
  /** The kind of call, as the host names it. */
  title: string;
};

/**
 * One visitor's visit to a booking page, from landing to a booked call.
 *
 * The visit record and every update to it are optional: none of them may
 * stop a booking. Autosaving never fails. The lead is submitted once per
 * session, however many tries the booking takes.
 */
type BookingSession = {
  readonly leadUnid: string;
  /** Records the visit. */
  start: () => Promise<void>;
  /** Keeps the answers so far. */
  saveDraft: (values: FormValues) => Promise<void>;
  /** Books the call. Throws what the API threw; see `bookingFailure`. */
  book: (choice: BookingChoice) => Promise<ConfirmationSearch>;
};

export function createBookingSession(
  api: BookingApi,
  {
    pageRef,
    leadUnid,
    arrival,
  }: { pageRef: PageRef; leadUnid: string; arrival: Arrival },
): BookingSession {
  let visitUnid: string | null = null;
  let leadSubmitted = false;

  /** A visit update that runs on its own; a failure goes nowhere. */
  const noteVisit = (update: Parameters<BookingApi["updateCallingVisit"]>[1]) => {
    if (!visitUnid) return;
    api.updateCallingVisit(visitUnid, update).catch(() => {
      // Visit tracking must never interrupt a booking.
    });
  };

  return {
    leadUnid,

    async start() {
      try {
        const visit = await api.createCallingVisit({
          source_page: arrival.path,
          referer: arrival.referrer || null,
          utm_source: queryValue(arrival.search, "utm_source"),
          utm_medium: queryValue(arrival.search, "utm_medium"),
          utm_campaign: queryValue(arrival.search, "utm_campaign"),
          user_agent: arrival.userAgent,
        });
        visitUnid = visit.unid;
      } catch {
        // Without a visit record the visitor can still book.
      }
    },

    async saveDraft(values) {
      try {
        await api.saveLead(pageRef, leadUnid, leadUpdate(values, visitUnid));
      } catch {
        // The answers are saved again when the visitor books.
      }
    },

    async book({ slot, values, timezone, hour12, hostName, title }) {
      await api.saveLead(pageRef, leadUnid, leadUpdate(values, visitUnid));
      if (!leadSubmitted) {
        await api.submitLead(pageRef, leadUnid);
        leadSubmitted = true;
        noteVisit({ form_submitted: true });
      }
      const booking = await api.createBooking(pageRef, {
        lead_unid: leadUnid,
        slot_start: slot.start,
        timezone,
        invitee_name: `${values.firstName.trim()} ${values.lastName.trim()}`.trim(),
        invitee_email: values.email.trim(),
      });
      noteVisit({ booked_call: true });
      return {
        start: booking.slot_start,
        minutes: slot.minutes,
        tz: timezone,
        h12: hour12,
        meet: booking.meeting_url ?? undefined,
        link: booking.meeting_url !== null,
        slug: pageRef.host,
        type: pageRef.type,
        host: hostName,
        title,
        manage: booking.manage_token,
      };
    },
  };
}

/** Why a booking or a move did not go through: the time was taken, or anything else. */
export function bookingFailure(error: unknown): "taken" | "failed" {
  return error instanceof ApiError && error.isConflict ? "taken" : "failed";
}
