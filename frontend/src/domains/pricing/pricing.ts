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

/** What a year costs paid monthly, and what paying yearly saves. `undefined` for Free. */
export function yearlySaving(
  offer: PlanOffer,
): { fullYearCents: number; savedCents: number } | undefined {
  const month = priceFor(offer, INTERVAL.MONTH);
  const year = priceFor(offer, INTERVAL.YEAR);
  if (!month || !year) return undefined;
  const fullYearCents = month.amount_cents * 12;
  return { fullYearCents, savedCents: fullYearCents - year.amount_cents };
}

/**
 * The most a yearly price saves over twelve monthly ones, in whole percent,
 * rounded down so the page never promises more than it gives.
 */
export function bestYearlySaving(offers: readonly PlanOffer[]): number {
  const savings = offers.map((offer) => {
    const saving = yearlySaving(offer);
    if (!saving) return 0;
    // Whole cents times 100 first, so 20% stays 20 and not 19.999….
    return Math.floor((saving.savedCents * 100) / saving.fullYearCents);
  });
  return Math.max(0, ...savings);
}

/** What a visitor tells the pricing page about their week. */
export type WeekNeeds = {
  callsPerWeek: number;
  severalKinds: boolean;
  /** Following up with people who left, and seeing who shows up. */
  clientTools: boolean;
};

/** The slider's top stop, shown as "30+". */
export const MAX_CALLS_PER_WEEK = 30;

/** Yearly first: it is the cheaper way to pay. */
export const DEFAULT_INTERVAL: BillingInterval = INTERVAL.YEAR;

/** Lands on Pro: a little more than Free's monthly limit allows. */
const DEFAULT_NEEDS: WeekNeeds = {
  callsPerWeek: 8,
  severalKinds: false,
  clientTools: false,
};

/** The pricing page's answers, as they sit in its URL. Defaults stay out. */
type PricingSearch = {
  interval?: BillingInterval;
  calls?: number;
  kinds?: "several";
  tools?: "yes";
};

/** Keeps the answers a link can carry and drops the rest. */
export function pricingSearch(search: Record<string, unknown>): PricingSearch {
  const { interval, calls, kinds, tools } = search;
  const result: PricingSearch = {};
  if (interval === INTERVAL.MONTH || interval === INTERVAL.YEAR)
    result.interval = interval;
  const count = Number(calls);
  if (calls !== undefined && Number.isFinite(count)) {
    result.calls = Math.min(MAX_CALLS_PER_WEEK, Math.max(1, Math.round(count)));
  }
  if (kinds === "several") result.kinds = kinds;
  if (tools === "yes") result.tools = tools;
  return result;
}

export function needsFrom(search: PricingSearch): WeekNeeds {
  return {
    callsPerWeek: search.calls ?? DEFAULT_NEEDS.callsPerWeek,
    severalKinds: search.kinds === "several",
    clientTools: search.tools === "yes",
  };
}

/** The URL for these answers, leaving out whatever is the default. */
export function searchFor(interval: BillingInterval, needs: WeekNeeds): PricingSearch {
  return {
    ...(interval === DEFAULT_INTERVAL ? {} : { interval }),
    ...(needs.callsPerWeek === DEFAULT_NEEDS.callsPerWeek
      ? {}
      : { calls: needs.callsPerWeek }),
    ...(needs.severalKinds ? { kinds: "several" as const } : {}),
    ...(needs.clientTools ? { tools: "yes" as const } : {}),
  };
}

/** Calls a week as calls in a month, rounded up. */
export function callsPerMonth(callsPerWeek: number): number {
  return Math.ceil((callsPerWeek * 52) / 12);
}

function severalKindsAllowed(entitlements: Entitlements): boolean {
  const max = entitlements.max_event_types;
  return max === null || max > 1;
}

/** Whether the plan includes the week. Past its included bookings a page
 * still takes calls, but the plan is no longer the one that fits. */
function covers(entitlements: Entitlements, needs: WeekNeeds): boolean {
  const included = entitlements.included_bookings_per_month;
  const enoughCalls = included === null || callsPerMonth(needs.callsPerWeek) <= included;
  return (
    enoughCalls &&
    (!needs.severalKinds || severalKindsAllowed(entitlements)) &&
    (!needs.clientTools || (entitlements.win_back && entitlements.attendance))
  );
}

