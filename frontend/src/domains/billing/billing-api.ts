import { queryOptions } from "@tanstack/react-query";

import type { BillingSummary } from "../../bindings/BillingSummary";
import type { CompPlan } from "../../bindings/CompPlan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import type { RedirectUrl } from "../../bindings/RedirectUrl";
import type { Referrals } from "../../bindings/Referrals";
import type { StartCheckout } from "../../bindings/StartCheckout";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

export const billingQuery = queryOptions({
  queryKey: QUERY_KEYS.billing,
  queryFn: () => api<BillingSummary>("/api/billing/me"),
});

export const referralsQuery = queryOptions({
  queryKey: QUERY_KEYS.referrals,
  queryFn: () => api<Referrals>("/api/billing/referrals"),
});

/** The plans change with a deploy, not while a page is open. */
export const billingPlansQuery = queryOptions({
  queryKey: QUERY_KEYS.billingPlans,
  queryFn: () => api<PlanOffer[]>("/api/billing/plans"),
  staleTime: Number.POSITIVE_INFINITY,
});

/** Leaves the app for Stripe's Checkout page. */
export async function goToCheckout(choice: StartCheckout): Promise<void> {
  const { url } = await api<RedirectUrl>("/api/billing/checkout", {
    method: "POST",
    body: JSON.stringify(choice),
  });
  window.location.assign(url);
}

/** Leaves the app for Stripe's Customer Portal. */
export async function goToPortal(): Promise<void> {
  const { url } = await api<RedirectUrl>("/api/billing/portal", { method: "POST" });
  window.location.assign(url);
}

/** Leaves the app for Stripe's screen confirming the move to yearly. */
export async function goToYearly(): Promise<void> {
  const { url } = await api<RedirectUrl>("/api/billing/yearly", { method: "POST" });
  window.location.assign(url);
}

/** An admin gives `userUnid` a paid plan for free, or takes it back with `null`. */
export function compPlan(userUnid: string, body: CompPlan): Promise<void> {
  return api<void>(`/api/admin/billing/${userUnid}`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
}
