import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { Skeleton } from "../../components/ui/skeleton";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";
import {
  ALL_COUNTRY_TIMEZONES,
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
import { DetailsStep } from "./DetailsStep";
import { SlotStep, type CalendarView, type PickedSlot } from "./SlotStep";
import { currentDateInTz } from "./slot-availability-checker";

const DEFAULT_TIMEZONE = "Europe/Paris";
const DEFAULT_TZ: CountryTimezone =
  ALL_COUNTRY_TIMEZONES.find((timezone) => timezone.iana === DEFAULT_TIMEZONE) ??
  ALL_COUNTRY_TIMEZONES[0];
const HOUR12_KEY = "cadence-hour12";

function initialHour12(): boolean {
  try {
    const stored = localStorage.getItem(HOUR12_KEY);
    if (stored !== null) return stored === "true";
  } catch {
    // Storage can be blocked; fall through to the locale's own convention.
  }
  return (
    new Intl.DateTimeFormat(undefined, { hour: "numeric" }).resolvedOptions().hour12 ??
    false
  );
}

function ignore() {
  // Visit tracking and autosave must never interrupt a booking.
}

function Booking({ countryVariant }: { countryVariant: string }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [values, setValues] = useState<FormValues>({ ...EMPTY_FORM, countryVariant });
  const [timezone, setTimezone] = useState<CountryTimezone>(
    () =>
      detectBrowserCountryTimezone() ??
      defaultTimezoneForCountry(countryVariant) ??
      DEFAULT_TZ,
  );
  const [view, setView] = useState<CalendarView>(() => {
    const today = currentDateInTz(timezone.iana);
    return { year: today.year, month: today.month, day: null };
  });
  const [slot, setSlot] = useState<PickedSlot | null>(null);
  const [hour12, setHour12] = useState(initialHour12);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [leadUnid] = useState(() => crypto.randomUUID());
  const [visitUnid, setVisitUnid] = useState<string | null>(null);
  const leadSubmitted = useRef(false);

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

  async function confirm(picked: PickedSlot) {
    setSubmitting(true);
    setSubmitError(null);
    try {
      await saveLead(leadUnid, leadUpdate(values));
      if (!leadSubmitted.current) {
        await submitLead(leadUnid, window.location.pathname);
        leadSubmitted.current = true;
        if (visitUnid) {
          void updateCallingVisit(visitUnid, { form_submitted: true }).catch(ignore);
        }
      }
      const result = await createBooking({
        date: picked.date,
        time: picked.time,
        timezone: timezone.iana,
        attendee_email: values.email.trim(),
        attendee_name: `${values.firstName.trim()} ${values.lastName.trim()}`.trim(),
        lead_id: leadUnid,
      });
      if (visitUnid) {
        void updateCallingVisit(visitUnid, { booked_call: true }).catch(ignore);
      }
      await navigate({
        to: ROUTES.bookingConfirmation,
        search: {
          date: picked.date,
          time: picked.time,
          tz: timezone.iana,
          meet: result.meet_link ?? undefined,
        },
      });
    } catch (error) {
      // The time may have just been taken, so the cached times can no longer be trusted.
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.publicSlotsAll });
      setSubmitError(
        error instanceof Error
          ? error.message
          : "The booking did not go through. Try again or pick another time.",
      );
    } finally {
      setSubmitting(false);
    }
  }

  return slot === null ? (
    <SlotStep
      timezone={timezone}
      onTimezoneChange={setTimezone}
      view={view}
      onViewChange={setView}
      hour12={hour12}
      onHour12Change={changeHour12}
      onPick={(picked) => {
        setSubmitError(null);
        setSlot(picked);
      }}
    />
  ) : (
    <DetailsStep
      slot={slot}
      timezone={timezone}
      hour12={hour12}
      values={values}
      onChange={setValues}
      onSave={(next) => void saveLead(leadUnid, leadUpdate(next)).catch(ignore)}
      onBack={() => setSlot(null)}
      onSubmit={() => void confirm(slot)}
      submitting={submitting}
      submitError={submitError}
    />
  );
}

export function BookingCard({
  initialCountryAlpha2,
}: {
  initialCountryAlpha2?: string | null;
}) {
  // Timezone, today's date and the clock format only exist in the browser, so the
  // card renders after mount instead of guessing them on the server.
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  const countryVariant =
    getCountryByAlpha2(initialCountryAlpha2 ?? "FR")?.variant ??
    EMPTY_FORM.countryVariant;

  return (
    <div id="booking-card" className="@container w-full scroll-mt-24">
      <div className="overflow-hidden rounded-2xl border bg-card shadow-[0_1px_2px_rgb(0_0_0/0.04),0_12px_32px_-12px_rgb(0_0_0/0.14)]">
        {mounted ? (
          <Booking countryVariant={countryVariant} />
        ) : (
          <div className="grid gap-4 p-6" role="status" aria-label="Loading booking">
            <Skeleton className="h-6 w-40" />
            <Skeleton className="h-72 w-full" />
          </div>
        )}
      </div>
    </div>
  );
}
