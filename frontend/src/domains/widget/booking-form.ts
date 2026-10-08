import type { BookingPage } from "../../bindings/BookingPage";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import { DEFAULT_TIMEZONE, getCountryByVariant } from "../shared/phone/country";
import {
  formatForCountry,
  hasRepeatedDigits,
  MIN_REPEATED_DIGITS,
} from "../shared/phone/phone-format";

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
  countryVariant: DEFAULT_TIMEZONE.variant,
  firstName: "",
  lastName: "",
  email: "",
  howHeard: [],
  currentlyWorkingOn: "",
  whatStoppingYou: [],
  investmentComfort: "",
  urgencyLevel: "",
};

/** The switches a host sets for the optional questions. */
type AskedQuestions = Pick<BookingPage, Extract<keyof BookingPage, `ask_${string}`>>;

type QuestionBase = { flag: keyof AskedQuestions; label: string };
type Question = QuestionBase &
  (
    | { kind: "text"; field: "currentlyWorkingOn"; placeholder: string }
    | { kind: "many"; field: "howHeard" | "whatStoppingYou"; options: string[] }
    | { kind: "one"; field: "investmentComfort" | "urgencyLevel"; options: string[] }
  );

const INVESTMENT_OPTIONS = [
  "$100–500 (courses, books)",
  "$500–2,000 (serious training)",
  "$2,000–5,000 (career investment)",
  "$5,000+ (maximum support)",
];

/** Every optional question, in form order. The host's settings decide which are asked. */
export const QUESTIONS: Question[] = [
  {
    kind: "text",
    field: "currentlyWorkingOn",
    flag: "ask_currently_working_on",
    label: "What are you currently working on?",
    placeholder: "A product, a job search, a career change",
  },
  {
    kind: "many",
    field: "howHeard",
    flag: "ask_how_heard_about_us",
    label: "How did you hear about us?",
    options: ["Word of Mouth", "Google", "LinkedIn", "YouTube"],
  },
  {
    kind: "many",
    field: "whatStoppingYou",
    flag: "ask_what_stopping_you",
    label: "What is stopping you?",
    options: [
      "Lack of time",
      "Lack of guidance/structure",
      "Don't know where to start",
      "Tried before and got stuck",
    ],
  },
  {
    kind: "one",
    field: "investmentComfort",
    flag: "ask_investment_comfort",
    label: "Investment comfort",
    options: INVESTMENT_OPTIONS,
  },
  {
    kind: "one",
    field: "urgencyLevel",
    flag: "ask_urgency_level",
    label: "How urgent is this?",
    options: [
      "1-3 (Nice to have)",
      "4-6 (Important but not urgent)",
      "7-8 (High priority)",
      "9-10 (Critical, need it now)",
    ],
  },
];

export function queryValue(key: string): string | null {
  return new URLSearchParams(window.location.search).get(key);
}

function qualification(values: FormValues): LeadUpdate["qualification_status"] {
  // Without an investment answer there is nothing to qualify on.
  if (!values.investmentComfort) return null;
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
export function formErrors(values: FormValues, asked: AskedQuestions): FormErrors {
  const errors: FormErrors = {};
  if (!validName(values.firstName)) errors.firstName = "Enter your first name.";
  if (!validName(values.lastName)) errors.lastName = "Enter your last name.";
  if (!validEmail(values.email)) errors.email = "Enter a valid email address.";
  if (!validPhone(values)) errors.phone = "Enter a complete phone number.";
  for (const question of QUESTIONS) {
    if (!asked[question.flag] || question.kind === "text") continue;
    if (values[question.field].length === 0) {
      errors[question.field] =
        question.kind === "many" ? "Pick at least one option." : "Pick one option.";
    }
  }
  return errors;
}

export function toggleValue(values: string[], value: string): string[] {
  return values.includes(value)
    ? values.filter((item) => item !== value)
    : [...values, value];
}
