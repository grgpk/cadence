import { ChevronLeft, CircleAlert, Loader2 } from "lucide-react";
import { useRef, useState, type ReactNode } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { Checkbox } from "../../components/ui/checkbox";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import { Textarea } from "../../components/ui/textarea";
import { formatDay } from "../../lib/format";
import { ConsentNotice } from "../shared/consent-notice";
import { Field, FieldError, fieldHelpId } from "../shared/field";
import {
  DEFAULT_TIMEZONE,
  getCountryByVariant,
  timezoneDisplayWithTime,
  type CountryTimezone,
} from "../shared/phone/country";
import { PhoneInput } from "../shared/phone/phone-input";
import { formErrors, QUESTIONS, toggleValue, type FormValues } from "./booking-form";
import { BookingSummary } from "./booking-summary";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { callEnd, clockLabel, dayPhase, zonedTime } from "../../lib/zoned-time";

const ID = "cadence";

function OptionGroup({
  legend,
  error,
  children,
}: {
  legend: string;
  error?: string;
  children: ReactNode;
}) {
  return (
    <fieldset className="grid content-start">
      <legend className="mb-2.5 text-sm leading-none font-medium">{legend}</legend>
      <div className="grid gap-2.5">{children}</div>
      <FieldError error={error} />
    </fieldset>
  );
}

function CheckboxOptions({
  name,
  options,
  values,
  invalid,
  onToggle,
}: {
  name: string;
  options: string[];
  values: string[];
  invalid: boolean;
  onToggle: (option: string) => void;
}) {
  return options.map((option, index) => (
    <Label key={option} htmlFor={`${ID}-${name}-${index}`} className="font-normal">
      <Checkbox
        id={`${ID}-${name}-${index}`}
        checked={values.includes(option)}
        aria-invalid={invalid}
        onCheckedChange={() => onToggle(option)}
      />
      {option}
    </Label>
  ));
}

function RadioOptions({
  name,
  options,
  value,
  invalid,
  onChange,
}: {
  name: string;
  options: string[];
  value: string;
  invalid: boolean;
  onChange: (option: string) => void;
}) {
  return (
    <RadioGroup value={value} onValueChange={onChange} className="gap-2.5">
      {options.map((option, index) => (
        <Label key={option} htmlFor={`${ID}-${name}-${index}`} className="font-normal">
          <RadioGroupItem
            id={`${ID}-${name}-${index}`}
            value={option}
            aria-invalid={invalid}
          />
          {option}
        </Label>
      ))}
    </RadioGroup>
  );
}

const CONTACT_FIELDS = ["firstName", "lastName", "email", "phone"] as const;
type ContactField = (typeof CONTACT_FIELDS)[number];

function isContactField(field: string): field is ContactField {
  return (CONTACT_FIELDS as readonly string[]).includes(field);
}

