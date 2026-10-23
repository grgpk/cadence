import type { LeadAnswer } from "../../bindings/LeadAnswer";
import type { LeadUpdate } from "../../bindings/LeadUpdate";
import type { Question as QuestionKey } from "../../bindings/Question";
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

type QuestionBase = { key: QuestionKey; label: string };
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
    key: "currently_working_on",
    label: "What are you currently working on?",
    placeholder: "A product, a job search, a career change",
  },
  {
    kind: "many",
    field: "howHeard",
    key: "how_heard_about_us",
    label: "How did you hear about us?",
    options: ["Word of Mouth", "Google", "LinkedIn", "YouTube"],
  },
  {
    kind: "many",
    field: "whatStoppingYou",
    key: "what_stopping_you",
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
    key: "investment_comfort",
    label: "Investment comfort",
    options: INVESTMENT_OPTIONS,
  },
  {
    kind: "one",
    field: "urgencyLevel",
    key: "urgency_level",
    label: "How urgent is this?",
    options: [
      "1-3 (Nice to have)",
      "4-6 (Important but not urgent)",
      "7-8 (High priority)",
      "9-10 (Critical, need it now)",
    ],
  },
];

/** Where a visitor landed: the booking page's address, read once. */
export type Arrival = {
  /** The path, such as `/book/ana`. */
  path: string;
  /** The query string, such as `?utm_source=newsletter`. */
  search: string;
  referrer: string;
  userAgent: string;
};

/** The address the browser is on now. */
export function arrivalHere(): Arrival {
  return {
    path: window.location.pathname,
    search: window.location.search,
    referrer: document.referrer,
    userAgent: navigator.userAgent,
  };
}

/** One value of the query string, such as a UTM tag. */
export function queryValue(search: string, key: string): string | null {
  return new URLSearchParams(search).get(key);
}

/** What a lead answered to one question, if it did. */
export function answerTo(
  answers: readonly LeadAnswer[],
  key: QuestionKey,
): string | null {
  return answers.find((answer) => answer.question === key)?.answer ?? null;
}

/** A question's answer as one line; several picks are joined by commas. */
function answerOf(values: FormValues, question: Question): string {
  const value = values[question.field];
  return typeof value === "string" ? value.trim() : value.join(", ");
}

/**
 * The form as the lead stores it. Every answer is sent, an empty one too, so
 * an answer taken back is removed; the server keeps only asked questions.
 */
export function leadUpdate(values: FormValues, visitUnid: string | null): LeadUpdate {
  const country = getCountryByVariant(values.countryVariant);
  const answers: LeadAnswer[] = QUESTIONS.map((question) => ({
    question: question.key,
    answer: answerOf(values, question),
  }));
  return {
    first_name: values.firstName.trim() || null,
    last_name: values.lastName.trim() || null,
    email: values.email.trim() || null,
    phone: values.phone.trim() || null,
    country_code: country ? String(country.dialCode) : null,
    answers,
    calling_visit_unid: visitUnid,
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
export function formErrors(
  values: FormValues,
  asked: readonly QuestionKey[],
): FormErrors {
  const errors: FormErrors = {};
  if (!validName(values.firstName)) errors.firstName = "Enter your first name.";
  if (!validName(values.lastName)) errors.lastName = "Enter your last name.";
  if (!validEmail(values.email)) errors.email = "Enter a valid email address.";
  if (!validPhone(values)) errors.phone = "Enter a complete phone number.";
  for (const question of QUESTIONS) {
    if (!asked.includes(question.key) || question.kind === "text") continue;
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
