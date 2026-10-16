import type { BillingInterval } from "../../bindings/BillingInterval";
import type { Entitlements } from "../../bindings/Entitlements";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import type { PlanPrice } from "../../bindings/PlanPrice";
import type { Reminder } from "../../bindings/Reminder";

export const PLAN = {
  FREE: "free",
  PRO: "pro",
  BUSINESS: "business",
} as const satisfies Record<Uppercase<Plan>, Plan>;

export const INTERVAL = {
  MONTH: "month",
  YEAR: "year",
} as const satisfies Record<Uppercase<BillingInterval>, BillingInterval>;

export const PLAN_NAMES: Record<Plan, string> = {
  free: "Free",
  pro: "Pro",
  business: "Business",
};

/** The plan the pricing page points people to. */
export const FEATURED_PLAN: Plan = PLAN.PRO;

/** The plan's price for `interval`. `undefined` for Free. */
export function priceFor(
  offer: PlanOffer,
  interval: BillingInterval,
): PlanPrice | undefined {
  return offer.prices.find((price) => price.interval === interval);
}

/** Whole dollars when the price is whole: "$8", "$12.50". */
export function formatCents(cents: number): string {
  const dollars = cents / 100;
  return Number.isInteger(dollars) ? `$${dollars}` : `$${dollars.toFixed(2)}`;
}

/**
 * The most a yearly price saves over twelve monthly ones, in whole percent,
 * rounded down so the page never promises more than it gives.
 */
export function bestYearlySaving(offers: readonly PlanOffer[]): number {
  const savings = offers.map((offer) => {
    const month = priceFor(offer, INTERVAL.MONTH);
    const year = priceFor(offer, INTERVAL.YEAR);
    if (!month || !year) return 0;
    const twelveMonths = month.amount_cents * 12;
    // Whole cents times 100 first, so 20% stays 20 and not 19.999….
    return Math.floor(((twelveMonths - year.amount_cents) * 100) / twelveMonths);
  });
  return Math.max(0, ...savings);
}

/** A cell of the comparison table: included, left out, or a value. */
export type FeatureValue = boolean | string;

type FeatureRow = {
  label: string;
  /** Promised on a plan but not built yet. */
  soon?: boolean;
  value: (entitlements: Entitlements) => FeatureValue;
};

type FeatureGroup = { title: string; rows: readonly FeatureRow[] };

const REMINDER_LABELS: Record<Reminder, string> = {
  before_24h: "24h",
  before_2h: "2h",
  before_30m: "30 min",
};

function limit(value: number | null, unit: string): string {
  return value === null ? "Unlimited" : `${value} ${unit}`;
}

const everyPlan = () => true;

export const FEATURE_GROUPS: readonly FeatureGroup[] = [
  {
    title: "Booking",
    rows: [
      { label: "Booking page with your own link", value: everyPlan },
      {
        label: "Bookings per month",
        value: (e) => limit(e.max_bookings_per_month, "a month"),
      },
      {
        label: "Event types",
        soon: true,
        value: (e) =>
          e.max_event_types === null ? "Unlimited" : String(e.max_event_types),
      },
      { label: "Weekly hours and dates off", value: everyPlan },
      { label: "Invitees reschedule and cancel themselves", value: everyPlan },
    ],
  },
  {
    title: "Calendar and meetings",
    rows: [
      { label: "Google Calendar busy times", value: everyPlan },
      { label: "Google Meet or your own meeting link", value: everyPlan },
    ],
  },
  {
    title: "Emails",
    rows: [
      { label: "Confirmation and change emails", value: everyPlan },
      {
        label: "Reminders before the call",
        value: (e) => e.reminders.map((reminder) => REMINDER_LABELS[reminder]).join(", "),
      },
    ],
  },
  {
    title: "Your brand",
    rows: [
      { label: "Your own booking form questions", value: (e) => e.custom_questions },
      { label: "Your accent colour", value: (e) => e.custom_accent },
      { label: "No Cadence branding", value: (e) => e.hide_branding },
    ],
  },
  {
    title: "Automation",
    rows: [{ label: "Webhooks", soon: true, value: (e) => e.webhooks }],
  },
];

/** What a plan card lists: the lines that set the plan apart. */
export const PLAN_HIGHLIGHTS: Record<Plan, readonly string[]> = {
  free: [
    "One booking page",
    "25 bookings a month",
    "Google Calendar and Meet",
    "Reminder a day before",
  ],
  pro: [
    "Unlimited bookings",
    "Reminders 24h, 2h and 30 min before",
    "Your own booking form questions",
    "Your colour, no Cadence branding",
  ],
  business: [
    "Everything in Pro",
    "Several event types (soon)",
    "Webhooks for your tools (soon)",
  ],
};

export const PLAN_TAGLINES: Record<Plan, string> = {
  free: "For trying Cadence with a few calls a week.",
  pro: "For people whose work runs on calls.",
  business: "For running a business on bookings.",
};

export const PLAN_CTA: Record<Plan, string> = {
  free: "Start free",
  pro: "Start 14-day free trial",
  business: "Start with Business",
};
