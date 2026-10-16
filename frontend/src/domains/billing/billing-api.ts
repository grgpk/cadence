import { queryOptions } from "@tanstack/react-query";

import type { BillingSummary } from "../../bindings/BillingSummary";
import type { PlanOffer } from "../../bindings/PlanOffer";
import type { RedirectUrl } from "../../bindings/RedirectUrl";
import type { StartCheckout } from "../../bindings/StartCheckout";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

export const billingQuery = queryOptions({
  queryKey: QUERY_KEYS.billing,
  queryFn: () => api<BillingSummary>("/api/billing/me"),
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
