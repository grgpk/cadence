import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { CircleAlert } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Skeleton } from "../../components/ui/skeleton";
import { useClockPreference } from "../../hooks/use-clock-preference";
import { ROUTES } from "../../lib/routes";
import {
  DEFAULT_TIMEZONE,
  defaultTimezoneForCountry,
  detectBrowserCountryTimezone,
  getCountryByAlpha2,
  type CountryTimezone,
} from "../shared/phone/country";
import { arrivalHere, EMPTY_FORM, type FormValues } from "./booking-form";
import { bookingFailure, CADENCE_API, createBookingSession } from "./booking-session";
import { refreshOpenTimes, type PageRef } from "./cadence-api";
import { BookingSummary } from "./booking-summary";
import { DetailsStep } from "./details-step";
import type { SlotLayout } from "./slot-days";
import { SlotPicker } from "./slot-picker";
import { TimezoneCombobox } from "./timezone-combobox";
import { ViewSwitch } from "./view-switch";

type StoredForm = { leadUnid: string; values: FormValues };

function formKey(pageRef: PageRef): string {
  return `cadence-booking-${pageRef.host}/${pageRef.type}`;
}

/** The form as it was before a refresh, for this tab and this booking page only. */
function storedForm(pageRef: PageRef): StoredForm | null {
  try {
    const stored = sessionStorage.getItem(formKey(pageRef));
    return stored ? (JSON.parse(stored) as StoredForm) : null;
  } catch {
    return null;
  }
}

function storeForm(pageRef: PageRef, form: StoredForm | null) {
  try {
    if (form) sessionStorage.setItem(formKey(pageRef), JSON.stringify(form));
    else sessionStorage.removeItem(formKey(pageRef));
  } catch {
    // Storage can be blocked; the form still works for this page load.
  }
}

function Booking({
  pageRef,
  page,
  countryVariant,
  layout,
  onLayoutChange,
}: {
  pageRef: PageRef;
  page: BookingPage;
  countryVariant: string;
  layout: SlotLayout;
  onLayoutChange: (layout: SlotLayout) => void;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [stored] = useState(() => storedForm(pageRef));
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
  const [slot, setSlot] = useState<TimeSlot | null>(null);
  // The picked time went to someone else: back on the times, saying why.
  const [taken, setTaken] = useState(false);
  const [hour12, setHour12] = useClockPreference();
  // One visit to this page: its visit record, its lead and its booking.
  const [session] = useState(() =>
    createBookingSession(CADENCE_API, {
      pageRef,
      leadUnid: stored?.leadUnid ?? crypto.randomUUID(),
      arrival: arrivalHere(),
    }),
  );
  // Set once the visitor moves between steps, so the first render takes no focus.
  const stepChanged = useRef(false);
  const focusStep = useCallback((step: HTMLElement | null) => {
    if (step && stepChanged.current) step.focus();
  }, []);

  function changeValues(next: FormValues) {
    setValues(next);
    storeForm(pageRef, { leadUnid: session.leadUnid, values: next });
  }

  function showStep(next: TimeSlot | null) {
    stepChanged.current = true;
    setSlot(next);
  }

  useEffect(() => {
    void session.start();
  }, [session]);

  const book = useMutation({
    mutationFn: async (picked: TimeSlot) => {
      const search = await session.book({
        slot: picked,
        values,
        timezone: timezone.iana,
        hour12,
        hostName: page.host_name,
        title: page.title,
      });
      storeForm(pageRef, null);
      // Part of the request, so the button stays busy until the next page is up.
      await navigate({ to: ROUTES.bookingConfirmation, search });
    },
    // The time may have just been taken, so the cached times can no longer be trusted.
    onError: (error) => {
      void refreshOpenTimes(queryClient, pageRef);
      if (bookingFailure(error) === "taken") {
        // The form is kept: picking another time comes straight back to it.
        setTaken(true);
        showStep(null);
      }
    },
  });
  const submitError =
    !book.error || bookingFailure(book.error) === "taken"
      ? null
      : "The booking did not go through. Try again or pick another time.";

  const emptyDescription = `${page.host_name} has no open times in the next ${page.booking_window_days} days. Check back later.`;
  const pickTime = (picked: TimeSlot) => {
    book.reset();
    setTaken(false);
    showStep(picked);
  };
  const summary = (callMinutes: number | null) => (
    <BookingSummary
      page={page}
      minutes={callMinutes}
      timezone={
        <TimezoneCombobox selected={timezone} hour12={hour12} onSelect={setTimezone} />
      }
    >
      <ViewSwitch value={layout} onChange={onLayoutChange} />
    </BookingSummary>
  );

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
      {taken ? (
        <Alert className="rounded-none border-x-0 border-t-0 bg-warning/10">
          <CircleAlert className="size-4" />
          <AlertDescription className="font-medium text-foreground">
            That time was just taken. Here are the times still open.
          </AlertDescription>
        </Alert>
      ) : null}
      <SlotPicker
        pageRef={pageRef}
        timeZone={timezone.iana}
        layout={layout}
        hour12={hour12}
        onHour12Change={setHour12}
        onPick={pickTime}
        summary={summary}
        wording={{ emptyDescription }}
        windowDays={page.booking_window_days}
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
        onSave={(next) => void session.saveDraft(next)}
        onBack={() => showStep(null)}
        onSubmit={() => book.mutate(slot)}
        submitting={book.isPending}
        submitError={submitError}
      />
    </section>
  );
}

export function BookingCard({
  pageRef,
  page,
  initialCountryAlpha2,
  layout,
  onLayoutChange,
}: {
  pageRef: PageRef;
  page: BookingPage;
  initialCountryAlpha2?: string | null;
  /** How the open times are laid out: the month calendar, a week, or a list. */
  layout: SlotLayout;
  onLayoutChange: (layout: SlotLayout) => void;
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
          <Booking
            pageRef={pageRef}
            page={page}
            countryVariant={countryVariant}
            layout={layout}
            onLayoutChange={onLayoutChange}
          />
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
