import { Link } from "@tanstack/react-router";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { ROUTES } from "../../lib/routes";
import { formatCents, INTERVAL, PLAN_NAMES, priceFor } from "./pricing";

/** The plan picked on the pricing page, shown while signing up for it. */
export function PickedPlan({
  plan,
  interval,
  offers,
}: {
  plan: Plan;
  interval: BillingInterval;
  offers: readonly PlanOffer[];
}) {
  const offer = offers.find((candidate) => candidate.plan === plan);
  const price = offer && priceFor(offer, interval);
  const period = interval === INTERVAL.YEAR ? "yearly" : "monthly";
  return (
    <div className="rounded-lg border border-dusk/50 bg-dusk/5 p-4 text-sm">
      <div className="flex items-baseline justify-between gap-3">
        <p className="font-semibold">
          {PLAN_NAMES[plan]}, {period}
          {price ? (
            <span className="text-clock font-normal text-muted-foreground">
              {" "}
              · {formatCents(price.monthly_cents)} / month
            </span>
          ) : null}
        </p>
        <Link to={ROUTES.pricing} className="font-medium underline underline-offset-4">
          Change
        </Link>
      </div>
      <p className="mt-1 text-muted-foreground">
        Your account starts with 14 days of Pro, no card needed. Subscribe to{" "}
        {PLAN_NAMES[plan]} on the next screen, or later from Billing.
      </p>
    </div>
  );
}
