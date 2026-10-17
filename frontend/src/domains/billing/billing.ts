import type { AdminPlan } from "../../bindings/AdminPlan";
import type { BillingInterval } from "../../bindings/BillingInterval";
import type { BillingSummary } from "../../bindings/BillingSummary";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import type { SubscriptionStatus } from "../../bindings/SubscriptionStatus";
import { formatDate } from "../../lib/format";
import { INTERVAL, PLAN, PLAN_NAMES } from "../pricing/pricing";

export const STATUS = {
  TRIALING: "trialing",
  ACTIVE: "active",
  PAST_DUE: "past_due",
  CANCELED: "canceled",
  INCOMPLETE: "incomplete",
  COMPED: "comped",
} as const satisfies Record<Uppercase<SubscriptionStatus>, SubscriptionStatus>;

const DAY_MS = 24 * 60 * 60 * 1000;
/** A trial is pointed out this many days before it ends. */
export const TRIAL_WARNING_DAYS = 5;

/** A paid plan someone picked before signing up or checking out. */
export type CheckoutIntent = { plan: Plan; interval: BillingInterval };

const PAID_PLANS: readonly unknown[] = [PLAN.PRO, PLAN.BUSINESS];
const INTERVALS: readonly unknown[] = Object.values(INTERVAL);

/** Reads `?plan=&interval=` from an address. Free and anything unknown is no intent. */
export function checkoutIntent(search: Record<string, unknown>): Partial<CheckoutIntent> {
  const { plan, interval } = search;
  if (!PAID_PLANS.includes(plan) || !INTERVALS.includes(interval)) return {};
  return { plan: plan as Plan, interval: interval as BillingInterval };
}

/** Stripe bills this subscription now: a plan change goes through the portal. */
export function isBilled(summary: Pick<BillingSummary, "status">): boolean {
  return summary.status === STATUS.ACTIVE || summary.status === STATUS.PAST_DUE;
}

/** Whether the host has a Stripe customer, so the portal can open. */
export function hasPortal(summary: BillingSummary): boolean {
  return (
    summary.status !== null &&
    summary.status !== STATUS.TRIALING &&
    summary.status !== STATUS.COMPED
  );
}

/** Whole days left in a running trial, rounded up; `null` without one. */
export function trialDaysLeft(summary: BillingSummary, now: number): number | null {
  if (summary.status !== STATUS.TRIALING || !summary.trial_ends_at) return null;
  const left = Date.parse(summary.trial_ends_at) - now;
  return left > 0 ? Math.ceil(left / DAY_MS) : null;
}

/** What a plan card on the billing page offers this host. */
type PlanAction = "current" | "checkout" | "portal" | "none";

export function planAction(summary: BillingSummary, plan: Plan): PlanAction {
  const onIt = summary.plan === plan;
  if (isBilled(summary)) return onIt ? "current" : "portal";
  // A trial is Pro until it ends; keeping Pro still means paying for it.
  if (onIt && summary.status !== STATUS.TRIALING) return "current";
  if (plan === PLAN.FREE) return onIt ? "current" : "none";
  return "checkout";
}

/** How much of the month's included bookings are used, as a share from 0 to 1. */
export function bookingUsage(summary: BillingSummary): number | null {
  const limit = summary.entitlements.included_bookings_per_month;
  if (limit === null || limit === 0) return null;
  return Math.min(1, summary.bookings_this_month / limit);
}

/**
 * During a trial, this month against what Free includes: what dropping to
 * Free would mean. `null` when there is nothing to decide.
 */
export function freeAllowanceLine(
  summary: Pick<BillingSummary, "status" | "entitlements" | "bookings_this_month">,
  offers: readonly PlanOffer[],
): string | null {
  if (summary.status !== STATUS.TRIALING) return null;
  if (summary.entitlements.included_bookings_per_month !== null) return null;
  const limit = offers.find((offer) => offer.plan === PLAN.FREE)?.entitlements
    .included_bookings_per_month;
  if (limit == null) return null;
  return summary.bookings_this_month > limit
    ? `That is past the ${limit} a month Free includes.`
    : `Free includes ${limit} a month.`;
}

/** One sentence on where the host's plan stands. */
export function statusLine(summary: BillingSummary, now: number): string {
  const name = PLAN_NAMES[summary.plan];
  const periodEnd = summary.current_period_end
    ? formatDate(summary.current_period_end)
    : null;
  switch (summary.status) {
    case STATUS.TRIALING: {
      const days = trialDaysLeft(summary, now);
      return days === null
        ? "Your Pro trial has ended. Pick a plan to get Pro features back."
        : `${days} ${days === 1 ? "day" : "days"} left of your Pro trial. Pick a plan to keep Pro features.`;
    }
    case STATUS.ACTIVE:
      if (!periodEnd) return `You are on ${name}.`;
      return summary.cancel_at_period_end
        ? `${name} ends on ${periodEnd}. You move to Free then, and nothing is deleted.`
        : `${name} renews ${summary.interval === INTERVAL.YEAR ? "yearly" : "monthly"} on ${periodEnd}.`;
    case STATUS.PAST_DUE:
      return `The last payment failed. Update your card to keep ${name}.`;
    case STATUS.INCOMPLETE:
      return "The first payment did not go through. Try checking out again.";
    case STATUS.CANCELED:
      return "Your subscription has ended. You are on Free, and nothing was deleted.";
    case STATUS.COMPED:
      return `${name} is on us. There is nothing to pay.`;
    default:
      return "Free for as long as you like. Upgrade any time.";
  }
}

/** How many hosts each plan has now. */
export function planCounts(
  rows: readonly Pick<AdminPlan, "plan">[],
): Record<Plan, number> {
  const counts: Record<Plan, number> = { free: 0, pro: 0, business: 0 };
  for (const row of rows) counts[row.plan] += 1;
  return counts;
}
