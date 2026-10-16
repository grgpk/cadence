import { describe, expect, it } from "vitest";

import type { CreatedBooking } from "../../bindings/CreatedBooking";
import type { Lead } from "../../bindings/Lead";
import { ApiError } from "../../lib/api";
import { type Arrival, EMPTY_FORM, type FormValues } from "./booking-form";
import {
  type BookingApi,
  type BookingChoice,
  bookingFailure,
  createBookingSession,
} from "./booking-session";

const ARRIVAL: Arrival = {
  path: "/book/ana",
  search: "?utm_source=newsletter&utm_campaign=autumn",
  referrer: "https://example.com/blog",
  userAgent: "test-browser",
};

const VALUES: FormValues = {
  ...EMPTY_FORM,
  firstName: " Mia ",
  lastName: "Chen",
  email: " mia@example.com ",
};

const CHOICE: BookingChoice = {
  slot: { start: "2026-10-20T09:00:00Z", minutes: 30 },
  values: VALUES,
  timezone: "Europe/Paris",
  hour12: false,
  hostName: "Ana",
};

type Failing = Partial<Record<keyof BookingApi, unknown>>;

/** An API that writes down every call, in order, and fails the ones asked to. */
function recorder(
  failing: Failing = {},
  meetingUrl: string | null = "https://meet.google.com/abc",
) {
  const calls: string[] = [];
  const answer = <T>(name: keyof BookingApi, line: string, value: T): Promise<T> => {
    calls.push(line);
    return name in failing ? Promise.reject(failing[name]) : Promise.resolve(value);
  };
  const api: BookingApi = {
    createCallingVisit: (request) =>
      answer("createCallingVisit", `visit ${request.utm_source} ${request.referer}`, {
        unid: "visit-1",
        source_page: request.source_page,
        created_at: "",
      }),
    updateCallingVisit: (unid, update) =>
      answer(
        "updateCallingVisit",
        `visit ${unid} ${Object.keys(update).join(",")}`,
        undefined,
      ),
    saveLead: (pageRef, leadUnid, update) =>
      answer(
        "saveLead",
        `save ${pageRef.host}/${pageRef.type} ${leadUnid} ${update.first_name} ${update.utm_source}`,
        {
          unid: leadUnid,
        } as Lead,
      ),
    submitLead: (pageRef, leadUnid, sourcePage) =>
      answer(
        "submitLead",
        `submit ${pageRef.host}/${pageRef.type} ${leadUnid} ${sourcePage}`,
        {
          unid: leadUnid,
        } as Lead,
      ),
    createBooking: (pageRef, request) =>
      answer(
        "createBooking",
        `book ${pageRef.host}/${pageRef.type} ${request.slot_start} ${request.invitee_name} <${request.invitee_email}> visit=${request.calling_visit_unid}`,
        {
          unid: "call-1",
          slot_start: request.slot_start,
          meeting_url: meetingUrl,
          manage_token: "abc123",
        } as CreatedBooking,
      ),
  };
  return { api, calls };
}

const session = (api: BookingApi) =>
  createBookingSession(api, {
    pageRef: { host: "ana", type: "intro" },
    leadUnid: "lead-1",
    arrival: ARRIVAL,
  });

describe("a booking session", () => {
  it("records the visit with where the visitor came from", async () => {
    const { api, calls } = recorder();
    await session(api).start();
    expect(calls).toEqual(["visit newsletter https://example.com/blog"]);
  });

  it("books in order: answers, submit, booking, with the visit told along the way", async () => {
    const { api, calls } = recorder();
    const visit = session(api);
    await visit.start();
    await visit.book(CHOICE);
    expect(calls).toEqual([
      "visit newsletter https://example.com/blog",
      "save ana/intro lead-1 Mia newsletter",
      "submit ana/intro lead-1 /book/ana",
      "visit visit-1 form_submitted",
      "book ana/intro 2026-10-20T09:00:00Z Mia Chen <mia@example.com> visit=visit-1",
      "visit visit-1 booked_call",
    ]);
  });

  it("submits the lead once, however many tries the booking takes", async () => {
    const taken = new ApiError("this time is no longer available", 409);
    const first = recorder({ createBooking: taken });
    const visit = session(first.api);
    await expect(visit.book(CHOICE)).rejects.toBe(taken);
    expect(first.calls.filter((line) => line.startsWith("submit"))).toHaveLength(1);

    // The same session, a second try that goes through.
    first.api.createBooking = recorder().api.createBooking;
    await visit.book(CHOICE);
    expect(first.calls.filter((line) => line.startsWith("submit"))).toHaveLength(1);
  });

  it("books without a visit record when recording the visit failed", async () => {
    const { api, calls } = recorder({ createCallingVisit: new Error("offline") });
    const visit = session(api);
    await visit.start();
    await visit.book(CHOICE);
    expect(calls.some((line) => line.includes("visit=null"))).toBe(true);
    expect(calls.filter((line) => line.startsWith("visit visit-1"))).toEqual([]);
  });

  it("books even when telling the visit fails", async () => {
    const { api } = recorder({ updateCallingVisit: new Error("offline") });
    const visit = session(api);
    await visit.start();
    await expect(visit.book(CHOICE)).resolves.toMatchObject({ manage: "abc123" });
  });

  it("never fails while autosaving", async () => {
    const { api, calls } = recorder({ saveLead: new Error("offline") });
    await expect(session(api).saveDraft(VALUES)).resolves.toBeUndefined();
    expect(calls).toEqual(["save ana/intro lead-1 Mia newsletter"]);
  });

  it("gives the confirmation page everything it shows", async () => {
    const { api } = recorder();
    expect(await session(api).book(CHOICE)).toEqual({
      start: "2026-10-20T09:00:00Z",
      minutes: 30,
      tz: "Europe/Paris",
      h12: false,
      meet: "https://meet.google.com/abc",
      link: true,
      slug: "ana",
      type: "intro",
      host: "Ana",
      manage: "abc123",
    });
  });

  it("says when a call has no place to meet", async () => {
    const { api } = recorder({}, null);
    const search = await session(api).book(CHOICE);
    expect(search.meet).toBeUndefined();
    expect(search.link).toBe(false);
  });
});

describe("bookingFailure", () => {
  it("tells a taken time from anything else", () => {
    expect(bookingFailure(new ApiError("taken", 409))).toBe("taken");
    expect(bookingFailure(new ApiError("bad", 400))).toBe("failed");
    expect(bookingFailure(new Error("offline"))).toBe("failed");
  });
});
