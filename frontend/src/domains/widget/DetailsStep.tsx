import { ChevronLeft, CircleAlert, Loader2 } from "lucide-react";
import { useRef, useState, type ReactNode } from "react";

import { Alert, AlertDescription } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { Checkbox } from "../../components/ui/checkbox";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import { Textarea } from "../../components/ui/textarea";
import { formatClock } from "../../lib/format";
import {
  getCountryByAlpha2,
  getCountryByVariant,
  timezoneDisplayWithTime,
  type CountryTimezone,
} from "../shared/phone/country";
import { PhoneInput } from "../shared/phone/PhoneInput";
import {
  formErrors,
  HOW_HEARD_OPTIONS,
  INVESTMENT_OPTIONS,
  STOPPING_OPTIONS,
  toggleValue,
  URGENCY_OPTIONS,
  type FormValues,
} from "./booking-form";
import { BookingSummary, CALL_MINUTES } from "./BookingSummary";
import type { PickedSlot } from "./SlotStep";

const ID = "cadence";

function endTime(time: string): string {
  const [hours, minutes] = time.split(":").map(Number);
  const end = hours * 60 + minutes + CALL_MINUTES;
  return `${Math.floor(end / 60) % 24}:${String(end % 60).padStart(2, "0")}`;
}

function Field({
  id,
  label,
  error,
  required = true,
  children,
}: {
  id: string;
  label: string;
  error?: string;
  required?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="grid content-start gap-1.5">
      <Label htmlFor={id}>
        {label}
        {required ? null : (
          <span className="font-normal text-muted-foreground">(optional)</span>
        )}
      </Label>
      {children}
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
    </div>
  );
}

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
    <fieldset className="grid content-start gap-2.5">
      <legend className="mb-2.5 text-sm leading-none font-medium">{legend}</legend>
      {children}
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
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

export function DetailsStep({
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
  slot: PickedSlot;
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
  const allErrors = formErrors(values);
  const errors = attempted ? allErrors : {};
  const country = getCountryByVariant(values.countryVariant) ?? getCountryByAlpha2("FR");

  const update = (patch: Partial<FormValues>) => onChange({ ...values, ...patch });
  const updateAndSave = (patch: Partial<FormValues>) => {
    const next = { ...values, ...patch };
    onChange(next);
    onSave(next);
  };

  const day = new Intl.DateTimeFormat("en", {
    weekday: "long",
    day: "numeric",
    month: "long",
    timeZone: "UTC",
  }).format(new Date(`${slot.date}T00:00:00Z`));

  return (
    <div className="grid divide-y @[760px]:grid-cols-[15rem_minmax(0,1fr)] @[760px]:divide-x @[760px]:divide-y-0">
      <div className="p-5 @[760px]:p-6">
        <BookingSummary timezone={timezoneDisplayWithTime(timezone)}>
          <div className="rounded-lg border bg-muted p-3">
            <p className="text-sm font-semibold">{day}</p>
            <p className="text-sm tabular-nums">
              {formatClock(slot.time, hour12)} to{" "}
              {formatClock(endTime(slot.time), hour12)}
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
        <h3 className="font-display text-lg font-semibold tracking-tight">
          Your details
        </h3>

        <div className="grid gap-4 @md/form:grid-cols-2">
          <Field id={`${ID}-first-name`} label="First name" error={errors.firstName}>
            <Input
              id={`${ID}-first-name`}
              autoComplete="given-name"
              maxLength={30}
              value={values.firstName}
              aria-invalid={Boolean(errors.firstName)}
              onChange={(event) => update({ firstName: event.target.value })}
              onBlur={() => onSave(values)}
            />
          </Field>
          <Field id={`${ID}-last-name`} label="Last name" error={errors.lastName}>
            <Input
              id={`${ID}-last-name`}
              autoComplete="family-name"
              maxLength={30}
              value={values.lastName}
              aria-invalid={Boolean(errors.lastName)}
              onChange={(event) => update({ lastName: event.target.value })}
              onBlur={() => onSave(values)}
            />
          </Field>
          <Field id={`${ID}-email`} label="Email" error={errors.email}>
            <Input
              id={`${ID}-email`}
              type="email"
              autoComplete="email"
              maxLength={50}
              value={values.email}
              aria-invalid={Boolean(errors.email)}
              onChange={(event) => update({ email: event.target.value })}
              onBlur={() => onSave(values)}
            />
          </Field>
          <Field id={`${ID}-phone`} label="Phone" error={errors.phone}>
            {country ? (
              <PhoneInput
                id={`${ID}-phone`}
                country={country}
                digits={values.phone}
                onCountryChange={(next) =>
                  updateAndSave({ countryVariant: next.variant })
                }
                onDigitsChange={(digits) => update({ phone: digits })}
                onBlur={() => onSave(values)}
                invalid={Boolean(errors.phone)}
              />
            ) : null}
          </Field>
        </div>

        <Field
          id={`${ID}-working-on`}
          label="What are you currently working on?"
          required={false}
        >
          <Textarea
            id={`${ID}-working-on`}
            rows={3}
            placeholder="A product, a job search, a career change"
            value={values.currentlyWorkingOn}
            onChange={(event) => update({ currentlyWorkingOn: event.target.value })}
            onBlur={() => onSave(values)}
          />
        </Field>

        <div className="grid gap-x-4 gap-y-5 @md/form:grid-cols-2">
          <OptionGroup legend="How did you hear about us?" error={errors.howHeard}>
            <CheckboxOptions
              name="heard"
              options={HOW_HEARD_OPTIONS}
              values={values.howHeard}
              invalid={Boolean(errors.howHeard)}
              onToggle={(option) =>
                updateAndSave({ howHeard: toggleValue(values.howHeard, option) })
              }
            />
          </OptionGroup>
          <OptionGroup legend="What is stopping you?" error={errors.whatStoppingYou}>
            <CheckboxOptions
              name="stopping"
              options={STOPPING_OPTIONS}
              values={values.whatStoppingYou}
              invalid={Boolean(errors.whatStoppingYou)}
              onToggle={(option) =>
                updateAndSave({
                  whatStoppingYou: toggleValue(values.whatStoppingYou, option),
                })
              }
            />
          </OptionGroup>
          <OptionGroup legend="Investment comfort" error={errors.investmentComfort}>
            <RadioOptions
              name="investment"
              options={INVESTMENT_OPTIONS}
              value={values.investmentComfort}
              invalid={Boolean(errors.investmentComfort)}
              onChange={(option) => updateAndSave({ investmentComfort: option })}
            />
          </OptionGroup>
          <OptionGroup legend="How urgent is this?" error={errors.urgencyLevel}>
            <RadioOptions
              name="urgency"
              options={URGENCY_OPTIONS}
              value={values.urgencyLevel}
              invalid={Boolean(errors.urgencyLevel)}
              onChange={(option) => updateAndSave({ urgencyLevel: option })}
            />
          </OptionGroup>
        </div>

        {submitError ? (
          <Alert variant="destructive">
            <CircleAlert className="size-4" />
            <AlertDescription>{submitError}</AlertDescription>
          </Alert>
        ) : null}

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
