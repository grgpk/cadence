import { ChevronLeft, ChevronRight, Check, Clock, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent } from "react";

import type { LeadUpdate } from "../../bindings/LeadUpdate";
import type { GoogleTimeSlot } from "../../bindings/GoogleTimeSlot";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import {
  getCountryByAlpha2,
  getCountryByVariant,
  ALL_COUNTRY_TIMEZONES,
  defaultTimezoneForCountry,
  detectBrowserCountryTimezone,
  type Country,
  type CountryTimezone,
} from "../shared/phone/country";
import {
  formatForCountry,
  hasRepeatedDigits,
  MIN_REPEATED_DIGITS,
} from "../shared/phone/phone-format";
import { PhoneInput } from "../shared/phone/PhoneInput";
import { calendarWeeks, isoDate, monthLabel, shiftMonth } from "./calendar-grid";
import { TimezoneCombobox } from "./TimezoneCombobox";
import {
  calculateAvailableDays,
  currentDateInTz,
  nextMonthHasDays,
} from "./slot-availability-checker";
import {
  createBooking,
  createCallingVisit,
  getAvailableSlots,
  getCalendarEvents,
  getLead,
  saveLead,
  submitLead,
  updateCallingVisit,
} from "./cadence-api";

const BRAND = "#f52326";
const DEFAULT_TIMEZONE = "Europe/Paris";
const DEFAULT_TZ: CountryTimezone =
  ALL_COUNTRY_TIMEZONES.find((timezone) => timezone.iana === DEFAULT_TIMEZONE) ??
  ALL_COUNTRY_TIMEZONES[0];
const BOOKING_DAYS_OF_WEEK = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
const HOW_HEARD_OPTIONS = ["Word of Mouth", "Google", "LinkedIn", "YouTube"];
const PREVIEW_DAYS_OF_WEEK = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const EXCLUDED_PREVIEW_WEEKDAYS = new Set([0, 6]);
const STOPPING_OPTIONS = [
  "Lack of time",
  "Lack of guidance/structure",
  "Don't know where to start",
  "Tried before and got stuck",
];
const INVESTMENT_OPTIONS = [
  "$100–500 (courses, books)",
  "$500–2,000 (serious training)",
  "$2,000–5,000 (career investment)",
  "$5,000+ (maximum support)",
];
const URGENCY_OPTIONS = [
  "1-3 (Nice to have)",
  "4-6 (Important but not urgent)",
  "7-8 (High priority)",
  "9-10 (Critical, need it now)",
];

type FormValues = {
  phone: string;
  countryVariant: string;
  firstName: string;
  lastName: string;
  email: string;
  howHeard: string[];
  currentlyWorkingOn: string;
  whatStoppingYou: string[];
  investmentComfort: string;
  urgencyLevel: string;
};

const EMPTY_FORM: FormValues = {
  phone: "",
  countryVariant: "France",
  firstName: "",
  lastName: "",
  email: "",
  howHeard: [],
  currentlyWorkingOn: "",
  whatStoppingYou: [],
  investmentComfort: "",
  urgencyLevel: "",
};

function leadUpdate(values: FormValues): LeadUpdate {
  const country = getCountryByVariant(values.countryVariant);
  return {
    first_name: values.firstName.trim() || null,
    last_name: values.lastName.trim() || null,
    email: values.email.trim() || null,
    phone: values.phone.trim() || null,
    country_code: country ? String(country.dialCode) : null,
    investment_comfort: values.investmentComfort || null,
    what_stopping_you: values.whatStoppingYou.join(", ") || null,
    how_heard_about_us: values.howHeard.join(", ") || null,
    currently_working_on: values.currentlyWorkingOn.trim() || null,
    urgency_level: values.urgencyLevel || null,
    source_page: typeof window === "undefined" ? null : window.location.pathname,
    utm_source: typeof window === "undefined" ? null : queryValue("utm_source"),
    utm_medium: typeof window === "undefined" ? null : queryValue("utm_medium"),
    utm_campaign: typeof window === "undefined" ? null : queryValue("utm_campaign"),
    qualification_status: qualification(values) as LeadUpdate["qualification_status"],
  };
}

function queryValue(key: string): string | null {
  return new URLSearchParams(window.location.search).get(key);
}

function qualification(values: FormValues): "Qualified" | "NotSure" | "Disqualified" {
  if (!values.investmentComfort) return "NotSure";
  return values.investmentComfort === INVESTMENT_OPTIONS[0] ? "NotSure" : "Qualified";
}

function validName(value: string): boolean {
  return value.trim().length >= 2 && !/\d/.test(value);
}

function validEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value.trim());
}

function toggleValue(values: string[], value: string): string[] {
  return values.includes(value)
    ? values.filter((item) => item !== value)
    : [...values, value];
}

function StepIndicator({
  active,
  done,
  label,
}: {
  active?: boolean;
  done?: boolean;
  label: string;
}) {
  return (
    <div className={`flex cursor-pointer items-center ${done ? "opacity-50" : ""}`}>
      {done ? (
        <Check className="size-4 text-slate-900" />
      ) : (
        <span
          className="size-2 rounded-full"
          style={{ background: active ? BRAND : "#d1d5db" }}
        />
      )}
      <span
        className={`pl-2 text-sm whitespace-nowrap text-slate-900 ${active || done ? "" : "opacity-50"}`}
      >
        {label}
      </span>
    </div>
  );
}

function OptionList({
  options,
  values,
  onChange,
}: {
  options: string[];
  values: string[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="mt-2 flex flex-col gap-2">
      {options.map((option) => (
        <label
          key={option}
          className="flex cursor-pointer items-center gap-2 text-sm text-slate-600"
        >
          <input
            type="checkbox"
            checked={values.includes(option)}
            onChange={() => onChange(option)}
            className="size-4 accent-[#f52326]"
          />
          {option}
        </label>
      ))}
    </div>
  );
}

function previewAvailableDays(now: Date, currentMonth: number): number[] {
  const days: number[] = [];
  let check = new Date(
    Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()),
  );

  while (days.length < 3) {
    const weekday = check.getUTCDay();
    if (
      !EXCLUDED_PREVIEW_WEEKDAYS.has(weekday) &&
      check.getUTCMonth() + 1 === currentMonth
    ) {
      days.push(check.getUTCDate());
    }
    check = new Date(
      Date.UTC(check.getUTCFullYear(), check.getUTCMonth(), check.getUTCDate() + 1),
    );
    if (check.getUTCMonth() + 1 !== currentMonth && days.length === 0) break;
  }

  return days;
}

function CalendarPreviewDay({
  day,
  isToday,
  available,
}: {
  day: number;
  isToday: boolean;
  available: boolean;
}) {
  return (
    <button
      type="button"
      disabled
      className="flex max-h-11 w-full max-w-11 flex-col items-center justify-center rounded-lg py-4 @[900px]:w-11"
      style={available ? { background: "rgba(245, 35, 38, 0.2)" } : undefined}
    >
      <span className="text-sm text-slate-900">{day}</span>
      {isToday && <span className="size-1.5 shrink-0 rounded-full bg-[#f52326]" />}
    </button>
  );
}

