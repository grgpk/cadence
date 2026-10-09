import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Skeleton } from "../../components/ui/skeleton";
import { ApiError } from "../../lib/api";
import { prefersHour12 } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";
import {
  DEFAULT_TIMEZONE,
  defaultTimezoneForCountry,
  detectBrowserCountryTimezone,
  getCountryByAlpha2,
  type CountryTimezone,
} from "../shared/phone/country";
import { EMPTY_FORM, leadUpdate, queryValue, type FormValues } from "./booking-form";
import {
  createBooking,
  createCallingVisit,
  saveLead,
  submitLead,
  updateCallingVisit,
} from "./cadence-api";
import { BookingSummary } from "./BookingSummary";
import { DetailsStep } from "./DetailsStep";
import { SlotStep, type CalendarView } from "./SlotStep";
import { TimezoneCombobox } from "./TimezoneCombobox";
import { zonedTime } from "./zoned-time";

const HOUR12_KEY = "cadence-hour12";

function initialHour12(): boolean {
  try {
    const stored = localStorage.getItem(HOUR12_KEY);
    if (stored !== null) return stored === "true";
  } catch {
    // Storage can be blocked; fall through to the locale's own convention.
  }
  return prefersHour12();
}

type StoredForm = { leadUnid: string; values: FormValues };

function formKey(slug: string): string {
  return `cadence-booking-${slug}`;
}

/** The form as it was before a refresh, for this tab and this booking page only. */
function storedForm(slug: string): StoredForm | null {
  try {
    const stored = sessionStorage.getItem(formKey(slug));
    return stored ? (JSON.parse(stored) as StoredForm) : null;
  } catch {
    return null;
  }
}

function storeForm(slug: string, form: StoredForm | null) {
  try {
    if (form) sessionStorage.setItem(formKey(slug), JSON.stringify(form));
    else sessionStorage.removeItem(formKey(slug));
  } catch {
    // Storage can be blocked; the form still works for this page load.
  }
}

function ignore() {
  // Visit tracking and autosave must never interrupt a booking.
}

