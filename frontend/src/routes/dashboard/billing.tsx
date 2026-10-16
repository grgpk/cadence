import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback } from "react";

import { type CheckoutIntent, checkoutIntent } from "../../domains/billing/billing";
import { BillingPage } from "../../domains/billing/billing-page";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";

type BillingSearch = Partial<CheckoutIntent> & { checkout?: "done" };

export const Route = createFileRoute("/dashboard/billing")({
  // `?plan=&interval=` marks a plan picked on the pricing page;
  // `?checkout=done` is Stripe sending the host back.
  validateSearch: (search): BillingSearch => ({
    ...checkoutIntent(search),
    ...(search.checkout === "done" ? { checkout: "done" as const } : {}),
  }),
  head: () => ({ meta: [{ title: `Billing | ${APP_NAME}` }] }),
  component: BillingRoute,
});

function BillingRoute() {
  const { checkout, ...intent } = Route.useSearch();
  const navigate = useNavigate();
  const onConfirmed = useCallback(
    () => void navigate({ to: ROUTES.dashboardBilling, search: {}, replace: true }),
    [navigate],
  );
  return (
    <BillingPage
      intent={intent}
      checkoutDone={checkout === "done"}
      onConfirmed={onConfirmed}
    />
  );
}