export function DetailsStep({
  page,
  slot,
  timezone,
  hour12,
  values,
  onChange,
  onSave,
  onBack,
  onSubmit,
  submitting,
  submitError,
}: {
  page: BookingPage;
  slot: TimeSlot;
  timezone: CountryTimezone;
  hour12: boolean;
  values: FormValues;
  onChange: (next: FormValues) => void;
  /** Stores the answers so far, so a half-filled form is not lost. */
  onSave: (next: FormValues) => void;
  onBack: () => void;
  onSubmit: () => void;
  submitting: boolean;
  submitError: string | null;
}) {
  const formRef = useRef<HTMLFormElement>(null);
  const [attempted, setAttempted] = useState(false);
  // Contact fields the visitor has left with something wrong. A field speaks up
  // when it is left, and goes quiet the moment it is fixed: it never scolds
  // while the visitor is still typing it.
  const [left, setLeft] = useState<ContactField[]>([]);
  const allErrors = formErrors(values, page.questions);
  const errors: typeof allErrors = Object.fromEntries(
    Object.entries(allErrors).filter(([field]) =>
      isContactField(field) ? left.includes(field) : attempted,
    ),
  );
  const leave = (field: ContactField) => {
    if (allErrors[field] && !left.includes(field)) setLeft([...left, field]);
    onSave(values);
  };
  const edit = (field: ContactField, patch: Partial<FormValues>) => {
    const next = { ...values, ...patch };
    onChange(next);
    if (left.includes(field) && !formErrors(next, page.questions)[field]) {
      setLeft(left.filter((each) => each !== field));
    }
  };
  const country =
    getCountryByVariant(values.countryVariant) ??
    getCountryByVariant(DEFAULT_TIMEZONE.variant);

  const update = (patch: Partial<FormValues>) => onChange({ ...values, ...patch });
  const updateAndSave = (patch: Partial<FormValues>) => {
    const next = { ...values, ...patch };
    onChange(next);
    onSave(next);
  };

  const start = new Date(slot.start);
  const phase = dayPhase(zonedTime(start, timezone.iana).hour);
  const textQuestions = QUESTIONS.filter(
    (question) => question.kind === "text" && page.questions.includes(question.key),
  );
  const choiceQuestions = QUESTIONS.filter(
    (question) => question.kind !== "text" && page.questions.includes(question.key),
  );

  return (
    <div className="grid divide-y @[760px]:grid-cols-[15rem_minmax(0,1fr)] @[760px]:divide-x @[760px]:divide-y-0">
      <div className="p-5 @[760px]:p-6">
        <BookingSummary
          page={page}
          minutes={slot.minutes}
          timezone={timezoneDisplayWithTime(timezone, hour12)}
        >
          <div className="relative overflow-hidden rounded-lg border bg-muted py-3 pr-3 pl-4">
            <span
              aria-hidden="true"
              className={`vt-booked-block absolute inset-y-0 left-0 w-[3px] ${PHASE_STYLES[phase].rail}`}
            />
            <p className="text-sm font-semibold">
              {formatDay(slot.start, timezone.iana)}
            </p>
            <p className="text-clock text-base">
              {clockLabel(start, timezone.iana, hour12)} to{" "}
              {clockLabel(callEnd(slot.start, slot.minutes), timezone.iana, hour12)}
            </p>
            <Button
              type="button"
              variant="link"
              size="sm"
              className="mt-1 h-auto p-0 has-[>svg]:px-0"
              onClick={onBack}
            >
              <ChevronLeft />
              Change time
            </Button>
          </div>
        </BookingSummary>
      </div>

      <form
        ref={formRef}
        noValidate
        autoComplete="on"
        className="@container/form grid gap-5 p-5 @[760px]:p-6"
        onSubmit={(event) => {
          event.preventDefault();
          setAttempted(true);
          setLeft(CONTACT_FIELDS.filter((field) => allErrors[field]));
          if (Object.keys(allErrors).length > 0) {
            // Wait for the error state to render, then land on the first problem.
            requestAnimationFrame(() =>
              formRef.current
                ?.querySelector<HTMLElement>('[aria-invalid="true"]')
                ?.focus(),
            );
            return;
          }
          if (!submitting) onSubmit();
        }}
      >
        <h2 className="font-display text-lg font-semibold tracking-tight">
          Your details
        </h2>

        <div className="grid gap-4 @md/form:grid-cols-2">
          <Field
            className="content-start"
            id={`${ID}-first-name`}
            label="First name"
            error={errors.firstName}
          >
            <Input
              id={`${ID}-first-name`}
              autoComplete="given-name"
              maxLength={60}
              value={values.firstName}
              aria-invalid={Boolean(errors.firstName)}
              aria-describedby={
                errors.firstName ? fieldHelpId(`${ID}-first-name`) : undefined
              }
              onChange={(event) => edit("firstName", { firstName: event.target.value })}
              onBlur={() => leave("firstName")}
            />
          </Field>
          <Field
            className="content-start"
            id={`${ID}-last-name`}
            label="Last name"
            error={errors.lastName}
          >
            <Input
              id={`${ID}-last-name`}
              autoComplete="family-name"
              maxLength={60}
              value={values.lastName}
              aria-invalid={Boolean(errors.lastName)}
              aria-describedby={
                errors.lastName ? fieldHelpId(`${ID}-last-name`) : undefined
              }
              onChange={(event) => edit("lastName", { lastName: event.target.value })}
              onBlur={() => leave("lastName")}
            />
          </Field>
          <Field
            className="content-start"
            id={`${ID}-email`}
            label="Email"
            error={errors.email}
          >
            <Input
              id={`${ID}-email`}
              type="email"
              autoComplete="email"
              maxLength={254}
              value={values.email}
              aria-invalid={Boolean(errors.email)}
              aria-describedby={errors.email ? fieldHelpId(`${ID}-email`) : undefined}
              onChange={(event) => edit("email", { email: event.target.value })}
              onBlur={() => leave("email")}
            />
          </Field>
          <Field
            className="content-start"
            id={`${ID}-phone`}
            label="Phone"
            error={errors.phone}
          >
            {country ? (
              <PhoneInput
                id={`${ID}-phone`}
                country={country}
                digits={values.phone}
                onCountryChange={(next) =>
                  updateAndSave({ countryVariant: next.variant })
                }
                onDigitsChange={(digits) => edit("phone", { phone: digits })}
                onBlur={() => leave("phone")}
                invalid={Boolean(errors.phone)}
                describedBy={errors.phone ? fieldHelpId(`${ID}-phone`) : undefined}
              />
            ) : null}
          </Field>
        </div>

        {textQuestions.map((question) =>
          question.kind === "text" ? (
            <Field
              key={question.field}
              id={`${ID}-${question.field}`}
              label={question.label}
              optional
            >
              <Textarea
                id={`${ID}-${question.field}`}
                rows={3}
                placeholder={question.placeholder}
                value={values[question.field]}
                onChange={(event) => update({ [question.field]: event.target.value })}
                onBlur={() => onSave(values)}
              />
            </Field>
          ) : null,
        )}

        {choiceQuestions.length > 0 ? (
          <div className="grid gap-x-4 gap-y-5 @md/form:grid-cols-2">
            {choiceQuestions.map((question) => {
              if (question.kind === "text") return null;
              const error = errors[question.field];
              return (
                <OptionGroup key={question.field} legend={question.label} error={error}>
                  {question.kind === "many" ? (
                    <CheckboxOptions
                      name={question.field}
                      options={question.options}
                      values={values[question.field]}
                      invalid={Boolean(error)}
                      onToggle={(option) =>
                        updateAndSave({
                          [question.field]: toggleValue(values[question.field], option),
                        })
                      }
                    />
                  ) : (
                    <RadioOptions
                      name={question.field}
                      options={question.options}
                      value={values[question.field]}
                      invalid={Boolean(error)}
                      onChange={(option) => updateAndSave({ [question.field]: option })}
                    />
                  )}
                </OptionGroup>
              );
            })}
          </div>
        ) : null}

        {submitError ? (
          <Alert
            variant="destructive"
            className="motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1"
          >
            <CircleAlert className="size-4" />
            <AlertDescription>{submitError}</AlertDescription>
          </Alert>
        ) : null}

        <ConsentNotice
          followUpHost={page.follow_up_notice ? page.host_name : undefined}
        />

        <div className="flex items-center justify-between gap-3 border-t pt-5">
          <Button type="button" variant="ghost" onClick={onBack}>
            Back
          </Button>
          <Button type="submit" disabled={submitting}>
            {submitting ? <Loader2 className="animate-spin" /> : null}
            {submitting ? "Booking" : "Confirm booking"}
          </Button>
        </div>
      </form>
    </div>
  );
}