function Booking({
  slug,
  page,
  countryVariant,
}: {
  slug: string;
  page: BookingPage;
  countryVariant: string;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [stored] = useState(() => storedForm(slug));
  const [values, setValues] = useState<FormValues>({
    ...EMPTY_FORM,
    countryVariant,
    ...stored?.values,
  });
  const [timezone, setTimezone] = useState<CountryTimezone>(
    () =>
      detectBrowserCountryTimezone() ??
      defaultTimezoneForCountry(countryVariant) ??
      DEFAULT_TIMEZONE,
  );
  const [view, setView] = useState<CalendarView>(() => {
    const today = zonedTime(new Date(), timezone.iana);
    return { year: today.year, month: today.month, day: null };
  });
  const [slot, setSlot] = useState<TimeSlot | null>(null);
  const [hour12, setHour12] = useState(initialHour12);
  const [leadUnid] = useState(() => stored?.leadUnid ?? crypto.randomUUID());
  const [visitUnid, setVisitUnid] = useState<string | null>(null);
  const leadSubmitted = useRef(false);
  // Set once the visitor moves between steps, so the first render takes no focus.
  const stepChanged = useRef(false);
  const focusStep = useCallback((step: HTMLElement | null) => {
    if (step && stepChanged.current) step.focus();
  }, []);

  function changeValues(next: FormValues) {
    setValues(next);
    storeForm(slug, { leadUnid, values: next });
  }

  function showStep(next: TimeSlot | null) {
    stepChanged.current = true;
    setSlot(next);
  }

  useEffect(() => {
    void createCallingVisit({
      source_page: window.location.pathname,
      referer: document.referrer || null,
      utm_source: queryValue("utm_source"),
      utm_medium: queryValue("utm_medium"),
      utm_campaign: queryValue("utm_campaign"),
      user_agent: navigator.userAgent,
    })
      .then((visit) => setVisitUnid(visit.unid))
      .catch(ignore);
  }, []);

  function changeHour12(next: boolean) {
    setHour12(next);
    try {
      localStorage.setItem(HOUR12_KEY, String(next));
    } catch {
      // The choice still applies for this visit.
    }
  }

  const book = useMutation({
    mutationFn: async (picked: TimeSlot) => {
      await saveLead(slug, leadUnid, leadUpdate(values));
      if (!leadSubmitted.current) {
        await submitLead(slug, leadUnid, window.location.pathname);
        leadSubmitted.current = true;
        if (visitUnid) {
          void updateCallingVisit(visitUnid, { form_submitted: true }).catch(ignore);
        }
      }
      const booking = await createBooking(slug, {
        lead_unid: leadUnid,
        calling_visit_unid: visitUnid,
        slot_start: picked.start,
        timezone: timezone.iana,
        invitee_name: `${values.firstName.trim()} ${values.lastName.trim()}`.trim(),
        invitee_email: values.email.trim(),
      });
      if (visitUnid) {
        void updateCallingVisit(visitUnid, { booked_call: true }).catch(ignore);
      }
      storeForm(slug, null);
      // Part of the request, so the button stays busy until the next page is up.
      await navigate({
        to: ROUTES.bookingConfirmation,
        search: {
          start: booking.slot_start,
          minutes: picked.minutes,
          tz: timezone.iana,
          h12: hour12,
          meet: booking.meeting_url ?? undefined,
          link: booking.meeting_url !== null,
          slug,
          host: page.host_name,
          manage: booking.manage_token,
        },
      });
    },
    // The time may have just been taken, so the cached times can no longer be trusted.
    onError: () =>
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.publicSlots(slug) }),
  });
  const submitError = !book.error
    ? null
    : book.error instanceof ApiError && book.error.isConflict
      ? "That time was just taken. Go back and pick another one."
      : "The booking did not go through. Try again or pick another time.";

  // Each step slides in from the side it lives on: details to the right of times.
  return slot === null ? (
    <section
      key="times"
      ref={focusStep}
      tabIndex={-1}
      aria-label="Pick a time"
      style={{ "--step-from": "-1rem" } as CSSProperties}
      className="outline-none motion-safe:animate-step-in"
    >
      <SlotStep
        slug={slug}
        timeZone={timezone.iana}
        emptyDescription={`${page.host_name} has no open times in the next ${page.booking_window_days} days. Check back later.`}
        summary={(callMinutes) => (
          <BookingSummary
            page={page}
            minutes={callMinutes}
            timezone={
              <TimezoneCombobox
                selected={timezone}
                hour12={hour12}
                onSelect={(next) => {
                  setTimezone(next);
                  setView({ ...view, day: null });
                }}
              />
            }
          />
        )}
        view={view}
        onViewChange={setView}
        hour12={hour12}
        onHour12Change={changeHour12}
        onPick={(picked) => {
          book.reset();
          showStep(picked);
        }}
      />
    </section>
  ) : (
    <section
      key="details"
      ref={focusStep}
      tabIndex={-1}
      aria-label="Your details"
      className="outline-none motion-safe:animate-step-in"
    >
      <DetailsStep
        page={page}
        slot={slot}
        timezone={timezone}
        hour12={hour12}
        values={values}
        onChange={changeValues}
        onSave={(next) => void saveLead(slug, leadUnid, leadUpdate(next)).catch(ignore)}
        onBack={() => showStep(null)}
        onSubmit={() => book.mutate(slot)}
        submitting={book.isPending}
        submitError={submitError}
      />
    </section>
  );
}

export function BookingCard({
  slug,
  page,
  initialCountryAlpha2,
}: {
  slug: string;
  page: BookingPage;
  initialCountryAlpha2?: string | null;
}) {
  // Timezone, today's date and the clock format only exist in the browser, so the
  // card renders after mount instead of guessing them on the server.
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  const countryVariant =
    (initialCountryAlpha2 ? getCountryByAlpha2(initialCountryAlpha2)?.variant : null) ??
    DEFAULT_TIMEZONE.variant;

  return (
    <div className="@container w-full">
      <div className="overflow-hidden rounded-2xl border bg-card shadow-[0_1px_2px_rgb(0_0_0/0.04),0_12px_32px_-12px_rgb(0_0_0/0.14)]">
        {mounted ? (
          <Booking slug={slug} page={page} countryVariant={countryVariant} />
        ) : (
          // The same three columns as the times step, so the card keeps its shape.
          <div
            role="status"
            aria-label="Loading booking"
            className="grid divide-y @[760px]:grid-cols-[15rem_minmax(0,1fr)_14rem] @[760px]:divide-x @[760px]:divide-y-0"
          >
            <div className="grid content-start gap-3 p-5 @[760px]:p-6">
              <Skeleton className="h-4 w-24" />
              <Skeleton className="h-7 w-40" />
              <Skeleton className="h-4 w-32" />
            </div>
            <div className="grid gap-4 p-5 @[760px]:p-6">
              <Skeleton className="h-7 w-32" />
              <Skeleton className="aspect-[7/5] w-full" />
            </div>
            <div className="grid content-start gap-2 p-5 @[760px]:p-6">
              <Skeleton className="h-11 w-full" />
              <Skeleton className="h-11 w-full" />
              <Skeleton className="h-11 w-full" />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
