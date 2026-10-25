import { queryOptions } from "@tanstack/react-query";

import type { BillingSummary } from "../../bindings/BillingSummary";
import type { CompPlan } from "../../bindings/CompPlan";
import type { AdminCryptoPayment } from "../../bindings/AdminCryptoPayment";
import type { CryptoRefund } from "../../bindings/CryptoRefund";
import type { CryptoSummary } from "../../bindings/CryptoSummary";
import type { PlanOffer } from "../../bindings/PlanOffer";
import type { RedirectUrl } from "../../bindings/RedirectUrl";
import type { AdminCancellation } from "../../bindings/AdminCancellation";
import type { LeaveFeedback } from "../../bindings/LeaveFeedback";
import type { Referrals } from "../../bindings/Referrals";
import type { RefundCrypto } from "../../bindings/RefundCrypto";
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

/** Paying with crypto: whether it is on, and the prepaid period running. */
export const cryptoQuery = queryOptions({
  queryKey: QUERY_KEYS.cryptoBilling,
  queryFn: () => api<CryptoSummary>("/api/billing/crypto"),
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

/** Leaves the app for the crypto provider's checkout page. */
export async function goToCryptoCheckout(choice: StartCheckout): Promise<void> {
  const { url } = await api<RedirectUrl>("/api/billing/crypto/checkout", {
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

function post(path: string, body?: unknown): Promise<void> {
  return api<void>(path, {
    method: "POST",
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
}

/** Ends the plan with the paid period, saying why. */
export const cancelPlan = (feedback: LeaveFeedback) =>
  post("/api/billing/cancel", feedback);
/** The one free month's pause: the plan stays on, nothing is charged. */
export const pausePlan = (feedback: LeaveFeedback) =>
  post("/api/billing/pause", feedback);
/** Takes back a cancel. */
export const keepPlan = () => post("/api/billing/resume");
/** Keeps why a host who meant to leave went yearly instead. */
export const yearlyInstead = (feedback: LeaveFeedback) =>
  post("/api/billing/yearly-instead", feedback);

export const adminCancellationsQuery = queryOptions({
  queryKey: QUERY_KEYS.adminCancellations,
  queryFn: () => api<AdminCancellation[]>("/api/admin/billing/cancellations"),
});

/** An admin gives `userUnid` a paid plan for free, or takes it back with `null`. */
export function compPlan(userUnid: string, body: CompPlan): Promise<void> {
  return api<void>(`/api/admin/billing/${userUnid}`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
}

/** A host's crypto payments that took money, newest first, for an admin. */
export const adminCryptoPaymentsQuery = (userUnid: string) =>
  queryOptions({
    queryKey: QUERY_KEYS.adminCryptoPayments(userUnid),
    queryFn: () =>
      api<AdminCryptoPayment[]>(`/api/admin/billing/${userUnid}/crypto-payments`),
  });

/**
 * An admin gives back part of one crypto payment, or with no amount all that
 * is left of it, which also takes back the time it bought.
 */
export function refundCryptoPayment(
  paymentUnid: string,
  body: RefundCrypto,
): Promise<CryptoRefund> {
  return api<CryptoRefund>(`/api/admin/billing/crypto-payments/${paymentUnid}/refund`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}