/**
 * The cheapest plan whose entitlements cover the week. The offers come
 * cheapest first; the last one stands in when nothing covers it.
 */
export function recommendPlan(offers: readonly PlanOffer[], needs: WeekNeeds): Plan {
  const fit = offers.find((offer) => covers(offer.entitlements, needs));
  return (fit ?? offers.at(-1))?.plan ?? PLAN.FREE;
}

/** One line on why the recommended plan fits, against the cheapest plan. */
export function fitReason(offers: readonly PlanOffer[], needs: WeekNeeds): string {
  const cheapest = offers[0];
  if (!cheapest) return "";
  const recommended = recommendPlan(offers, needs);
  const name = PLAN_NAMES[recommended];
  const included = cheapest.entitlements.included_bookings_per_month;
  const monthly = callsPerMonth(needs.callsPerWeek);
  if (covers(cheapest.entitlements, needs)) {
    return included === null
      ? `${name} covers your week.`
      : `${name} covers your week: about ${monthly} of the ${included} bookings it includes each month.`;
  }
  if (
    needs.clientTools &&
    recommendPlan(offers, { ...needs, clientTools: false }) !== recommended
  ) {
    return `${name} fits: follow up with people who left, see who will show up, and get your week in numbers.`;
  }
  if (needs.severalKinds && !severalKindsAllowed(cheapest.entitlements)) {
    return `${name} fits: several kinds of call on one link. ${PLAN_NAMES[cheapest.plan]} has one.`;
  }
  return `${name} fits: about ${monthly} bookings a month, past the ${included} ${PLAN_NAMES[cheapest.plan]} includes.`;
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
        label: "Bookings a month",
        value: (e) => limit(e.included_bookings_per_month, "included"),
      },
      { label: "Nobody is turned away past it", value: everyPlan },
      {
        label: "Event types",
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
      { label: "No Cadence logo on your page", value: (e) => e.hide_branding },
    ],
  },
  {
    title: "Business tools",
    rows: [
      { label: "Send your times to people who left", value: (e) => e.win_back },
      { label: "Invitees confirm they are coming", value: (e) => e.attendance },
      { label: "No-show tracking", value: (e) => e.attendance },
      { label: "Your week in numbers, every Monday", value: (e) => e.insights },
      { label: "Webhooks", value: (e) => e.webhooks },
    ],
  },
];

/** What a plan card lists: the lines that set the plan apart. */
export const PLAN_HIGHLIGHTS: Record<Plan, readonly string[]> = {
  free: [
    "One booking page",
    "25 bookings a month included",
    "Google Calendar and Meet",
    "Reminder a day before",
  ],
  pro: [
    "No Cadence logo on your page",
    "Several kinds of call on one link",
    "Unlimited bookings",
    "Reminders 24h, 2h and 30 min before",
    "Your own booking form questions",
    "Your own accent colour",
  ],
  business: [
    "Everything in Pro",
    "Win back people who left without booking",
    "Know who will show up",
    "Your week in numbers, every Monday",
    "Webhooks for your own tools",
  ],
};

export const PLAN_TAGLINES: Record<Plan, string> = {
  free: "For trying Cadence with a few calls a week.",
  pro: "For people whose work runs on calls.",
  business: "For turning bookings into clients.",
};

export const PLAN_CTA: Record<Plan, string> = {
  free: "Start on Free",
  pro: "Start 14-day Pro trial",
  business: "Start with Business",
};

/** What every plan promises, in the words the page, cards and FAQ share. */
export const PROMISES = {
  neverTurnedAway: {
    title: "Nobody is turned away.",
    detail: "Past the bookings Free includes, your page stays open.",
  },
  priceStays: {
    title: "Your price stays.",
    detail: "It never goes up while you stay subscribed.",
  },
  nothingDeleted: {
    title: "Nothing is deleted.",
    detail: "Change plans or cancel, and your settings wait for you.",
  },
} as const;

export type PricingPromise = keyof typeof PROMISES;
