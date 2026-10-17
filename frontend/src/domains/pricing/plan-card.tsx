import { cn } from "cn";
import { Check } from "lucide-react";
import type { ReactNode } from "react";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { Badge } from "../../components/ui/badge";
import {
  formatCents,
  INTERVAL,
  PLAN_HIGHLIGHTS,
  PLAN_NAMES,
  PLAN_TAGLINES,
  priceFor,
  yearlySaving,
} from "./pricing";

/** The plans side by side in one box, stacked where the box is narrow. */
export function PlanRow({ children }: { children: ReactNode }) {
  return (
    <div className="@container">
      <div className="grid divide-y overflow-hidden rounded-xl border bg-card @2xl:grid-cols-3 @2xl:divide-x @2xl:divide-y-0">
        {children}
      </div>
    </div>
  );
}

/** One plan in a `PlanRow`. */
export function PlanCard({
  offer,
  interval,
  featured = false,
  badge = "Most popular",
  fits = false,
  action,
}: {
  offer: PlanOffer;
  interval: BillingInterval;
  featured?: boolean;
  badge?: string;
  /** Matches what the visitor told the pricing page about their week. */
  fits?: boolean;
  /** The button: sign up on the pricing page, checkout on the billing page. */
  action: ReactNode;
}) {
  const price = priceFor(offer, interval);
  const monthly = priceFor(offer, INTERVAL.MONTH);
  const saving = yearlySaving(offer);
  const yearly = interval === INTERVAL.YEAR && price && monthly && saving;
  return (
    <section
      aria-label={PLAN_NAMES[offer.plan]}
      className={cn(
        "relative p-6 sm:p-7",
        (featured || fits) && "bg-dusk/5 dark:bg-dusk/10",
      )}
    >
      {featured || fits ? (
        <span aria-hidden className="absolute inset-x-0 top-0 h-[3px] bg-dusk" />
      ) : null}
      <div className="flex items-center justify-between gap-3">
        <h3 className="font-display font-wide text-xl font-bold">
          {PLAN_NAMES[offer.plan]}
        </h3>
        {fits ? (
          <Badge className="bg-dusk text-dusk-foreground">
            <Check aria-hidden />
            Fits your week
          </Badge>
        ) : featured ? (
          <Badge className="bg-dusk text-dusk-foreground">{badge}</Badge>
        ) : null}
      </div>
      <p className="mt-1 text-sm text-muted-foreground">{PLAN_TAGLINES[offer.plan]}</p>

      <div className="mt-6 flex items-baseline gap-1.5">
        <span className="text-clock text-5xl font-semibold tracking-tight">
          {formatCents(price?.monthly_cents ?? 0)}
        </span>
        {yearly ? (
          <s className="text-clock text-xl text-muted-foreground">
            <span className="sr-only">was </span>
            {formatCents(monthly.monthly_cents)}
          </s>
        ) : null}
        <span className="text-sm text-muted-foreground">/ month</span>
      </div>
      <p className="mt-1 h-5 text-sm text-muted-foreground">
        {!price ? (
          "Free for as long as you like"
        ) : yearly ? (
          <>
            {formatCents(price.amount_cents)} a year,{" "}
            <span className="font-medium text-success">
              save {formatCents(saving.savedCents)}
            </span>
          </>
        ) : (
          "Billed monthly"
        )}
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
    </section>
  );
}
