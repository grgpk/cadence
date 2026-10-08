import type { LeadUpdate } from "../../bindings/LeadUpdate";
import { getCountryByVariant } from "../shared/phone/country";
import {
  formatForCountry,
  hasRepeatedDigits,
  MIN_REPEATED_DIGITS,
} from "../shared/phone/phone-format";

export const HOW_HEARD_OPTIONS = ["Word of Mouth", "Google", "LinkedIn", "YouTube"];
export const STOPPING_OPTIONS = [
  "Lack of time",
  "Lack of guidance/structure",
  "Don't know where to start",
  "Tried before and got stuck",
];
export const INVESTMENT_OPTIONS = [
  "$100–500 (courses, books)",
  "$500–2,000 (serious training)",
  "$2,000–5,000 (career investment)",
  "$5,000+ (maximum support)",
];
export const URGENCY_OPTIONS = [
  "1-3 (Nice to have)",
  "4-6 (Important but not urgent)",
  "7-8 (High priority)",
  "9-10 (Critical, need it now)",
];

export type FormValues = {
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

export const EMPTY_FORM: FormValues = {
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

export function queryValue(key: string): string | null {
  return new URLSearchParams(window.location.search).get(key);
}

function qualification(values: FormValues): LeadUpdate["qualification_status"] {
  if (!values.investmentComfort) return "NotSure";
  return values.investmentComfort === INVESTMENT_OPTIONS[0] ? "NotSure" : "Qualified";
}

export function leadUpdate(values: FormValues): LeadUpdate {
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
    source_page: window.location.pathname,
    utm_source: queryValue("utm_source"),
    utm_medium: queryValue("utm_medium"),
    utm_campaign: queryValue("utm_campaign"),
    qualification_status: qualification(values),
  };
}

function validName(value: string): boolean {
  return value.trim().length >= 2 && !/\d/.test(value);
}

function validEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value.trim());
}

function validPhone(values: FormValues): boolean {
  const country = getCountryByVariant(values.countryVariant);
  if (!country) return false;
  return (
    values.phone.length === formatForCountry(country.variant).maxDigits &&
    !hasRepeatedDigits(values.phone, MIN_REPEATED_DIGITS)
  );
}

type FormErrors = Partial<Record<keyof FormValues, string>>;

/** One message per invalid field; an empty object means the form can be sent. */
export function formErrors(values: FormValues): FormErrors {
  const errors: FormErrors = {};
  if (!validName(values.firstName)) errors.firstName = "Enter your first name.";
  if (!validName(values.lastName)) errors.lastName = "Enter your last name.";
  if (!validEmail(values.email)) errors.email = "Enter a valid email address.";
  if (!validPhone(values)) errors.phone = "Enter a complete phone number.";
  if (values.howHeard.length === 0) errors.howHeard = "Pick at least one option.";
  if (values.whatStoppingYou.length === 0) {
    errors.whatStoppingYou = "Pick at least one option.";
  }
  if (!values.investmentComfort) errors.investmentComfort = "Pick one option.";
  if (!values.urgencyLevel) errors.urgencyLevel = "Pick one option.";
  return errors;
}

export function toggleValue(values: string[], value: string): string[] {
  return values.includes(value)
    ? values.filter((item) => item !== value)
    : [...values, value];
}