function CalendarPreview() {
  const now = new Date();
  const currentYear = now.getUTCFullYear();
  const currentMonth = now.getUTCMonth() + 1;
  const today = now.getUTCDate();
  const availableDays = previewAvailableDays(now, currentMonth);
  const weeks = calendarWeeks(currentYear, currentMonth);

  return (
    <div className="min-w-0 flex-1 border-slate-200 px-5 @[900px]:border-l">
      <div className="relative mx-auto w-full max-w-[412px] pb-4 @[900px]:mx-0 @[900px]:w-[450px] @[900px]:py-6">
        <div className="opacity-30 transition-opacity duration-300">
          <div className="mb-6">
            <div className="flex items-center justify-between">
              <span className="text-base font-medium text-slate-900">
                {monthLabel(currentYear, currentMonth)}
              </span>
              <div className="flex items-center gap-2">
                <CalendarButton direction="back" disabled />
                <CalendarButton direction="forward" disabled />
              </div>
            </div>

            <div className="overflow-x-auto pt-4 @[900px]:pt-3">
              <table className="w-full">
                <thead>
                  <tr className="flex">
                    {PREVIEW_DAYS_OF_WEEK.map((day) => (
                      <th key={day} className="flex-1">
                        <div className="flex w-full justify-center">
                          <span className="text-center text-xs font-semibold uppercase text-slate-500">
                            {day}
                          </span>
                        </div>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="space-y-1.5 @[900px]:space-y-2">
                  {weeks.map((week, weekIndex) => (
                    <tr
                      key={weekIndex}
                      className={`flex gap-1.5 @[900px]:gap-2 ${weekIndex === 0 ? "mt-2" : ""}`}
                    >
                      {week.map((day, dayIndex) => (
                        <td key={dayIndex} className="flex-1 p-0">
                          {day !== null && (
                            <CalendarPreviewDay
                              day={day}
                              isToday={day === today}
                              available={availableDays.includes(day)}
                            />
                          )}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
        <div className="absolute inset-0 flex items-center justify-center">
          <p className="max-w-[270px] rounded border border-slate-200 bg-white p-3 text-center text-sm leading-5 text-slate-900 shadow-md">
            Please fill out the form before choosing your time slot.
          </p>
        </div>
      </div>
    </div>
  );
}

function CalendarButton({
  direction,
  disabled,
  onClick,
}: {
  direction: "back" | "forward";
  disabled: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={`calendar ${direction}`}
      disabled={disabled}
      onClick={onClick}
      className="flex size-8 items-center justify-center rounded border border-slate-200 bg-white disabled:cursor-not-allowed disabled:opacity-50"
    >
      {direction === "back" ? (
        <ChevronLeft className="size-4" />
      ) : (
        <ChevronRight className="size-4" />
      )}
    </button>
  );
}

function BookingCalendarDay({
  day,
  isToday,
  available,
  isSelected,
  onSelect,
}: {
  day: number;
  isToday: boolean;
  available: boolean;
  isSelected: boolean;
  onSelect: () => void;
}) {
  const style = isSelected
    ? { borderColor: BRAND, background: BRAND, color: "white" }
    : available
      ? { borderColor: "transparent", background: "rgba(245, 35, 38, 0.3)" }
      : { borderColor: "transparent" };

  return (
    <button
      type="button"
      onClick={onSelect}
      disabled={!available}
      className={`flex max-h-11 w-full max-w-11 flex-col items-center justify-center rounded-lg border-2 py-4 font-medium @[900px]:w-11 ${available ? "cursor-pointer" : ""}`}
      style={style}
    >
      <span className="text-sm">{day}</span>
      {isToday && (
        <span
          className="size-1.5 shrink-0 rounded-full"
          style={{ background: isSelected ? "white" : BRAND }}
        />
      )}
    </button>
  );
}

function GoogleMeetIcon() {
  return (
    <svg
      width="22"
      height="22"
      viewBox="0 0 22 22"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path d="M5.5 7.33341L5.5 14.6667H12.8333V7.33341L5.5 7.33341Z" fill="none" />
      <path
        d="M1.375 7.79159V14.2083L3.66667 14.6666L5.95833 14.2083V7.79159L3.66667 7.33325L1.375 7.79159Z"
        fill="#1E88E5"
      />
      <path
        d="M16.9583 11.0001V17.4167C16.9583 18.1762 16.3428 18.7917 15.5833 18.7917H5.95833L5.5 16.5001L5.95833 14.2084H12.375V11.0001L14.6667 10.5417L16.9583 11.0001Z"
        fill="#4CAF50"
      />
      <path
        d="M16.9583 4.58325V10.9999H12.375V7.79158H5.95833L5.5 5.49992L5.95833 3.20825H15.5833C16.3428 3.20825 16.9583 3.82379 16.9583 4.58325Z"
        fill="#FBC02D"
      />
      <path
        d="M5.95833 14.2083V18.7916H2.75C1.99054 18.7916 1.375 18.176 1.375 17.4166V14.2083H5.95833Z"
        fill="#1565C0"
      />
      <path d="M5.95833 3.20825V7.79159H1.375L5.95833 3.20825Z" fill="#E53935" />
      <path
        d="M17.4167 11.0001L16.9583 14.873L12.375 11.0001L16.9583 7.1272L17.4167 11.0001Z"
        fill="#2E7D32"
      />
      <path
        d="M21.082 4.63383V17.3663C21.082 17.7513 20.6329 17.9667 20.3349 17.7238L16.957 14.873V7.12716L20.3349 4.27633C20.6329 4.03341 21.082 4.24883 21.082 4.63383Z"
        fill="#4CAF50"
      />
    </svg>
  );
}

function StepOne({
  leadUnid,
  values,
  setValues,
  onSubmit,
  submitting,
}: {
  leadUnid: string;
  values: FormValues;
  setValues: (next: FormValues) => void;
  onSubmit: () => void;
  submitting: boolean;
}) {
  const [attempted, setAttempted] = useState(false);
  const country = getCountryByVariant(values.countryVariant) ?? getCountryByAlpha2("FR");
  const phoneFormat = country ? formatForCountry(country.variant) : null;
  const phoneValid =
    country !== undefined &&
    phoneFormat !== null &&
    values.phone.length === phoneFormat.maxDigits &&
    !hasRepeatedDigits(values.phone, MIN_REPEATED_DIGITS);
  const valid =
    validName(values.firstName) &&
    validName(values.lastName) &&
    validEmail(values.email) &&
    phoneValid &&
    values.howHeard.length > 0 &&
    values.whatStoppingYou.length > 0 &&
    Boolean(values.investmentComfort) &&
    Boolean(values.urgencyLevel);
  const isFirstSectionValid =
    validName(values.firstName) && validName(values.lastName) && phoneValid;
  const update = (patch: Partial<FormValues>) => setValues({ ...values, ...patch });
  const save = (next: FormValues) =>
    void saveLead(leadUnid, leadUpdate(next)).catch(() => undefined);
  const updateAndSave = (patch: Partial<FormValues>) => {
    const next = { ...values, ...patch };
    update(patch);
    save(next);
  };
  const error = (show: boolean) => (show && attempted ? "border-red-500" : "");

  return (
    <>
      <div className="flex flex-wrap justify-center gap-2 border-b border-slate-200 px-4 py-4 @[900px]:flex-row @[900px]:space-x-4 @[900px]:py-2.5">
        <StepIndicator active label="Fill out the form" />
        <StepIndicator label="Book your event" />
      </div>
      <div className="mx-0 min-h-[calc(100vh-85px)] @[520px]:min-h-0 @[900px]:flex">
        <div className="w-full max-w-[520px] px-5 pb-10 pt-5 @[520px]:max-w-[480px] @[900px]:w-[404px] @[900px]:p-6">
          <div className="mb-4 flex items-center gap-2">
            <span className="text-xl">◉</span>
            <span className="text-base font-bold text-slate-900">Cadence</span>
          </div>
          <h2 className="break-words text-lg font-semibold leading-6 text-slate-900 sm:text-xl">
            See if Cadence is right for your team
          </h2>
          <div className="mt-2 text-sm font-normal leading-tight text-slate-600">
            <p>
              👉 This call is for teams that want to generate more qualified calls and
              understand whether Cadence can help.
            </p>
            <p className="my-3">
              🎯 <strong>On this 30-minute diagnostic call</strong>, we will review your
              current process, identify where qualified opportunities are being lost, and
              outline the next step.
            </p>
            <p>
              ⚡ <strong>No pressure.</strong> We will tell you honestly whether Cadence
              is a fit for your team.
            </p>
          </div>

          <form
            autoComplete="on"
            className="mt-4 min-w-[250px]"
            onSubmit={(event: FormEvent) => {
              event.preventDefault();
              setAttempted(true);
              if (valid && !submitting) onSubmit();
            }}
          >
            <div className="grid grid-cols-2 gap-x-4">
              <div className="col-span-2 mb-4 w-full">
                <label className="sr-only" htmlFor="cadence-phone">
                  Phone number
                </label>
                {country && (
                  <PhoneInput
                    country={country}
                    digits={values.phone}
                    onCountryChange={(next: Country) =>
                      updateAndSave({ countryVariant: next.variant })
                    }
                    onDigitsChange={(digits) => update({ phone: digits })}
                    onBlur={() => save(values)}
                    invalid={attempted && !phoneValid}
                  />
                )}
              </div>
              <div className="mb-4 w-full">
                <Input
                  id="cadence-first-name"
                  autoComplete="given-name"
                  placeholder="First name *"
                  maxLength={30}
                  value={values.firstName}
                  onChange={(event) => update({ firstName: event.target.value })}
                  onBlur={() => save(values)}
                  className={`h-10 border-slate-200 bg-background ${error(!validName(values.firstName))}`}
                />
              </div>
              <div className="mb-4 w-full">
                <Input
                  id="cadence-last-name"
                  autoComplete="family-name"
                  placeholder="Last name *"
                  maxLength={30}
                  value={values.lastName}
                  onChange={(event) => update({ lastName: event.target.value })}
                  onBlur={() => save(values)}
                  className={`h-10 border-slate-200 bg-background ${error(!validName(values.lastName))}`}
                />
              </div>
            </div>
            {isFirstSectionValid && (
              <div className="mb-4 w-full">
                <label className="sr-only" htmlFor="cadence-email">
                  Email address
                </label>
                <Input
                  id="cadence-email"
                  type="email"
                  autoComplete="email"
                  placeholder="Email address *"
                  maxLength={50}
                  value={values.email}
                  onChange={(event) => update({ email: event.target.value })}
                  onBlur={() => save(values)}
                  className={`h-10 border-slate-200 bg-background ${error(!validEmail(values.email))}`}
                />
              </div>
            )}
            {isFirstSectionValid && (
              <>
                <div className="mb-4 w-full">
                  <p className="mb-2 block text-sm font-medium text-slate-900">
                    How did you hear about us? *
                  </p>
                  <OptionList
                    options={HOW_HEARD_OPTIONS}
                    values={values.howHeard}
                    onChange={(value) =>
                      updateAndSave({ howHeard: toggleValue(values.howHeard, value) })
                    }
                  />
                </div>
                <div className="mb-4 w-full">
                  <label
                    htmlFor="cadence-currently-working"
                    className="mb-2 block text-sm font-medium text-slate-900"
                  >
                    What are you currently working on?
                  </label>
                  <textarea
                    id="cadence-currently-working"
                    value={values.currentlyWorkingOn}
                    onChange={(event) =>
                      update({ currentlyWorkingOn: event.target.value })
                    }
                    onBlur={() => save(values)}
                    rows={3}
                    placeholder="e.g. a product, a job search, a career transition..."
                    className="mt-2 w-full resize-y rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring"
                  />
                </div>
                <div className="mb-4 w-full">
                  <p className="mb-2 block text-sm font-medium text-slate-900">
                    What is stopping you? *
                  </p>
                  <OptionList
                    options={STOPPING_OPTIONS}
                    values={values.whatStoppingYou}
                    onChange={(value) =>
                      updateAndSave({
                        whatStoppingYou: toggleValue(values.whatStoppingYou, value),
                      })
                    }
                  />
                </div>
                <fieldset className="mb-4 w-full">
                  <legend className="text-sm font-medium text-slate-900">
                    Investment comfort *
                  </legend>
                  <div className="mt-2 flex flex-col gap-2">
                    {INVESTMENT_OPTIONS.map((option) => (
                      <label
                        key={option}
                        className="flex items-center gap-2 text-sm text-slate-600"
                      >
                        <input
                          type="radio"
                          name="investment"
                          checked={values.investmentComfort === option}
                          onChange={() => updateAndSave({ investmentComfort: option })}
                          className="accent-[#f52326]"
                        />
                        {option}
                      </label>
                    ))}
                  </div>
                </fieldset>
                <fieldset className="mb-4 w-full">
                  <legend className="text-sm font-medium text-slate-900">
                    How urgent is this? *
                  </legend>
                  <div className="mt-2 flex flex-col gap-2">
                    {URGENCY_OPTIONS.map((option) => (
                      <label
                        key={option}
                        className="flex items-center gap-2 text-sm text-slate-600"
                      >
                        <input
                          type="radio"
                          name="urgency"
                          checked={values.urgencyLevel === option}
                          onChange={() => updateAndSave({ urgencyLevel: option })}
                          className="accent-[#f52326]"
                        />
                        {option}
                      </label>
                    ))}
                  </div>
                </fieldset>
              </>
            )}
            {attempted && !valid && (
              <p className="mb-4 text-sm text-red-600">
                Please complete all required fields.
              </p>
            )}
            <Button
              type="submit"
              disabled={submitting}
              className="h-11 w-full rounded-lg bg-[#f52326] font-semibold text-white hover:opacity-90 md:h-10"
            >
              {submitting ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Saving...
                </>
              ) : (
                <>
                  Continue to book a slot <ChevronRight className="size-4" />
                </>
              )}
            </Button>
          </form>
        </div>
        <CalendarPreview />
      </div>
    </>
  );
}

function StepTwo({
  leadUnid,
  visitUnid,
  values,
  onBack,
}: {
  leadUnid: string;
  visitUnid: string | null;
  values: FormValues;
  onBack: () => void;
}) {
  const now = useMemo(() => new Date(), []);
  const browserTimezone = detectBrowserCountryTimezone();
  const [currentYear, setCurrentYear] = useState(now.getUTCFullYear());
  const [currentMonth, setCurrentMonth] = useState(now.getUTCMonth() + 1);
  const [selectedDay, setSelectedDay] = useState<number | null>(null);
  const [selectedTime, setSelectedTime] = useState<{
    time: string;
    dayOffset: number;
  } | null>(null);
  const [isBooking, setIsBooking] = useState(false);
  const [selectedTimezone, setSelectedTimezone] = useState<CountryTimezone>(
    browserTimezone ?? DEFAULT_TZ,
  );
  const [connectionStatus, setConnectionStatus] = useState<
    "loading" | "connected" | "error"
  >("loading");
  const [slots, setSlots] = useState<GoogleTimeSlot[] | null>(null);
  const [slotsLoading, setSlotsLoading] = useState(false);
  const [slotsError, setSlotsError] = useState(false);
  const [bookingError, setBookingError] = useState<string | null>(null);

  useEffect(() => {
    if (browserTimezone) return;
    const country = getCountryByVariant(values.countryVariant);
    const fallback = country ? defaultTimezoneForCountry(country.variant) : undefined;
    if (fallback) setSelectedTimezone(fallback);
  }, [browserTimezone, values.countryVariant]);

  useEffect(() => {
    void getCalendarEvents()
      .then(() => setConnectionStatus("connected"))
      .catch(() => setConnectionStatus("error"));
  }, []);

  const today = useMemo(
    () => currentDateInTz(selectedTimezone.iana),
    [selectedTimezone.iana],
  );
  const canGoPrev =
    currentYear > today.year ||
    (currentYear === today.year && currentMonth > today.month);
  const canGoNext = nextMonthHasDays(selectedTimezone.iana, currentYear, currentMonth);
  const monthAvailableDays = calculateAvailableDays(
    selectedTimezone.iana,
    currentYear,
    currentMonth,
  );
  const weeks = calendarWeeks(currentYear, currentMonth, true);
  const todayDay =
    today.year === currentYear && today.month === currentMonth ? today.day : null;
  const selectedDayDisplay = selectedDay
    ? `${new Intl.DateTimeFormat("en-US", {
        weekday: "short",
        timeZone: "UTC",
      }).format(
        new Date(Date.UTC(currentYear, currentMonth - 1, selectedDay)),
      )} ${selectedDay}`
    : null;
  const availableSlots = (slots ?? []).filter((slot) => slot.available);

  // monthAvailableDays is derived calendar data and creates a new array each render.
  // Keep dependencies aligned with the source values, matching Rustify's widget.
  // biome-ignore lint/correctness/useExhaustiveDependencies: derived calendar data
  useEffect(() => {
    setSelectedTime(null);
    if (selectedDay !== null && !monthAvailableDays.includes(selectedDay)) {
      setSelectedDay(null);
      return;
    }
    if (selectedDay === null) {
      setSlots(null);
      return;
    }

    const date = isoDate(currentYear, currentMonth, selectedDay);
    setSlotsLoading(true);
    setSlotsError(false);
    setSlots(null);
    void getAvailableSlots(date, selectedTimezone.iana)
      .then(setSlots)
      .catch(() => setSlotsError(true))
      .finally(() => setSlotsLoading(false));
  }, [currentMonth, currentYear, selectedDay, selectedTimezone.iana]);

  function goPrevMonth() {
    const previous = shiftMonth(currentYear, currentMonth, -1);
    setCurrentYear(previous.year);
    setCurrentMonth(previous.month);
    setSelectedDay(null);
    setSelectedTime(null);
  }

  function goNextMonth() {
    if (!canGoNext) return;
    const next = shiftMonth(currentYear, currentMonth, 1);
    setCurrentYear(next.year);
    setCurrentMonth(next.month);
    setSelectedDay(null);
    setSelectedTime(null);
  }

  async function book(slot: GoogleTimeSlot) {
    if (selectedDay === null || isBooking) return;
    const base = new Date(Date.UTC(currentYear, currentMonth - 1, selectedDay));
    base.setUTCDate(base.getUTCDate() + slot.day_offset);
    const date = isoDate(
      base.getUTCFullYear(),
      base.getUTCMonth() + 1,
      base.getUTCDate(),
    );
    setIsBooking(true);
    setBookingError(null);
    try {
      await createBooking({
        date,
        time: slot.time,
        timezone: selectedTimezone.iana,
        attendee_email: values.email.trim(),
        attendee_name: `${values.firstName.trim()} ${values.lastName.trim()}`.trim(),
        lead_id: leadUnid,
      });
      if (visitUnid) void updateCallingVisit(visitUnid, { booked_call: true });
      window.location.href = `/booking-confirmation?date=${encodeURIComponent(date)}&time=${encodeURIComponent(slot.time)}&tz=${encodeURIComponent(selectedTimezone.iana)}`;
    } catch (error) {
      setBookingError(
        error instanceof Error ? error.message : "Booking failed. Please try again.",
      );
    } finally {
      setIsBooking(false);
    }
  }

  return (
    <div className="border-border bg-card">
      <div className="relative w-full overflow-hidden border-b border-border px-4 py-4 text-sm @[900px]:py-2.5">
        <div className="flex w-full flex-wrap items-center justify-center gap-y-2 space-x-3">
          <StepIndicator done label="Fill out the form" />
          <StepIndicator active label="Book your event" />
        </div>
      </div>
      <div className="min-h-[calc(100vh-85px)] px-5 @[520px]:min-h-0 @[900px]:flex">
        <div className="flex flex-col border-border pt-5 pb-4 @[900px]:mr-6 @[900px]:min-h-[460px] @[900px]:w-[280px] @[900px]:border-r @[900px]:py-6 @[900px]:pt-6 @[900px]:pr-6">
          <div className="mb-4">
            <h2 className="pr-4 text-lg font-semibold break-words text-foreground">
              See if Cadence is right for your team
            </h2>
          </div>
          <div className="flex h-full flex-col items-start justify-between">
            <div className="w-full space-y-2">
              <div className="flex items-center space-x-2 px-0.5 py-1 text-sm font-medium">
                <GoogleMeetIcon />
                <p className="text-sm font-medium text-foreground">Google Meet</p>
                {connectionStatus === "connected" && (
                  <Check className="size-4 text-green-500" />
                )}
              </div>
              <div className="flex items-center space-x-2 px-0.5 py-1 text-sm font-medium">
                <Clock className="size-5 text-muted-foreground" />
                <p className="text-sm font-medium text-foreground">30 minutes</p>
              </div>
              <TimezoneCombobox
                selected={selectedTimezone}
                onSelect={(timezone) => {
                  setSelectedTimezone(timezone);
                  setSelectedDay(null);
                  setSelectedTime(null);
                }}
              />
            </div>
          </div>
          <button
            type="button"
            onClick={onBack}
            className="mt-auto pt-6 text-left text-sm text-muted-foreground hover:text-foreground"
          >
            ← Back to form
          </button>
        </div>

        <div className="relative mx-auto w-full max-w-[412px] pb-4 @[900px]:w-[380px] @[900px]:py-6">
          <div className="flex items-center justify-center transition-opacity duration-300">
            <div className="mx-auto w-full">
              <div className="@[900px]:pr-6">
                <div className="flex items-center justify-between">
                  <p className="text-base font-medium text-foreground">
                    {monthLabel(currentYear, currentMonth)}
                  </p>
                  <div className="relative z-[2] flex items-center gap-2">
                    <CalendarButton
                      direction="back"
                      disabled={!canGoPrev}
                      onClick={goPrevMonth}
                    />
                    <CalendarButton
                      direction="forward"
                      disabled={!canGoNext}
                      onClick={goNextMonth}
                    />
                  </div>
                </div>
                <div className="overflow-x-auto pt-4 @[900px]:pt-3">
                  <table className="w-full">
                    <thead>
                      <tr className="flex">
                        {BOOKING_DAYS_OF_WEEK.map((day) => (
                          <th key={day} className="flex-1">
                            <div className="flex w-full justify-center">
                              <p className="text-center text-xs font-semibold uppercase text-muted-foreground">
                                {day}
                              </p>
                            </div>
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody className="space-y-1.5 @[900px]:space-y-2">
                      {weeks.map((week, weekIndex) => (
                        <tr
                          key={weekIndex}
                          className={`flex gap-1.5 @[900px]:gap-2 ${weekIndex === 0 ? "mt-2" : ""}`}
                        >
                          {week.map((day, dayIndex) => (
                            <td key={dayIndex} className="flex-1 p-0">
                              {day !== null && (
                                <BookingCalendarDay
                                  day={day}
                                  isToday={day === todayDay}
                                  available={monthAvailableDays.includes(day)}
                                  isSelected={selectedDay === day}
                                  onSelect={() => {
                                    setSelectedDay(day);
                                    setSelectedTime(null);
                                  }}
                                />
                              )}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="border-border pb-4 @[900px]:w-[210px] @[900px]:border-l @[900px]:py-6 @[900px]:pl-6">
          {selectedDay !== null ? (
            <div>
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="text-base leading-6 font-medium text-muted-foreground">
                  {selectedDayDisplay}
                  <span className="text-sm font-normal text-muted-foreground/70">
                    {slots && ` (${availableSlots.length} slots)`}
                  </span>
                </h2>
                <div className="h-8 rounded-md border border-border px-2 py-1 text-sm font-medium text-[#f52326]">
                  24h
                </div>
              </div>

              {bookingError && (
                <p className="mb-2 text-sm text-red-500">{bookingError}</p>
              )}

              <div className="mt-4 max-h-[300px] space-y-3 overflow-y-auto pr-2 @[900px]:max-h-[400px]">
                {slotsLoading ? (
                  <div className="flex items-center justify-center py-4">
                    <div className="size-5 animate-spin rounded-full border-b-2 border-muted-foreground" />
                  </div>
                ) : slotsError ? (
                  <p className="py-4 text-center text-sm text-red-500">
                    Failed to load time slots. Please try again.
                  </p>
                ) : availableSlots.length === 0 ? (
                  <p className="py-4 text-center text-sm text-muted-foreground">
                    No available slots. Please check other days for availability.
                  </p>
                ) : (
                  availableSlots.map((slot) => {
                    const key = `${slot.time}_${slot.day_offset}`;
                    const isSelected =
                      selectedTime?.time === slot.time &&
                      selectedTime?.dayOffset === slot.day_offset;
                    const displayTime =
                      slot.day_offset === -1
                        ? `${slot.time} (prev day)`
                        : slot.day_offset === 1
                          ? `${slot.time} (next day)`
                          : slot.time;
                    return (
                      <div key={key} className="flex w-full overflow-hidden">
                        <button
                          type="button"
                          onClick={() => {
                            if (!isBooking) {
                              setSelectedTime({
                                time: slot.time,
                                dayOffset: slot.day_offset,
                              });
                            }
                          }}
                          disabled={isBooking}
                          className={`flex h-10 items-center justify-center rounded-lg border border-border py-2 text-sm font-medium transition-all hover:border-[rgba(245,35,38,0.3)] hover:bg-[rgba(245,35,38,0.1)] disabled:cursor-not-allowed disabled:opacity-40 ${isSelected ? "w-[calc(50%-6px)]" : "w-full @[900px]:max-w-[185px]"}`}
                        >
                          {displayTime}
                        </button>
                        <button
                          type="button"
                          onClick={() => void book(slot)}
                          disabled={isBooking}
                          className={`ml-3 flex h-10 items-center justify-center rounded-lg py-2 text-sm font-medium text-white transition-all hover:opacity-90 active:scale-[0.98] disabled:opacity-60 ${isSelected ? "visible w-[calc(50%-6px)]" : "invisible w-0"}`}
                          style={{ background: BRAND }}
                        >
                          {isBooking ? "Booking..." : "Confirm"}
                        </button>
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          ) : (
            <div className="flex h-full min-h-[200px] items-center justify-center">
              <p className="text-sm text-muted-foreground">Select a date</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function CadenceBookingWidget({
  initialCountryAlpha2,
}: {
  initialCountryAlpha2?: string | null;
}) {
  const initialCountryVariant =
    getCountryByAlpha2(initialCountryAlpha2 ?? "FR")?.variant ??
    EMPTY_FORM.countryVariant;
  const [values, setValues] = useState<FormValues>(() => ({
    ...EMPTY_FORM,
    countryVariant: initialCountryVariant,
  }));
  const [step, setStep] = useState<1 | 2>(1);
  const [submitting, setSubmitting] = useState(false);
  const [leadUnid] = useState(() =>
    typeof crypto !== "undefined" ? crypto.randomUUID() : "",
  );
  const [visitUnid, setVisitUnid] = useState<string | null>(null);

  useEffect(() => {
    if (!leadUnid) return;
    void createCallingVisit({
      source_page: window.location.pathname,
      referer: document.referrer || null,
      utm_source: queryValue("utm_source"),
      utm_medium: queryValue("utm_medium"),
      utm_campaign: queryValue("utm_campaign"),
      user_agent: navigator.userAgent,
    })
      .then((visit) => setVisitUnid(visit.unid))
      .catch(() => undefined);
    return undefined;
  }, [leadUnid]);

  async function submit() {
    setSubmitting(true);
    try {
      await saveLead(leadUnid, leadUpdate(values));
      await submitLead(leadUnid, window.location.pathname);
      if (visitUnid) void updateCallingVisit(visitUnid, { form_submitted: true });
      await getLead(leadUnid);
      setStep(2);
      document
        .getElementById("booking-widget")
        ?.scrollIntoView({ behavior: "smooth", block: "start" });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div
      id="booking-widget"
      className="flex min-w-full flex-col items-center overflow-auto @container"
    >
      <div className="w-full max-w-[520px] @[520px]:max-w-[480px] @[900px]:w-fit @[900px]:max-w-full">
        <div className="border-slate-200 bg-white @[520px]:rounded-lg @[520px]:border">
          {step === 1 ? (
            <StepOne
              leadUnid={leadUnid}
              values={values}
              setValues={setValues}
              onSubmit={() => void submit()}
              submitting={submitting}
            />
          ) : (
            <StepTwo
              leadUnid={leadUnid}
              visitUnid={visitUnid}
              values={values}
              onBack={() => setStep(1)}
            />
          )}
        </div>
      </div>
    </div>
  );
}
