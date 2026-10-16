import { cn } from "cn";
import { Check } from "lucide-react";
import type { ReactNode } from "react";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { Badge } from "../../components/ui/badge";
import { Card } from "../../components/ui/card";
import {
  formatCents,
  INTERVAL,
  PLAN_HIGHLIGHTS,
  PLAN_NAMES,
  PLAN_TAGLINES,
  priceFor,
} from "./pricing";

export function PlanCard({
  offer,
  interval,
  featured,
  badge = "Most popular",
  action,
}: {
  offer: PlanOffer;
  interval: BillingInterval;
  featured: boolean;
  badge?: string;
  /** The button: sign up on the pricing page, checkout on the billing page. */
  action: ReactNode;
}) {
  const price = priceFor(offer, interval);
  return (
    <Card
      className={cn(
        "relative gap-0 p-6",
        featured && "border-primary shadow-md ring-1 ring-primary",
      )}
    >
      <div className="flex items-center justify-between gap-3">
        <h2 className="font-display font-wide text-xl font-bold">
          {PLAN_NAMES[offer.plan]}
        </h2>
        {featured ? <Badge>{badge}</Badge> : null}
      </div>
      <p className="mt-1 text-sm text-muted-foreground">{PLAN_TAGLINES[offer.plan]}</p>

      <div className="mt-6 flex items-baseline gap-1.5">
        <span className="text-clock text-4xl font-semibold tracking-tight">
          {formatCents(price?.monthly_cents ?? 0)}
        </span>
        <span className="text-sm text-muted-foreground">/ month</span>
      </div>
      <p className="mt-1 h-5 text-sm text-muted-foreground">
        {!price
          ? "Free for as long as you like"
          : interval === INTERVAL.YEAR
            ? `${formatCents(price.amount_cents)} billed yearly`
            : "Billed monthly"}
      </p>

      <div className="mt-6 grid">{action}</div>

      <ul className="mt-6 grid gap-2.5 text-sm">
        {PLAN_HIGHLIGHTS[offer.plan].map((line) => (
          <li key={line} className="flex gap-2">
            <Check className="mt-0.5 size-4 shrink-0 text-success" aria-hidden />
            {line}
          </li>
        ))}
      </ul>
    </Card>
  );
}
